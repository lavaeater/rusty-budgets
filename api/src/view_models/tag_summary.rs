use crate::models::{CostKind, Matching, Money, Periodicity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Per-tag spending summary, computed from all historical transaction data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TagSummary {
    pub tag_id: Uuid,
    pub name: String,
    pub cost_kind: CostKind,
    pub matching: Matching,
    /// Classification was derived from a legacy default rather than chosen.
    pub needs_review: bool,
    /// Observed average per month across the whole history window (signed —
    /// negative for expenses, positive for income). Noisy for infrequent bills.
    pub average_monthly: Money,
    /// Total for the most recent calendar month with transaction data.
    pub last_month: Money,
    /// What to budget each month. For a cost billed less often than monthly
    /// this is the billed amount spread across its cycle — an annual 12 000 kr
    /// insurance contributes 1 000 kr/month — so a once-a-year bill does not
    /// land as an unbudgeted spike. Equals `average_monthly` otherwise.
    pub monthly_budget_contribution: Money,
    /// Number of tagged transactions included in the average.
    pub transaction_count: u32,
}

impl TagSummary {
    pub fn periodicity(&self) -> Option<Periodicity> {
        self.cost_kind.periodicity()
    }

    /// The amount that must be available when the bill lands — i.e. the buffer
    /// a `Recurring` cost needs to accumulate between billings. `None` for
    /// costs billed monthly or with no cadence, which need no buffer.
    pub fn buffer_target(&self) -> Option<Money> {
        let months = self.cost_kind.cycle_months()?;
        (months > 1).then(|| self.monthly_budget_contribution.multiply(months))
    }
}
