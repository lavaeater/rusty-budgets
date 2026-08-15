use crate::models::budget_item::Periodicity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// How a tag's cost behaves over time — drives **budgeting**.
///
/// A `Recurring` cost is periodised: an annual 12 000 kr insurance contributes
/// 1 000 kr to every month's budget and accumulates a buffer, rather than
/// landing as a single unbudgeted spike. `Variable` costs are budgeted per
/// month as they happen and never buffered.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CostKind {
    /// A commitment billed on a known cadence (rent, insurance, broadband).
    Recurring(Periodicity),
    /// Spending with no billing cadence (groceries, shopping, eating out).
    Variable,
}

impl CostKind {
    /// Months between billings; `None` when the cost has no cadence.
    pub fn cycle_months(self) -> Option<i64> {
        match self {
            CostKind::Recurring(Periodicity::Monthly | Periodicity::OneOff) => Some(1),
            CostKind::Recurring(Periodicity::Quarterly) => Some(3),
            CostKind::Recurring(Periodicity::Annual) => Some(12),
            CostKind::Variable => None,
        }
    }

    /// The cadence, when there is one.
    pub fn periodicity(self) -> Option<Periodicity> {
        match self {
            CostKind::Recurring(p) => Some(p),
            CostKind::Variable => None,
        }
    }

    /// Whether the monthly budget contribution differs from the billed amount,
    /// i.e. whether this cost needs a buffer to accumulate between billings.
    pub fn needs_buffer(self) -> bool {
        self.cycle_months().is_some_and(|m| m > 1)
    }
}

/// What an auto-generated [`crate::models::MatchRule`] is allowed to do when it
/// matches an imported transaction — drives **import behaviour**.
///
/// Deliberately independent of [`CostKind`]: `Livsmedel` is `Variable` but its
/// payees (ICA, Coop) match perfectly, while `Bil - Underhåll` is a real
/// recurring cost billed by a different garage every time.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Matching {
    /// Apply the tag on import without asking.
    Automatic,
    /// Offer the tag as a suggestion for the user to confirm.
    Suggest,
}

impl CostKind {
    /// The matching mode a tag of this kind gets unless overridden — bills are
    /// safe to auto-apply, ad-hoc spending is not.
    pub fn default_matching(self) -> Matching {
        match self {
            CostKind::Recurring(_) => Matching::Automatic,
            CostKind::Variable => Matching::Suggest,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub cost_kind: CostKind,
    pub matching: Matching,
    pub deleted: bool,
    /// True when the classification was *derived* from a legacy
    /// `Periodicity::OneOff` rather than chosen, so the UI can prompt for a
    /// decision. `OneOff` was the serde default, so it means "never answered"
    /// as often as it means "one-off".
    pub needs_review: bool,
}

impl Tag {
    pub fn new(id: Uuid, name: String, cost_kind: CostKind, matching: Matching) -> Self {
        Self {
            id,
            name,
            cost_kind,
            matching,
            deleted: false,
            needs_review: false,
        }
    }

    /// Rebuilds a tag from a legacy `TagCreated` event, which only carried a
    /// `Periodicity`. A real cadence implies a bill that is safe to auto-apply;
    /// `OneOff` is ambiguous, so it becomes `Variable` **and** is flagged for
    /// review rather than silently losing auto-categorisation.
    pub fn from_legacy_periodicity(id: Uuid, name: String, periodicity: Periodicity) -> Self {
        let (cost_kind, needs_review) = match periodicity {
            Periodicity::OneOff => (CostKind::Variable, true),
            p => (CostKind::Recurring(p), false),
        };
        Self {
            id,
            name,
            cost_kind,
            matching: cost_kind.default_matching(),
            deleted: false,
            needs_review,
        }
    }

    pub fn periodicity(&self) -> Option<Periodicity> {
        self.cost_kind.periodicity()
    }

    pub fn is_automatic(&self) -> bool {
        self.matching == Matching::Automatic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tag_not_deleted() {
        let id = Uuid::new_v4();
        let tag = Tag::new(
            id,
            "Electricity".to_string(),
            CostKind::Recurring(Periodicity::Monthly),
            Matching::Automatic,
        );
        assert_eq!(tag.id, id);
        assert_eq!(tag.name, "Electricity");
        assert_eq!(tag.periodicity(), Some(Periodicity::Monthly));
        assert!(!tag.deleted);
        assert!(!tag.needs_review);
    }

    #[test]
    fn serde_round_trip() {
        let tag = Tag::new(
            Uuid::new_v4(),
            "Insurance".to_string(),
            CostKind::Recurring(Periodicity::Annual),
            Matching::Automatic,
        );
        let json = serde_json::to_string(&tag).unwrap();
        let back: Tag = serde_json::from_str(&json).unwrap();
        assert_eq!(tag, back);
    }

    #[test]
    fn legacy_cadence_becomes_an_automatic_bill() {
        for p in [
            Periodicity::Monthly,
            Periodicity::Quarterly,
            Periodicity::Annual,
        ] {
            let tag = Tag::from_legacy_periodicity(Uuid::new_v4(), "Bredband".into(), p);
            assert_eq!(tag.cost_kind, CostKind::Recurring(p));
            assert_eq!(tag.matching, Matching::Automatic);
            assert!(!tag.needs_review, "an explicit cadence is an answer");
        }
    }

    #[test]
    fn legacy_one_off_is_flagged_for_review() {
        // `OneOff` was `Periodicity::default()`, so it means "never answered"
        // as often as it means "one-off". It must not silently disable
        // auto-categorisation for a real bill.
        let tag = Tag::from_legacy_periodicity(Uuid::new_v4(), "Bredband".into(), Periodicity::OneOff);
        assert_eq!(tag.cost_kind, CostKind::Variable);
        assert_eq!(tag.matching, Matching::Suggest);
        assert!(tag.needs_review);
    }

    #[test]
    fn buffer_is_needed_only_between_billings() {
        assert!(CostKind::Recurring(Periodicity::Annual).needs_buffer());
        assert!(CostKind::Recurring(Periodicity::Quarterly).needs_buffer());
        assert!(!CostKind::Recurring(Periodicity::Monthly).needs_buffer());
        assert!(!CostKind::Variable.needs_buffer());
    }

    #[test]
    fn cycle_months_match_the_cadence() {
        assert_eq!(CostKind::Recurring(Periodicity::Monthly).cycle_months(), Some(1));
        assert_eq!(CostKind::Recurring(Periodicity::Quarterly).cycle_months(), Some(3));
        assert_eq!(CostKind::Recurring(Periodicity::Annual).cycle_months(), Some(12));
        assert_eq!(CostKind::Variable.cycle_months(), None);
    }

    #[test]
    fn default_matching_follows_cost_kind() {
        assert_eq!(
            CostKind::Recurring(Periodicity::Monthly).default_matching(),
            Matching::Automatic
        );
        assert_eq!(CostKind::Variable.default_matching(), Matching::Suggest);
    }
}
