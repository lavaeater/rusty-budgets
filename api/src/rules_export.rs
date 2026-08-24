//! Export/import of a budget's tags and auto-tagging rules as a portable
//! JSON document, independent of any one budget's `Uuid`s.
//!
//! Meant for the "I built up a good set of rules while testing, now I want
//! them on a clean budget" workflow: export from the source budget, import
//! into the target one. Tags and rules are matched by tag *name*, not id,
//! since ids from the source budget mean nothing in the target one.

use crate::cqrs::framework::DomainEvent;
use crate::models::{
    BankAccountType, Budget, BudgetEvent, CostKind, Matching, Periodicity, normalize_account_number,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Which sections of a budget's rules/accounts to include in an export.
/// Import needs no equivalent — [`apply_rules_export`] simply processes
/// whatever sections a given export happens to carry.
#[derive(Debug, Clone, Copy)]
pub struct ExportSelection {
    pub tags_and_rules: bool,
    pub transfer_rules: bool,
    pub bank_accounts: bool,
}

impl Default for ExportSelection {
    fn default() -> Self {
        Self {
            tags_and_rules: true,
            transfer_rules: true,
            bank_accounts: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RulesExport {
    pub tags: Vec<TagExport>,
    pub rules: Vec<RuleExport>,
    /// Absent from exports written before this field existed; imports of
    /// those files simply carry no transfer rules.
    #[serde(default)]
    pub transfer_rules: Vec<TransferRuleExport>,
    /// Absent from exports written before this field existed; imports of
    /// those files simply carry no bank accounts.
    #[serde(default)]
    pub bank_accounts: Vec<BankAccountExport>,
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

/// A learned [`crate::models::TransferRule`] pattern, portable across budgets
/// the same way as [`RuleExport`]: the tag is carried by name and re-resolved
/// against the target budget's tags on import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRuleExport {
    pub outgoing_account: String,
    pub incoming_account: String,
    pub outgoing_key: Vec<String>,
    pub incoming_key: Vec<String>,
    /// `None` replays as a plain internal transfer; `Some` as a savings
    /// contribution tagged with the named tag.
    pub tag_name: Option<String>,
}

/// An account's name and type, portable across budgets by (normalized)
/// account number — never by id, since a fresh import of the same bank
/// statement mints its own account ids.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccountExport {
    pub account_number: String,
    pub description: String,
    pub account_type: BankAccountType,
}

/// Deleted tags aren't exported: they exist only to keep historical
/// transactions intact, not as something to reapply on a clean budget. Any
/// rule pointing at one is exported with `tag_name: None`. `selection`
/// controls which sections are populated — an unselected section exports as
/// an empty `Vec`, which [`apply_rules_export`] treats as "nothing to do"
/// on import, so a partial export round-trips safely.
pub fn export_tags_and_rules(budget: &Budget, selection: ExportSelection) -> RulesExport {
    let live_tag_name_by_id: HashMap<_, _> = budget
        .get_tags()
        .iter()
        .filter(|t| !t.deleted)
        .map(|t| (t.id, t.name.clone()))
        .collect();

    let tags = if selection.tags_and_rules {
        budget
            .get_tags()
            .iter()
            .filter(|t| !t.deleted)
            .map(|t| TagExport {
                name: t.name.clone(),
                cost_kind: t.cost_kind,
                matching: t.matching,
            })
            .collect()
    } else {
        Vec::new()
    };

    let rules = if selection.tags_and_rules {
        budget
            .match_rules
            .iter()
            .map(|r| RuleExport {
                tag_name: r.tag_id.and_then(|id| live_tag_name_by_id.get(&id).cloned()),
                transaction_key: r.transaction_key.clone(),
                item_key: r.item_key.clone(),
                always_apply: r.always_apply,
            })
            .collect()
    } else {
        Vec::new()
    };

    let transfer_rules = if selection.transfer_rules {
        budget
            .transfer_rules
            .iter()
            .map(|r| TransferRuleExport {
                outgoing_account: r.outgoing_account.clone(),
                incoming_account: r.incoming_account.clone(),
                outgoing_key: r.outgoing_key.clone(),
                incoming_key: r.incoming_key.clone(),
                tag_name: r.tag_id.and_then(|id| live_tag_name_by_id.get(&id).cloned()),
            })
            .collect()
    } else {
        Vec::new()
    };

    let bank_accounts = if selection.bank_accounts {
        budget
            .accounts
            .iter()
            .map(|a| BankAccountExport {
                account_number: a.account_number.clone(),
                description: a.description.clone(),
                account_type: a.account_type,
            })
            .collect()
    } else {
        Vec::new()
    };

    RulesExport {
        tags,
        rules,
        transfer_rules,
        bank_accounts,
    }
}

/// Outcome of applying a [`RulesExport`] onto a budget.
#[derive(Debug, Clone, Copy, Default)]
pub struct ImportSummary {
    pub tags_created: usize,
    pub tags_reused: usize,
    pub rules_created: usize,
    pub rules_skipped: usize,
    pub transfer_rules_created: usize,
    pub transfer_rules_skipped: usize,
    pub bank_accounts_created: usize,
    pub bank_accounts_updated: usize,
    pub bank_accounts_reused: usize,
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

    for rule in &export.transfer_rules {
        let tag_id = match &rule.tag_name {
            Some(name) => {
                let Some(t) = current.get_tags().iter().find(|t| &t.name == name) else {
                    summary.transfer_rules_skipped += 1;
                    continue;
                };
                Some(t.id)
            }
            None => None,
        };

        match current.add_transfer_rule(
            rule.outgoing_account.clone(),
            rule.incoming_account.clone(),
            rule.outgoing_key.clone(),
            rule.incoming_key.clone(),
            tag_id,
        ) {
            Ok(added) => {
                added.apply(current);
                events.push(added.into());
                summary.transfer_rules_created += 1;
            }
            Err(_) => summary.transfer_rules_skipped += 1, // rule already exists
        }
    }

    for account in &export.bank_accounts {
        apply_bank_account_export(current, events, account, &mut summary);
    }

    summary
}

/// One [`BankAccountExport`] entry: updates the matching existing account
/// (by normalized number) if its type or description drifted, or creates it
/// if this budget has never seen that account before.
fn apply_bank_account_export(
    current: &mut Budget,
    events: &mut Vec<BudgetEvent>,
    account: &BankAccountExport,
    summary: &mut ImportSummary,
) {
    let normalized_number = normalize_account_number(&account.account_number);
    if let Some(existing) = current.accounts.iter().find(|a| a.account_number == normalized_number) {
        let existing_id = existing.id;
        let type_changed = existing.account_type != account.account_type;
        let description_changed = existing.description != account.description;
        if !type_changed && !description_changed {
            summary.bank_accounts_reused += 1;
            return;
        }
        if let Ok(modified) = current.modify_bank_account(
            existing_id,
            type_changed.then_some(account.account_type),
            description_changed.then(|| account.description.clone()),
        ) {
            modified.apply(current);
            events.push(modified.into());
            summary.bank_accounts_updated += 1;
        }
        return;
    }

    let Ok(created) = current.create_bank_account(normalized_number, account.description.clone()) else {
        return;
    };
    let account_id = created.apply(current);
    events.push(created.into());
    if account.account_type != BankAccountType::default()
        && let Ok(modified) = current.modify_bank_account(account_id, Some(account.account_type), None)
    {
        modified.apply(current);
        events.push(modified.into());
    }
    summary.bank_accounts_created += 1;
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
        rt.execute(user_id, budget_id, |budget| {
            budget
                .add_transfer_rule(
                    "1111".to_string(),
                    "2222".to_string(),
                    vec!["savings".to_string()],
                    vec!["deposit".to_string()],
                    Some(tag_id),
                )
                .map(Into::into)
        })
        .unwrap();

        let budget = rt.load(budget_id).unwrap();
        let export = export_tags_and_rules(&budget, ExportSelection::default());
        assert_eq!(export.tags.len(), 1);
        assert_eq!(export.tags[0].name, "Electricity");
        assert_eq!(export.rules.len(), 1);
        assert_eq!(export.rules[0].tag_name.as_deref(), Some("Electricity"));
        assert_eq!(export.transfer_rules.len(), 1);
        assert_eq!(
            export.transfer_rules[0].tag_name.as_deref(),
            Some("Electricity")
        );

        // Apply onto a fresh, unrelated budget.
        let target_budget_id = new_budget(&rt, user_id);
        let mut current = rt.load(target_budget_id).unwrap();
        let mut events = Vec::new();
        let summary = apply_rules_export(&mut current, &mut events, &export);
        assert_eq!(summary.tags_created, 1);
        assert_eq!(summary.rules_created, 1);
        assert_eq!(summary.transfer_rules_created, 1);
        assert_eq!(events.len(), 4); // TagCreated + TagClassified + RuleAdded + TransferRuleAdded

        let new_tag = current.get_tags().iter().find(|t| t.name == "Electricity").unwrap();
        assert_eq!(new_tag.cost_kind, CostKind::Recurring(Periodicity::Monthly));
        assert_eq!(new_tag.matching, Matching::Automatic);
        assert_ne!(new_tag.id, tag_id, "target tag must get its own id");
        assert_eq!(current.match_rules.len(), 1);
        let new_rule = current.match_rules.iter().next().unwrap();
        assert_eq!(new_rule.tag_id, Some(new_tag.id));
        assert_eq!(current.transfer_rules.len(), 1);
        let new_transfer_rule = current.transfer_rules.iter().next().unwrap();
        assert_eq!(new_transfer_rule.tag_id, Some(new_tag.id));

        // Re-applying the same export is a no-op, not a duplicate.
        let mut events2 = Vec::new();
        let summary2 = apply_rules_export(&mut current, &mut events2, &export);
        assert_eq!(summary2.tags_created, 0);
        assert_eq!(summary2.tags_reused, 1);
        assert_eq!(summary2.rules_created, 0);
        assert_eq!(summary2.rules_skipped, 1);
        assert_eq!(summary2.transfer_rules_created, 0);
        assert_eq!(summary2.transfer_rules_skipped, 1);
        assert!(events2.is_empty());
    }

    #[test]
    fn export_selection_filters_sections() {
        let rt = JoyDbBudgetRuntime::new_in_memory();
        let user_id = Uuid::new_v4();
        let budget_id = new_budget(&rt, user_id);
        rt.create_tag(user_id, budget_id, "Electricity".to_string(), Periodicity::Monthly)
            .unwrap();
        rt.ensure_account(user_id, budget_id, "1234567890", "Skandiabanken")
            .unwrap();

        let budget = rt.load(budget_id).unwrap();
        let export = export_tags_and_rules(
            &budget,
            ExportSelection {
                tags_and_rules: false,
                transfer_rules: false,
                bank_accounts: true,
            },
        );
        assert!(export.tags.is_empty());
        assert!(export.rules.is_empty());
        assert!(export.transfer_rules.is_empty());
        assert_eq!(export.bank_accounts.len(), 1);
        assert_eq!(export.bank_accounts[0].account_number, "1234567890");
    }

    #[test]
    fn bank_accounts_round_trip_create_then_update() {
        let rt = JoyDbBudgetRuntime::new_in_memory();
        let user_id = Uuid::new_v4();
        let budget_id = new_budget(&rt, user_id);
        let account_id = rt
            .ensure_account(user_id, budget_id, "91594824853", "Skandiabanken")
            .unwrap();
        rt.modify_bank_account(
            user_id,
            budget_id,
            account_id,
            Some(crate::models::BankAccountType::Savings),
            Some("Barnens sparkonto".to_string()),
        )
        .unwrap();

        let budget = rt.load(budget_id).unwrap();
        let export = export_tags_and_rules(&budget, ExportSelection::default());
        assert_eq!(export.bank_accounts.len(), 1);
        assert_eq!(export.bank_accounts[0].account_number, "91594824853");
        assert_eq!(export.bank_accounts[0].description, "Barnens sparkonto");
        assert_eq!(
            export.bank_accounts[0].account_type,
            crate::models::BankAccountType::Savings
        );

        // Importing onto a fresh budget creates the account with its type.
        let target_budget_id = new_budget(&rt, user_id);
        let mut current = rt.load(target_budget_id).unwrap();
        let mut events = Vec::new();
        let summary = apply_rules_export(&mut current, &mut events, &export);
        assert_eq!(summary.bank_accounts_created, 1);
        let created = current.accounts.iter().find(|a| a.account_number == "91594824853").unwrap();
        assert_eq!(created.description, "Barnens sparkonto");
        assert_eq!(created.account_type, crate::models::BankAccountType::Savings);

        // Re-importing after a further rename updates rather than duplicates.
        let mut renamed_export = export.clone();
        renamed_export.bank_accounts[0].description = "Sparkonto Alice".to_string();
        let mut events2 = Vec::new();
        let summary2 = apply_rules_export(&mut current, &mut events2, &renamed_export);
        assert_eq!(summary2.bank_accounts_updated, 1);
        assert_eq!(summary2.bank_accounts_created, 0);
        assert_eq!(current.accounts.len(), 1, "must update in place, not duplicate");
        assert_eq!(current.accounts[0].description, "Sparkonto Alice");
    }
}
