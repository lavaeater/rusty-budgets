use crate::cqrs::framework::Aggregate;
use crate::cqrs::framework::DomainEvent;
use crate::events::{BudgetCreated, ItemAdded, ActualAdded, TransactionAdded, TransactionConnected, TransactionIgnored, BudgetedFundsReallocated, ActualBudgetedFundsAdjusted, ItemModified, ActualModified, RuleAdded, AllocationCreated, AllocationDeleted, BankAccountCreated, TagCreated, TagModified, TagClassified, CarryoverConfigured, TransactionTagged, TransactionUntagged, RuleModified, RuleDeleted, ItemBufferSet, TransferPairRejected, TransferRuleAdded};
use crate::models::budget_item::{BudgetItem, Periodicity};
use crate::models::budget_period::RuleMatch;
use crate::models::budget_period_id::PeriodId;
use crate::models::budgeting_type::BudgetingType;
use crate::models::money::{Currency, Money};
use crate::models::rule_packages::RulePackages;
use crate::models::{
    ActualItem, BankAccount, BankTransaction, BudgetPeriod, MatchRule, Matching, MonthBeginsOn, Tag,
    TransactionAllocation, TransferRule,
};
use crate::pub_events_enum;
use crate::view_models::{BudgetingTypeOverview, TagSummary};
use chrono::{DateTime, Utc};
use joydb::Model as JoyModel;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use serde_json::Value;
use uuid::Uuid;

pub_events_enum! {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum BudgetEvent {
        BudgetCreated,
        ItemAdded,
        ActualAdded,
        TransactionAdded,
        TransactionConnected,
        TransactionIgnored,
        BudgetedFundsReallocated,
        ActualBudgetedFundsAdjusted,
        ItemModified,
        ActualModified,
        RuleAdded,
        AllocationCreated,
        AllocationDeleted,
        BankAccountCreated,
        TagCreated,
        TagModified,
        TagClassified,
        CarryoverConfigured,
        TransactionTagged,
        TransactionUntagged,
        RuleModified,
        RuleDeleted,
        ItemBufferSet,
        TransferPairRejected,
        TransferRuleAdded,
    }
}

// --- Budget Domain ---
#[derive(Debug, Clone, Serialize, Deserialize, JoyModel)]
#[derive(Default)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub items: Vec<BudgetItem>,
    pub month_begins_on: MonthBeginsOn,
    pub periods: Vec<BudgetPeriod>,
    #[serde(default)]
    pub rules: RulePackages,
    pub transaction_hashes: HashSet<u64>,
    pub match_rules: HashSet<MatchRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub default_budget: bool,
    pub last_event: i64,
    pub version: i64,
    pub currency: Currency,
    #[serde(default)]
    pub accounts: Vec<BankAccount>,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub rejected_transfer_pairs: HashSet<(Uuid, Uuid)>,
    /// Learned patterns for recurring internal-transfer pairs, created
    /// whenever the user resolves a [`Self::potential_internal_transfers`]
    /// pair by hand. See [`Self::suggested_transfer_resolutions`].
    #[serde(default)]
    pub transfer_rules: HashSet<TransferRule>,
    /// First period whose category balances carry forward. `None` disables
    /// carryover — the pre-existing behaviour, and the default for budgets
    /// whose snapshots predate this field.
    #[serde(default)]
    pub carryover_from: Option<PeriodId>,
}


impl Budget {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            periods: vec![BudgetPeriod::new(PeriodId::from_date(
                Utc::now(),
                MonthBeginsOn::default(),
            ))],
            ..Default::default()
        }
    }

    pub fn get_account(&self, account_number: &str) -> Option<&BankAccount> {
        self.accounts
            .iter()
            .find(|a| a.account_number == account_number)
    }

    pub fn has_account(&self, account_number: &str) -> bool {
        self.accounts
            .iter()
            .any(|a| a.account_number == account_number)
    }

    pub fn add_account(&mut self, account: BankAccount) {
        if !self.has_account(&account.account_number) {
            self.accounts.push(account);
        }
    }

    pub fn get_item(&self, item_id: Uuid) -> Option<&BudgetItem> {
        self.items.iter().find(|item| item.id == item_id)
    }

    pub fn get_item_mut(&mut self, item_id: Uuid) -> Option<&mut BudgetItem> {
        self.items.iter_mut().find(|item| item.id == item_id)
    }

    pub fn get_period(&self, period_id: PeriodId) -> Option<&BudgetPeriod> {
        self.periods.iter().find(|period| period.id == period_id)
    }

    pub fn all_actuals(&self, period_id: PeriodId) -> Vec<&ActualItem> {
        self.get_period(period_id)
            .map(|p| p.all_actuals())
            .unwrap_or_default()
    }

    pub fn ignored_transactions(&self, period_id: PeriodId) -> Vec<&BankTransaction> {
        self.get_period(period_id)
            .map(|p| p.ignored_transactions())
            .unwrap_or_default()
    }

    pub fn month_begins_on(&self) -> MonthBeginsOn {
        self.month_begins_on
    }

    pub fn items_by_type(
        &self,
        budget_period_id: PeriodId,
    ) -> Vec<(usize, BudgetingType, BudgetingTypeOverview, Vec<ActualItem>)> {
        self.get_period(budget_period_id)
            .map(|p| p.items_by_type(&self.rules))
            .unwrap_or_default()
    }

    pub fn all_items(&self) -> &Vec<BudgetItem> {
        &self.items
    }

    pub fn budgeted_for_type(&self, budgeting_type: BudgetingType, period_id: PeriodId) -> Money {
        self.get_period(period_id)
            .map(|p| p.budgeted_for_type(budgeting_type))
            .unwrap_or_default()
    }

    pub fn spent_for_type(&self, budgeting_type: BudgetingType, period_id: PeriodId) -> Money {
        self.get_period(period_id)
            .map(|p| p.spent_for_type(budgeting_type))
            .unwrap_or_default()
    }

    pub fn insert_item(&mut self, item: BudgetItem) {
        self.items.push(item);
    }

    pub fn remove_item(&mut self, item_id: Uuid) {
        self.items.retain(|item| item.id != item_id);
    }

    pub fn insert_transaction(&mut self, tx: BankTransaction) -> bool {
        if self.transaction_hashes.insert(tx.get_hash()) {
            let period_id = PeriodId::from_date(tx.date, self.month_begins_on);
            self.with_period_mut(period_id).insert_transaction(tx);
            true
        } else {
            false
        }
    }

    pub fn can_insert_transaction(&self, tx_hash: &u64) -> bool {
        !self.transaction_hashes.contains(tx_hash)
    }

    pub fn contains_transaction(&self, tx_id: Uuid) -> bool {
        self.periods
            .iter()
            .any(|p| p.transactions.iter().any(|t| t.id == tx_id))
    }

    pub fn get_period_for_transaction(&self, tx_id: Uuid) -> Option<&BudgetPeriod> {
        self.periods
            .iter()
            .find(|p| p.transactions.iter().any(|t| t.id == tx_id))
    }

    pub fn get_period_for_transaction_mut(&mut self, tx_id: Uuid) -> Option<&mut BudgetPeriod> {
        self.periods
            .iter_mut()
            .find(|p| p.transactions.iter().any(|t| t.id == tx_id))
    }

    pub fn contains_budget_item(&self, item_id: Uuid) -> bool {
        self.items.iter().any(|i| i.id == item_id)
    }

    pub fn contains_item_with_name(&self, name: &str) -> bool {
        self.items.iter().any(|i| i.name == name)
    }

    pub fn get_transaction_mut(&mut self, tx_id: Uuid) -> Option<&mut BankTransaction> {
        self.get_period_for_transaction_mut(tx_id)
            .and_then(|p| p.transactions.iter_mut().find(|t| t.id == tx_id))
    }

    pub fn get_transaction(&self, tx_id: Uuid) -> Option<&BankTransaction> {
        self.get_period_for_transaction(tx_id)
            .and_then(|p| p.transactions.iter().find(|t| t.id == tx_id))
    }

    pub fn update_actuals_for_item(&mut self, item_id: Uuid) {
        if let Some(item) = self.get_item(item_id).cloned() {
            self.periods
                .iter_mut()
                .for_each(|p| p.update_actuals_from_item(&item));
        }
    }

    pub fn modify_budget_item(
        &mut self,
        id: Uuid,
        name: Option<String>,
        budgeting_type: Option<BudgetingType>,
        tag_ids: Option<Vec<Uuid>>,
        periodicity: Option<Periodicity>,
    ) {
        let mut was_updated = false;
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            if let Some(name) = name {
                item.name = name;
            }
            if let Some(item_type) = budgeting_type {
                item.budgeting_type = item_type;
            }
            if let Some(tag_ids) = tag_ids {
                item.tag_ids = tag_ids;
            }
            if let Some(periodicity) = periodicity {
                item.periodicity = periodicity;
            }
            was_updated = true;
        }

        if was_updated {
            self.update_actuals_for_item(id);
        }
    }

    pub fn contains_tag_with_name(&self, name: &str) -> bool {
        self.tags.iter().any(|t| t.name == name)
    }

    pub fn contains_tag(&self, tag_id: Uuid) -> bool {
        self.tags.iter().any(|t| t.id == tag_id)
    }

    pub fn get_tags(&self) -> &Vec<Tag> {
        &self.tags
    }

    pub fn get_active_tags(&self) -> Vec<&Tag> {
        self.tags.iter().filter(|t| !t.deleted).collect()
    }

    /// Budgeted and actual for one item in one period, using the *same*
    /// definitions the projection uses — actual comes from tagged transactions
    /// and is abs-normalised for Expense/Savings so it compares against a
    /// positive budgeted amount.
    ///
    /// Factored out so carryover and `BudgetItemViewModel` cannot drift apart:
    /// if they disagreed, a category's running balance would not match the
    /// numbers shown next to it.
    fn item_period_totals(&self, item: &BudgetItem, period: &BudgetPeriod) -> (Money, Money) {
        let actual_item = period
            .actual_items
            .iter()
            .find(|ai| ai.budget_item_id == item.id);
        let budgeted = actual_item.map_or_else(|| Money::zero(self.currency), |ai| ai.budgeted_amount);
        let effective_type = actual_item.map_or(item.budgeting_type, |ai| ai.budgeting_type);

        let tagged: Money = period
            .transactions
            .iter()
            .filter(|tx| !tx.ignored)
            .filter(|tx| tx.tag_id.is_some_and(|tid| item.tag_ids.contains(&tid)))
            .map(|tx| tx.amount)
            .sum();
        let actual = match effective_type {
            BudgetingType::Expense | BudgetingType::Savings => tagged.abs(),
            _ => tagged,
        };
        // Fall back to the stored actual when nothing is tagged, mirroring the
        // projection's precedence.
        let actual = if actual.is_zero() {
            actual_item.map_or_else(|| Money::zero(self.currency), |ai| ai.actual_amount)
        } else {
            actual
        };
        (budgeted, actual)
    }

    /// Running category balances entering `period_id`, keyed by budget item.
    ///
    /// This is the envelope identity applied across periods:
    /// `available(n) = available(n-1) + budgeted(n) - actual(n)`, accumulated
    /// from `carryover_from` up to but excluding `period_id`. Overspending
    /// carries forward as a **negative** balance, so a category stays visibly
    /// in the hole until it is topped up.
    ///
    /// Empty when carryover is disabled (`carryover_from == None`), which keeps
    /// the pre-existing per-period behaviour for budgets that have not opted in.
    ///
    /// Derived rather than stored: editing a past month must flow forward
    /// automatically, and storing it would need an event per item per period.
    pub fn carryover_into(&self, period_id: PeriodId) -> std::collections::HashMap<Uuid, Money> {
        let mut balances = std::collections::HashMap::new();
        let Some(from) = self.carryover_from else {
            return balances;
        };

        let mut periods: Vec<&BudgetPeriod> = self
            .periods
            .iter()
            .filter(|p| p.id >= from && p.id < period_id)
            .collect();
        periods.sort_by_key(|p| p.id);

        for period in periods {
            for item in &self.items {
                let (budgeted, actual) = self.item_period_totals(item, period);
                let entry = balances
                    .entry(item.id)
                    .or_insert_with(|| Money::zero(self.currency));
                *entry += budgeted - actual;
            }
        }
        balances
    }

    /// Compute average monthly and yearly spend per active tag, across all transaction history.
    ///
    /// The denominator is the number of calendar months spanned by the full transaction dataset
    /// (from earliest to latest transaction, inclusive). Amounts are signed — expenses are
    /// negative, income positive.
    ///
    /// # Panics
    /// Never panics: the `min`/`max` unwraps below are guarded by the preceding `is_empty` check.
    pub fn get_tag_summaries(&self) -> Vec<TagSummary> {
        use chrono::Datelike;
        use std::collections::HashMap;

        let all_txs: Vec<&BankTransaction> = self
            .periods
            .iter()
            .flat_map(|p| p.transactions.iter())
            .filter(|tx| !tx.ignored)
            .collect();

        if all_txs.is_empty() {
            return vec![];
        }

        let min_date = all_txs.iter().map(|tx| tx.date).min().unwrap();
        let max_date = all_txs.iter().map(|tx| tx.date).max().unwrap();
        let months_covered = {
            let months = (i64::from(max_date.year()) - i64::from(min_date.year())) * 12
                + (i64::from(max_date.month()) - i64::from(min_date.month()))
                + 1;
            months.max(1)
        };

        let mut per_tag: HashMap<Uuid, (Money, u32)> = HashMap::new();
        for tx in &all_txs {
            if let Some(tag_id) = tx.tag_id {
                let entry = per_tag
                    .entry(tag_id)
                    .or_insert((Money::zero(self.currency), 0));
                entry.0 += tx.amount;
                entry.1 += 1;
            }
        }

        self.tags
            .iter()
            .filter(|t| !t.deleted)
            .map(|tag| {
                let (total, count) = per_tag
                    .get(&tag.id)
                    .copied()
                    .unwrap_or((Money::zero(self.currency), 0));
                let average_monthly = total.divide(months_covered);
                let average_yearly = average_monthly.multiply(12);
                let monthly_budget_contribution = Self::periodised_monthly(
                    tag,
                    &all_txs,
                    max_date,
                    average_monthly,
                );
                TagSummary {
                    tag_id: tag.id,
                    name: tag.name.clone(),
                    cost_kind: tag.cost_kind,
                    matching: tag.matching,
                    needs_review: tag.needs_review,
                    average_monthly,
                    average_yearly,
                    monthly_budget_contribution,
                    transaction_count: count,
                }
            })
            .collect()
    }

    /// What to budget per month for `tag`.
    ///
    /// A cost billed less often than monthly is spread across its cycle: the
    /// trailing cycle's actual spend divided by the cycle length, so a single
    /// 12 000 kr annual insurance becomes 1 000 kr/month. Using the *trailing
    /// cycle* rather than the whole-history average matters because the naive
    /// average is badly skewed by a partial window — 13 months containing two
    /// annual payments averages to 1 846 kr/month, not 1 000.
    ///
    /// Monthly and `Variable` costs keep the whole-history average, which is a
    /// better estimator for them than any single recent month (electricity
    /// swings seasonally; groceries swing weekly).
    fn periodised_monthly(
        tag: &Tag,
        all_txs: &[&BankTransaction],
        max_date: DateTime<Utc>,
        average_monthly: Money,
    ) -> Money {
        let Some(cycle_months) = tag.cost_kind.cycle_months() else {
            return average_monthly;
        };
        if cycle_months <= 1 {
            return average_monthly;
        }
        let cycle_start = max_date
            .checked_sub_months(chrono::Months::new(
                u32::try_from(cycle_months).unwrap_or(12),
            ))
            .unwrap_or(max_date);
        let trailing: Money = all_txs
            .iter()
            .filter(|tx| tx.tag_id == Some(tag.id) && tx.date > cycle_start)
            .map(|tx| tx.amount)
            .sum();
        if trailing.amount_in_cents() == 0 {
            // No billing observed in the trailing cycle — fall back rather than
            // claiming this cost is free.
            return average_monthly;
        }
        trailing.divide(cycle_months)
    }

    pub fn get_next_untagged_transaction(&self) -> Option<&BankTransaction> {
        self.periods
            .iter()
            .flat_map(|p| p.transactions.iter())
            .find(|tx| tx.tag_id.is_none() && !tx.ignored)
    }

    /// Rule matches that may be applied **without asking**, i.e. those whose
    /// tag is [`Matching::Automatic`] — in practice bills, whose payee text is
    /// stable enough to trust.
    ///
    /// Matches against [`Matching::Suggest`] tags are deliberately excluded;
    /// see [`Self::suggest_tag_rules`].
    pub fn evaluate_tag_rules(&self) -> Vec<(Uuid, Uuid)> {
        self.rule_matches(Matching::Automatic)
    }

    /// Rule matches that need the user to confirm them — card spending, where a
    /// payee maps to a category only most of the time.
    ///
    /// Same matching engine as [`Self::evaluate_tag_rules`]; only the tag's
    /// [`Matching`] mode decides which side a match lands on, so the two are
    /// always disjoint and together cover every match.
    pub fn suggest_tag_rules(&self) -> Vec<(Uuid, Uuid)> {
        self.rule_matches(Matching::Suggest)
    }

    fn rule_matches(&self, mode: Matching) -> Vec<(Uuid, Uuid)> {
        let mut matches = Vec::new();
        for period in &self.periods {
            for tx in &period.transactions {
                if tx.tag_id.is_some() || tx.ignored {
                    continue;
                }
                // Tokenize once per transaction, not once per (transaction,
                // rule) pair — with hundreds of rules this turned an
                // O(transactions) scan into O(transactions × rules).
                let tokens: HashSet<String> =
                    crate::models::match_rule::tokenize_description(&tx.description)
                        .into_iter()
                        .collect();
                for rule in &self.match_rules {
                    if let Some(tag_id) = rule.tag_id
                        && rule.matches_tokens(&tokens)
                    {
                        // A rule pointing at a deleted or unknown tag is inert.
                        if let Some(tag) = self.tags.iter().find(|t| t.id == tag_id && !t.deleted)
                            && tag.matching == mode
                        {
                            matches.push((tx.id, tag_id));
                        }
                        break;
                    }
                }
            }
        }
        matches
    }

    pub fn preview_rule_matches(&self, tx_id: Uuid) -> Vec<BankTransaction> {
        let Some(tx) = self.get_transaction(tx_id) else {
            return Vec::new();
        };
        let key = MatchRule::create_transaction_key(tx);
        self.periods
            .iter()
            .flat_map(|p| p.transactions.iter())
            .filter(|t| t.id != tx_id && MatchRule::create_transaction_key(t) == key)
            .cloned()
            .collect()
    }

    pub fn get_budgeted_by_type(
        &self,
        budgeting_type: &BudgetingType,
        period_id: PeriodId,
    ) -> Money {
        match self.get_period(period_id) {
            Some(p) => p.budgeted_for_type(*budgeting_type),
            None => Money::zero(self.currency),
        }
    }

    pub fn get_actual_by_type(&self, budgeting_type: &BudgetingType, period_id: PeriodId) -> Money {
        match self.get_period(period_id) {
            Some(p) => p.spent_for_type(*budgeting_type),
            None => Money::zero(self.currency),
        }
    }

    pub fn get_budgeting_overview(
        &self,
        budgeting_type: BudgetingType,
        period_id: PeriodId,
    ) -> BudgetingTypeOverview {
        match budgeting_type {
            BudgetingType::Expense => self
                .get_period(period_id)
                .map(|p| p.get_expense_overview(&self.rules))
                .unwrap_or_default(),
            BudgetingType::Income => self
                .get_period(period_id)
                .map(|p| p.get_income_overview(&self.rules))
                .unwrap_or_default(),
            BudgetingType::Savings => self
                .get_period(period_id)
                .map(|p| p.get_savings_overview(&self.rules))
                .unwrap_or_default(),
            BudgetingType::InternalTransfer => self
                .get_period(period_id)
                .map(|p| p.get_transfer_overview(&self.rules))
                .unwrap_or_default(),
        }
    }

    pub fn set_transaction_ignored(&mut self, tx_id: Uuid) -> bool {
        match self.get_transaction_mut(tx_id) {
            Some(tx) => {
                if tx.ignored {
                    return false;
                }
                tx.ignored = true;
                tx.actual_id = None;
                true
            }
            None => false,
        }
    }

    pub fn transactions_for_actual(
        &self,
        period_id: PeriodId,
        actual_id: Uuid,
        sorted: bool,
    ) -> Vec<&BankTransaction> {
        self.get_period(period_id)
            .map(|p| p.transactions_for_actual(actual_id, sorted))
            .unwrap_or_default()
    }

    pub fn unconnected_transactions(&self, period_id: PeriodId) -> Vec<&BankTransaction> {
        self.get_period(period_id)
            .map(|p| {
                p.transactions
                    .iter()
                    .filter(|t| {
                        !t.ignored
                            && t.actual_id.is_none()
                            && t.tag_id.is_none()
                            && !p.allocations.iter().any(|a| a.transaction_id == t.id)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn connected_transactions(&self, period_id: PeriodId) -> Vec<&BankTransaction> {
        self.get_period(period_id)
            .map(|p| {
                p.transactions
                    .iter()
                    .filter(|t| {
                        t.actual_id.is_some()
                            || p.allocations.iter().any(|a| a.transaction_id == t.id)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn all_transactions(&self) -> Vec<&BankTransaction> {
        self.periods
            .iter()
            .flat_map(|p| p.transactions.iter())
            .collect()
    }

    pub fn all_transactions_mut(&mut self) -> Vec<&mut BankTransaction> {
        self.periods
            .iter_mut()
            .flat_map(|p| p.transactions.iter_mut())
            .collect()
    }

    pub fn create_period_before(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
        let period_id = period_id.month_before();
        self.get_or_create_period(period_id)
    }

    /// # Panics
    /// Never panics: the `key` was just found in `self.periods`, so `find` always succeeds.
    pub fn get_period_before(&self, id: PeriodId) -> Option<&BudgetPeriod> {
        if self.periods.is_empty() {
            return None;
        }
        self.periods
            .iter()
            .map(|p| p.id)
            .filter(|key| key < &id)
            .max()
            .map(|key| self.periods.iter().find(|p| p.id == key).unwrap())
    }

    fn get_or_create_period(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
        let idx = if let Some(idx) = self.periods.iter().position(|p| p.id == period_id) {
            idx
        } else {
            let previous_period = self.get_period_before(period_id);
            let period = if let Some(previous_period) = previous_period {
                previous_period.clone_to(period_id)
            } else {
                BudgetPeriod::new(period_id)
            };
            self.periods.push(period);
            self.periods.len() - 1
        };
        &mut self.periods[idx]
    }

    pub fn create_period_after(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
        let period_id = period_id.month_after();
        self.get_or_create_period(period_id)
    }

    pub fn potential_internal_transfers(&self) -> Vec<(Uuid, Uuid)> {
        let all_txs: Vec<&BankTransaction> = self.all_transactions();
        let allocated_ids: std::collections::HashSet<Uuid> = self
            .periods
            .iter()
            .flat_map(|p| p.allocations.iter().map(|a| a.transaction_id))
            .collect();
        let unallocated: Vec<&BankTransaction> = all_txs
            .into_iter()
            .filter(|tx| !tx.ignored && tx.actual_id.is_none() && !allocated_ids.contains(&tx.id))
            .collect();

        // Group by abs(cents) so each tx only checks candidates with the matching amount.
        let mut by_amount: std::collections::HashMap<i64, Vec<&BankTransaction>> =
            std::collections::HashMap::new();
        for tx in &unallocated {
            by_amount
                .entry(tx.amount.abs().amount_in_cents())
                .or_default()
                .push(tx);
        }

        let mut pairs: Vec<(Uuid, Uuid)> = Vec::new();
        let mut used: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
        for tx in &unallocated {
            if used.contains(&tx.id) {
                continue;
            }
            let Some(candidates) = by_amount.get(&tx.amount.abs().amount_in_cents()) else {
                continue;
            };
            if let Some(counterpart) = candidates.iter().find(|other| {
                other.id != tx.id
                    && !used.contains(&other.id)
                    && other.account_number != tx.account_number
                    && other.amount == -tx.amount
                    && (other.date - tx.date).num_days().abs() <= 3
                    && !self.rejected_transfer_pairs.contains(&(tx.id, other.id))
                    && !self.rejected_transfer_pairs.contains(&(other.id, tx.id))
            }) {
                used.insert(tx.id);
                used.insert(counterpart.id);
                pairs.push((tx.id, counterpart.id));
            }
        }
        pairs
    }

    /// Potential transfer pairs (see [`Self::potential_internal_transfers`])
    /// that also match a previously learned [`TransferRule`], paired with
    /// the resolution that rule implies — `(outgoing_id, incoming_id,
    /// tag_id)`, in the same shape `resolve_transfer_pair` expects.
    ///
    /// Always a **suggestion**: the caller decides whether to show it for
    /// confirmation or apply it via a bulk "confirm all" action — this never
    /// resolves anything on its own, since it moves money between
    /// accounts/tags.
    pub fn suggested_transfer_resolutions(&self) -> Vec<(Uuid, Uuid, Option<Uuid>)> {
        self.potential_internal_transfers()
            .into_iter()
            .filter_map(|(a_id, b_id)| {
                let a = self.get_transaction(a_id)?;
                let b = self.get_transaction(b_id)?;
                self.transfer_rules.iter().find_map(|rule| {
                    rule.matches_pair(a, b)
                        .map(|(out_id, in_id)| (out_id, in_id, rule.tag_id))
                })
            })
            .collect()
    }

    pub fn evaluate_rules(&self) -> Vec<RuleMatch> {
        self.periods
            .iter()
            .flat_map(|p| p.evaluate_rules(&self.match_rules, &self.items))
            .collect::<Vec<_>>()
    }

    pub fn contains_period(&self, period_id: PeriodId) -> bool {
        self.periods.iter().any(|p| p.id == period_id)
    }

    fn ensure_period(&mut self, period_id: PeriodId) -> usize {
        if let Some(idx) = self.periods.iter().position(|p| p.id == period_id) {
            idx
        } else {
            self.periods.push(BudgetPeriod::new(period_id));
            self.periods.len() - 1
        }
    }

    pub fn with_period_mut(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
        let idx = self.ensure_period(period_id);
        &mut self.periods[idx]
    }

    pub fn with_period(&mut self, period_id: PeriodId) -> &BudgetPeriod {
        let idx = self.ensure_period(period_id);
        &self.periods[idx]
    }

    pub fn mutate_actual(
        &mut self,
        period_id: PeriodId,
        actual_id: Uuid,
        mutator: impl FnMut(&mut ActualItem),
    ) {
        self.with_period_mut(period_id)
            .mutate_actual(actual_id, mutator);
    }

    pub fn allocations_for_actual(
        &self,
        period_id: PeriodId,
        actual_id: Uuid,
    ) -> Vec<&TransactionAllocation> {
        self.get_period(period_id)
            .map(|p| p.allocations_for_actual(actual_id))
            .unwrap_or_default()
    }

    pub fn allocations_for_transaction(&self, transaction_id: Uuid) -> Vec<&TransactionAllocation> {
        self.get_period_for_transaction(transaction_id)
            .map(|p| p.allocations_for_transaction(transaction_id))
            .unwrap_or_default()
    }

    pub fn allocated_amount_for_actual(&self, period_id: PeriodId, actual_id: Uuid) -> Money {
        self.get_period(period_id)
            .map_or(Money::zero(self.currency), |p| p.allocated_amount_for_actual(actual_id))
    }
}

// --- Aggregate implementation ---
impl Aggregate for Budget {
    type Id = Uuid;

    fn _new(id: Self::Id) -> Self {
        Self {
            id,
            ..Self::default()
        }
    }

    fn _default() -> Self {
        Self::default()
    }

    fn update_timestamp(&mut self, timestamp: i64, updated_at: DateTime<Utc>) {
        if self.last_event < timestamp {
            self.last_event = timestamp;
            self.updated_at = updated_at;
            if self.version == 0 {
                self.created_at = updated_at;
            }
            self.version += 1;
        } else {
            panic!("Event timestamp is older than last event timestamp");
        }
    }

    fn version(&self) -> i64 {
        self.version
    }
}
