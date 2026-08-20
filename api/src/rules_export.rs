//! Export/import of a budget's tags and auto-tagging rules as a portable
//! JSON document, independent of any one budget's `Uuid`s.
//!
//! Meant for the "I built up a good set of rules while testing, now I want
//! them on a clean budget" workflow: export from the source budget, import
//! into the target one. Tags and rules are matched by tag *name*, not id,
//! since ids from the source budget mean nothing in the target one.

use crate::cqrs::framework::DomainEvent;
use crate::models::{Budget, BudgetEvent, CostKind, Matching, Periodicity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RulesExport {
    pub tags: Vec<TagExport>,
    pub rules: Vec<RuleExport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagExport {
    pub name: String,
    pub cost_kind: CostKind,
    pub matching: Matching,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleExport {
    /// `None` for a rule with no tag attached — never fires but round-trips.
    pub tag_name: Option<String>,
    pub transaction_key: Vec<String>,
    pub item_key: Vec<String>,
    pub always_apply: bool,
}

/// Deleted tags aren't exported: they exist only to keep historical
/// transactions intact, not as something to reapply on a clean budget. Any
/// rule pointing at one is exported with `tag_name: None`.
pub fn export_tags_and_rules(budget: &Budget) -> RulesExport {
    let tags = budget
        .get_tags()
        .iter()
        .filter(|t| !t.deleted)
        .map(|t| TagExport {
            name: t.name.clone(),
            cost_kind: t.cost_kind,
            matching: t.matching,
        })
        .collect();

    let live_tag_name_by_id: HashMap<_, _> = budget
        .get_tags()
        .iter()
        .filter(|t| !t.deleted)
        .map(|t| (t.id, t.name.clone()))
        .collect();

    let rules = budget
        .match_rules
        .iter()
        .map(|r| RuleExport {
            tag_name: r.tag_id.and_then(|id| live_tag_name_by_id.get(&id).cloned()),
            transaction_key: r.transaction_key.clone(),
            item_key: r.item_key.clone(),
            always_apply: r.always_apply,
        })
        .collect();

    RulesExport { tags, rules }
}

/// Outcome of applying a [`RulesExport`] onto a budget.
#[derive(Debug, Clone, Copy, Default)]
pub struct ImportSummary {
    pub tags_created: usize,
    pub tags_reused: usize,
    pub rules_created: usize,
    pub rules_skipped: usize,
}

/// Applies `export` to `current` **in memory**: creates any tag that doesn't
/// already exist by name (reusing the existing one otherwise, whatever its
/// current classification), then creates every rule that doesn't already
/// exist, resolving `tag_name` against the now-current set of tags. Pushes
/// every resulting event onto `events`.
///
/// Callers are expected to `append_many` + `snapshot` once after this, same
/// as every other bulk in-memory call site — see `db::import_tags_and_rules`.
pub fn apply_rules_export(
    current: &mut Budget,
    events: &mut Vec<BudgetEvent>,
    export: &RulesExport,
) -> ImportSummary {
    let mut summary = ImportSummary::default();

    for tag in &export.tags {
        if current.get_tags().iter().any(|t| t.name == tag.name) {
            summary.tags_reused += 1;
            continue;
        }
        let periodicity = tag.cost_kind.periodicity().unwrap_or(Periodicity::OneOff);
        let Ok(created) = current.create_tag(tag.name.clone(), periodicity) else {
            summary.tags_reused += 1;
            continue;
        };
        let tag_id = created.apply(current);
        events.push(created.into());

        if let Ok(classified) = current.classify_tag(tag_id, tag.cost_kind, tag.matching) {
            classified.apply(current);
            events.push(classified.into());
        }
        summary.tags_created += 1;
    }

    for rule in &export.rules {
        let tag_id = match &rule.tag_name {
            Some(name) => {
                let Some(t) = current.get_tags().iter().find(|t| &t.name == name) else {
                    summary.rules_skipped += 1;
                    continue;
                };
                Some(t.id)
            }
            None => None,
        };

        match current.add_rule(
            rule.transaction_key.clone(),
            rule.item_key.clone(),
            rule.always_apply,
            tag_id,
        ) {
            Ok(added) => {
                added.apply(current);
                events.push(added.into());
                summary.rules_created += 1;
            }
            Err(_) => summary.rules_skipped += 1, // rule already exists
        }
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cqrs::runtime::{BudgetCommandsTrait, JoyDbBudgetRuntime};
    use crate::cqrs::framework::Runtime;
    use crate::models::{Currency, MonthBeginsOn};
    use uuid::Uuid;

    fn new_budget(rt: &JoyDbBudgetRuntime, user_id: Uuid) -> Uuid {
        rt.create_budget(user_id, "Test Budget", true, MonthBeginsOn::default(), Currency::SEK)
            .unwrap()
    }

    #[test]
    fn round_trips_tags_and_rules() {
        let rt = JoyDbBudgetRuntime::new_in_memory();
        let user_id = Uuid::new_v4();
        let budget_id = new_budget(&rt, user_id);

        let tag_id = rt
            .create_tag(user_id, budget_id, "Electricity".to_string(), Periodicity::Monthly)
            .unwrap();
        rt.classify_tag(
            user_id,
            budget_id,
            tag_id,
            CostKind::Recurring(Periodicity::Monthly),
            Matching::Automatic,
        )
        .unwrap();
        rt.add_rule(
            user_id,
            budget_id,
            vec!["vattenfall".to_string()],
            Vec::new(),
            true,
            Some(tag_id),
        )
        .unwrap();

        let budget = rt.load(budget_id).unwrap();
        let export = export_tags_and_rules(&budget);
        assert_eq!(export.tags.len(), 1);
        assert_eq!(export.tags[0].name, "Electricity");
        assert_eq!(export.rules.len(), 1);
        assert_eq!(export.rules[0].tag_name.as_deref(), Some("Electricity"));

        // Apply onto a fresh, unrelated budget.
        let target_budget_id = new_budget(&rt, user_id);
        let mut current = rt.load(target_budget_id).unwrap();
        let mut events = Vec::new();
        let summary = apply_rules_export(&mut current, &mut events, &export);
        assert_eq!(summary.tags_created, 1);
        assert_eq!(summary.rules_created, 1);
        assert_eq!(events.len(), 3); // TagCreated + TagClassified + RuleAdded

        let new_tag = current.get_tags().iter().find(|t| t.name == "Electricity").unwrap();
        assert_eq!(new_tag.cost_kind, CostKind::Recurring(Periodicity::Monthly));
        assert_eq!(new_tag.matching, Matching::Automatic);
        assert_ne!(new_tag.id, tag_id, "target tag must get its own id");
        assert_eq!(current.match_rules.len(), 1);
        let new_rule = current.match_rules.iter().next().unwrap();
        assert_eq!(new_rule.tag_id, Some(new_tag.id));

        // Re-applying the same export is a no-op, not a duplicate.
        let mut events2 = Vec::new();
        let summary2 = apply_rules_export(&mut current, &mut events2, &export);
        assert_eq!(summary2.tags_created, 0);
        assert_eq!(summary2.tags_reused, 1);
        assert_eq!(summary2.rules_created, 0);
        assert_eq!(summary2.rules_skipped, 1);
        assert!(events2.is_empty());
    }
}
