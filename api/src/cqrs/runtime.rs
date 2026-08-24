
use crate::api_error::RustyError;
use crate::cqrs::framework::{AsyncRuntime, CommandError, Runtime, StoredEvent};
const DEFAULT_USER_EMAIL: &str = "tommie.nygren@gmail.com";
use crate::models::{User, Budget, BankAccountType, MonthBeginsOn, Currency, BudgetingType, CostKind, Matching, Money, PeriodId, Periodicity, BudgetEvent};
use crate::view_models::BudgetSummary;
#[cfg(feature = "server")]
use crate::pg_models::{PgBankTransaction, PgBudget, PgStoredBudgetEvent, PgUser, PgUserBudgets};
use crate::{cqrs, models};
use chrono::{DateTime, NaiveDate, Utc};
use dioxus::logger::tracing;
use dioxus::prelude::{debug, error, info};
use joydb::Model as JoyModel;
use joydb::adapters::{FromPath, JsonAdapter};
use joydb::{Joydb, JoydbConfig, JoydbMode, SyncPolicy};
use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use serde::{Deserializer, Serializer};
#[cfg(feature = "server")]
use sqlx::Any;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;
#[cfg(feature = "server")]
use welds::Client;
#[cfg(feature = "server")]
use welds::connections::any::AnyClient;
#[cfg(feature = "server")]
use welds::{Syntax, WeldsError, prelude::*};

fn get_data_file() -> PathBuf {
    env::var("DATA_FILE").map_or_else(|_| {
            info!("DATA_FILE not set, using default data.json");
            PathBuf::from("data.json")
        }, PathBuf::from)
}

#[cfg(feature = "server")]
pub async fn migrate_to_postgres() -> Result<(), RustyError> {
    let jr = JoyDbBudgetRuntime::new(get_data_file());
    let pr = create_runtime().await;
    /*
    StoredBudgetEvent, Budget, User, UserBudgets
     */

    let users = jr.db.get_all::<User>()?;
    for user in users {
        info!("Migrating user {}", user.id);
        let mut pg_user: DbState<PgUser> = user.into();
        pg_user.save(pr.client.as_ref()).await?;
    }

    let events = jr.db.get_all::<StoredBudgetEvent>()?;
    for event in events {
        info!("Migrating event {:?}", event);
        let mut pg_event: DbState<PgStoredBudgetEvent> = event.into();
        pg_event.save(pr.client.as_ref()).await?;
    }

    let budgets = jr.db.get_all::<Budget>()?;
    for budget in budgets {
        info!("Migrating budget {:?}", budget);
        let mut pg_budget: DbState<PgBudget> = budget.into();
        pg_budget.save(pr.client.as_ref()).await?;
    }

    let user_budgets = jr.db.get_all::<UserBudgets>()?;
    for budget in user_budgets {
        info!("Migrating user budgets {:?}", budget);
        let mut pg_budget: DbState<PgUserBudgets> = budget.into();
        pg_budget.save(pr.client.as_ref()).await?;
    }
    Ok(())
}

impl BudgetCommandsTrait for JoyDbBudgetRuntime {
    fn create_budget(
        &self,
        user_id: Uuid,
        budget_name: &str,
        default_budget: bool,
        month_begins_on: MonthBeginsOn,
        currency: Currency,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, Uuid::default(), |budget| {
            budget.create_budget(
                budget_name.to_string(),
                user_id,
                month_begins_on,
                default_budget,
                currency,
            )
        })
    }
    fn add_item(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_name: String,
        item_type: BudgetingType,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.add_item(item_name.clone(), item_type)
        })
    }
    fn add_actual(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        amount: Money,
        period_id: PeriodId,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.add_actual(item_id, period_id, amount)
        })
    }
    #[allow(clippy::too_many_arguments)]
    fn modify_item(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        name: Option<String>,
        item_type: Option<BudgetingType>,
        tag_ids: Option<Vec<Uuid>>,
        periodicity: Option<Periodicity>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.modify_item(item_id, name, item_type, tag_ids, periodicity)
        })
    }
    fn create_tag(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        name: String,
        periodicity: Periodicity,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.create_tag(name, periodicity)
        })
    }
    fn modify_tag(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tag_id: Uuid,
        name: Option<String>,
        periodicity: Option<Periodicity>,
        deleted: Option<bool>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.modify_tag(tag_id, name, periodicity, deleted)
        })
    }
    fn modify_bank_account(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        account_id: Uuid,
        account_type: Option<BankAccountType>,
        description: Option<String>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.modify_bank_account(account_id, account_type, description)
        })
    }
    fn normalize_account_numbers(&self, user_id: Uuid, budget_id: Uuid) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, Budget::normalize_account_numbers)
    }
    fn classify_tag(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tag_id: Uuid,
        cost_kind: CostKind,
        matching: Matching,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.classify_tag(tag_id, cost_kind, matching)
        })
    }
    fn configure_carryover(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        from_period: Option<PeriodId>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.configure_carryover(from_period)
        })
    }
    #[allow(clippy::too_many_arguments)]
    fn modify_actual(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        period_id: PeriodId,
        budgeted_amount: Option<Money>,
        actual_amount: Option<Money>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.modify_actual(
                actual_id,
                period_id,
                budgeted_amount,
                actual_amount,
                None,
                None,
            )
        })
    }
    #[allow(clippy::too_many_arguments)]
    fn add_and_connect_tx(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        bank_account_number: &str,
        amount: Money,
        balance: Money,
        description: &str,
        date: DateTime<Utc>,
    ) -> Result<Uuid, RustyError> {
        let tx_id = self.add_transaction(
            user_id,
            budget_id,
            bank_account_number,
            amount,
            balance,
            description,
            date,
        )?;
        self.connect_transaction(user_id, budget_id, tx_id, actual_id)
    }
    #[allow(clippy::too_many_arguments)]
    fn add_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        bank_account_number: &str,
        amount: Money,
        balance: Money,
        description: &str,
        date: DateTime<Utc>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.add_transaction(
                bank_account_number.to_string(),
                amount,
                balance,
                description.to_string(),
                date,
            )
        })
    }
    fn connect_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
        actual_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        let (amount, existing_allocations) = {
            let budget = self.load(budget_id)?;
            let amount = budget
                .get_transaction(tx_id)
                .map(|tx| tx.amount)
                .ok_or_else(|| {
                    RustyError::ItemNotFound(tx_id.to_string(), "Transaction not found".to_string())
                })?;
            let existing = budget
                .allocations_for_transaction(tx_id)
                .iter()
                .map(|a| (a.id, a.transaction_id))
                .collect::<Vec<_>>();
            (amount, existing)
        };
        for (alloc_id, transaction_id) in existing_allocations {
            self.delete_allocation(user_id, budget_id, alloc_id, transaction_id)?;
        }
        self.create_allocation(user_id, budget_id, tx_id, actual_id, amount, String::new())
    }
    fn ensure_account(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        account_number: &str,
        description: &str,
    ) -> Result<Uuid, RustyError> {
        let budget = self.load(budget_id)?;
        if let Some(existing) = budget.get_account(account_number) {
            return Ok(existing.id);
        }
        self.cmd(user_id, budget_id, |budget| {
            budget.create_bank_account(account_number.to_string(), description.to_string())
        })
    }
    fn ignore_transaction(
        &self,
        budget_id: Uuid,
        tx_id: Uuid,
        user_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.ignore_transaction(tx_id)
        })
    }
    fn reallocate_budgeted_funds(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        period_id: PeriodId,
        from_actual_id: Uuid,
        to_actual_id: Uuid,
        amount: Money,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.reallocate_budgeted_funds(period_id, from_actual_id, to_actual_id, amount)
        })
    }
    fn adjust_budgeted_amount(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        period_id: PeriodId,
        budgeted_amount: Money,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.adjust_actual_budgeted_funds(actual_id, period_id, budgeted_amount)
        })
    }
    fn add_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        transaction_key: Vec<String>,
        item_key: Vec<String>,
        always_apply: bool,
        tag_id: Option<Uuid>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.add_rule(transaction_key, item_key, always_apply, tag_id)
        })
    }
    fn tag_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
        tag_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.do_transaction_tagged(tx_id, tag_id)
        })
    }
    fn untag_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.do_transaction_untagged(tx_id)
        })
    }
    fn reject_transfer_pair(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        outgoing_tx_id: Uuid,
        incoming_tx_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.reject_transfer_pair(outgoing_tx_id, incoming_tx_id)
        })
    }
    fn modify_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        rule_id: Uuid,
        transaction_key: Vec<String>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.modify_rule(rule_id, transaction_key)
        })
    }
    fn delete_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        rule_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| budget.delete_rule(rule_id))
    }
    fn set_item_buffer(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        buffer_target: Option<Money>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.set_item_buffer(item_id, buffer_target)
        })
    }
    fn create_allocation(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        transaction_id: Uuid,
        actual_id: Uuid,
        amount: Money,
        tag: String,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.create_allocation(transaction_id, actual_id, amount, tag)
        })
    }
    fn delete_allocation(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        allocation_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.delete_allocation(allocation_id, transaction_id)
        })
    }

    fn user_exists(&self, email: &str) -> Result<bool, RustyError> {
        let users = self.db.get_all_by(|u: &User| u.email == email)?;
        Ok(!users.is_empty())
    }

    fn get_default_user(&self) -> Result<User, RustyError> {
        match self.db.get_all_by(|u: &User| u.email == DEFAULT_USER_EMAIL) {
            Ok(mut users) => {
                if users.is_empty() {
                    self.create_user(
                        "tommie",
                        DEFAULT_USER_EMAIL,
                        "Tommie",
                        "Nygren",
                        Some("0704382781".to_string()),
                        Some(
                            NaiveDate::parse_from_str("1973-05-12", "%Y-%m-%d").unwrap_or_default(),
                        ),
                    )
                } else {
                    Ok(users.remove(0))
                }
            }
            Err(e) => {
                error!(error = %e, "Could not get default user");
                Err(RustyError::JoydbError(e))
            }
        }
    }

    fn get_default_budget(&self, user_id: Uuid) -> Result<Budget, RustyError> {
        let user_budgets = self.db.get::<UserBudgets>(&user_id)?;
        match user_budgets {
            None => {
                info!("User has no budgets");
                Err(RustyError::DefaultBudgetNotFound)
            }
            Some(b) => if let Some((budget_id, _)) = b.budgets.iter().find(|(_, default)| *default) { Ok(self.load(*budget_id)?) } else {
                info!("User had budgets but none were default");
                Err(RustyError::DefaultBudgetNotFound)
            },
        }
    }

    fn list_budgets(&self, user_id: Uuid) -> Result<Vec<BudgetSummary>, RustyError> {
        let user_budgets = self.db.get::<UserBudgets>(&user_id)?;
        let Some(user_budgets) = user_budgets else {
            return Ok(Vec::new());
        };
        user_budgets
            .budgets
            .iter()
            .map(|(budget_id, default)| {
                let budget = self.load(*budget_id)?;
                Ok(BudgetSummary {
                    id: budget.id,
                    name: budget.name,
                    default: *default,
                })
            })
            .collect()
    }

    fn add_budget_to_user(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        default: bool,
    ) -> Result<Uuid, RustyError> {
        let user_budgets = self.db.get::<UserBudgets>(&user_id)?;
        // Drop any existing entry for this budget first (its `default` flag
        // may differ from the one being set now — a plain `contains` check
        // would miss that and leave a stale duplicate entry behind), then
        // clear every other default before adding this one back.
        let mut budgets: Vec<(Uuid, bool)> = user_budgets
            .map(|b| b.budgets)
            .unwrap_or_default()
            .into_iter()
            .filter(|(id, _)| *id != budget_id)
            .collect();
        if default {
            for entry in &mut budgets {
                entry.1 = false;
            }
        }
        budgets.push((budget_id, default));
        let list = UserBudgets { id: user_id, budgets };
        match self.db.upsert(&list) {
            Ok(()) => Ok(user_id),
            Err(e) => Err(RustyError::JoydbError(e)),
        }
    }

    fn create_user(
        &self,
        user_name: &str,
        email: &str,
        first_name: &str,
        last_name: &str,
        phone: Option<String>,
        birthday: Option<NaiveDate>,
    ) -> Result<User, RustyError> {
        let user = User::new(user_name, email, first_name, last_name, phone, birthday);
        self.db.insert(&user)?;
        Ok(user)
    }
}

pub trait BudgetCommandsTrait {
    fn create_budget(
        &self,
        user_id: Uuid,
        budget_name: &str,
        default_budget: bool,
        month_begins_on: MonthBeginsOn,
        currency: Currency,
    ) -> Result<Uuid, RustyError>;
    fn add_item(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_name: String,
        item_type: BudgetingType,
    ) -> Result<Uuid, RustyError>;
    fn add_actual(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        amount: Money,
        period_id: PeriodId,
    ) -> Result<Uuid, RustyError>;
    #[allow(clippy::too_many_arguments)]
    fn modify_item(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        name: Option<String>,
        item_type: Option<BudgetingType>,
        tag_ids: Option<Vec<Uuid>>,
        periodicity: Option<Periodicity>,
    ) -> Result<Uuid, RustyError>;
    fn create_tag(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        name: String,
        periodicity: Periodicity,
    ) -> Result<Uuid, RustyError>;
    fn modify_tag(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tag_id: Uuid,
        name: Option<String>,
        periodicity: Option<Periodicity>,
        deleted: Option<bool>,
    ) -> Result<Uuid, RustyError>;
    fn modify_bank_account(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        account_id: Uuid,
        account_type: Option<BankAccountType>,
        description: Option<String>,
    ) -> Result<Uuid, RustyError>;
    fn normalize_account_numbers(&self, user_id: Uuid, budget_id: Uuid) -> Result<Uuid, RustyError>;
    fn classify_tag(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tag_id: Uuid,
        cost_kind: CostKind,
        matching: Matching,
    ) -> Result<Uuid, RustyError>;
    fn configure_carryover(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        from_period: Option<PeriodId>,
    ) -> Result<Uuid, RustyError>;
    #[allow(clippy::too_many_arguments)]
    fn modify_actual(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        period_id: PeriodId,
        budgeted_amount: Option<Money>,
        actual_amount: Option<Money>,
    ) -> Result<Uuid, RustyError>;
    #[allow(clippy::too_many_arguments)]
    fn add_and_connect_tx(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        bank_account_number: &str,
        amount: Money,
        balance: Money,
        description: &str,
        date: DateTime<Utc>,
    ) -> Result<Uuid, RustyError>;
    #[allow(clippy::too_many_arguments)]
    fn add_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        bank_account_number: &str,
        amount: Money,
        balance: Money,
        description: &str,
        date: DateTime<Utc>,
    ) -> Result<Uuid, RustyError>;
    fn connect_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
        actual_id: Uuid,
    ) -> Result<Uuid, RustyError>;
    fn ensure_account(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        account_number: &str,
        description: &str,
    ) -> Result<Uuid, RustyError>;
    fn ignore_transaction(
        &self,
        budget_id: Uuid,
        tx_id: Uuid,
        user_id: Uuid,
    ) -> Result<Uuid, RustyError>;
    fn reallocate_budgeted_funds(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        period_id: PeriodId,
        from_actual_id: Uuid,
        to_actual_id: Uuid,
        amount: Money,
    ) -> Result<Uuid, RustyError>;
    fn adjust_budgeted_amount(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        period_id: PeriodId,
        budgeted_amount: Money,
    ) -> Result<Uuid, RustyError>;
    fn add_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        transaction_key: Vec<String>,
        item_key: Vec<String>,
        always_apply: bool,
        tag_id: Option<Uuid>,
    ) -> Result<Uuid, RustyError>;
    fn tag_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
        tag_id: Uuid,
    ) -> Result<Uuid, RustyError>;
    fn untag_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
    ) -> Result<Uuid, RustyError>;
    fn reject_transfer_pair(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        outgoing_tx_id: Uuid,
        incoming_tx_id: Uuid,
    ) -> Result<Uuid, RustyError>;
    fn modify_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        rule_id: Uuid,
        transaction_key: Vec<String>,
    ) -> Result<Uuid, RustyError>;
    fn delete_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        rule_id: Uuid,
    ) -> Result<Uuid, RustyError>;
    fn set_item_buffer(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        buffer_target: Option<Money>,
    ) -> Result<Uuid, RustyError>;
    fn create_allocation(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        transaction_id: Uuid,
        actual_id: Uuid,
        amount: Money,
        tag: String,
    ) -> Result<Uuid, RustyError>;
    fn delete_allocation(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        allocation_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<Uuid, RustyError>;

    fn user_exists(&self, email: &str) -> Result<bool, RustyError>;
    fn get_default_user(&self) -> Result<User, RustyError>;
    fn get_default_budget(&self, user_id: Uuid) -> Result<Budget, RustyError>;
    fn list_budgets(&self, user_id: Uuid) -> Result<Vec<BudgetSummary>, RustyError>;
    fn add_budget_to_user(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        default: bool,
    ) -> Result<Uuid, RustyError>;
    fn create_user(
        &self,
        user_name: &str,
        email: &str,
        first_name: &str,
        last_name: &str,
        phone: Option<String>,
        birthday: Option<NaiveDate>,
    ) -> Result<User, RustyError>;
}

#[allow(async_fn_in_trait)]
pub trait AsyncBudgetCommandsTrait {
    async fn create_budget(
        &self,
        user_id: Uuid,
        budget_name: &str,
        default_budget: bool,
        month_begins_on: MonthBeginsOn,
        currency: Currency,
    ) -> Result<Uuid, RustyError>;
    async fn add_item(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_name: String,
        item_type: BudgetingType,
    ) -> Result<Uuid, RustyError>;
    async fn add_actual(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        amount: Money,
        period_id: PeriodId,
    ) -> Result<Uuid, RustyError>;
    #[allow(clippy::too_many_arguments)]
    async fn modify_item(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        name: Option<String>,
        item_type: Option<BudgetingType>,
        tag_ids: Option<Vec<Uuid>>,
        periodicity: Option<Periodicity>,
    ) -> Result<Uuid, RustyError>;
    async fn create_tag(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        name: String,
        periodicity: Periodicity,
    ) -> Result<Uuid, RustyError>;
    async fn modify_tag(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tag_id: Uuid,
        name: Option<String>,
        periodicity: Option<Periodicity>,
        deleted: Option<bool>,
    ) -> Result<Uuid, RustyError>;
    async fn modify_bank_account(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        account_id: Uuid,
        account_type: Option<BankAccountType>,
        description: Option<String>,
    ) -> Result<Uuid, RustyError>;
    async fn normalize_account_numbers(&self, user_id: Uuid, budget_id: Uuid) -> Result<Uuid, RustyError>;
    async fn classify_tag(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tag_id: Uuid,
        cost_kind: CostKind,
        matching: Matching,
    ) -> Result<Uuid, RustyError>;
    async fn configure_carryover(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        from_period: Option<PeriodId>,
    ) -> Result<Uuid, RustyError>;
    #[allow(clippy::too_many_arguments)]
    async fn modify_actual(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        period_id: PeriodId,
        budgeted_amount: Option<Money>,
        actual_amount: Option<Money>,
    ) -> Result<Uuid, RustyError>;
    #[allow(clippy::too_many_arguments)]
    async fn add_and_connect_tx(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        bank_account_number: &str,
        amount: Money,
        balance: Money,
        description: &str,
        date: DateTime<Utc>,
    ) -> Result<Uuid, RustyError>;
    #[allow(clippy::too_many_arguments)]
    async fn add_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        bank_account_number: &str,
        amount: Money,
        balance: Money,
        description: &str,
        date: DateTime<Utc>,
    ) -> Result<Uuid, RustyError>;
    async fn connect_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
        actual_id: Uuid,
    ) -> Result<Uuid, RustyError>;
    async fn ensure_account(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        account_number: &str,
        description: &str,
    ) -> Result<Uuid, RustyError>;
    async fn ignore_transaction(
        &self,
        budget_id: Uuid,
        tx_id: Uuid,
        user_id: Uuid,
    ) -> Result<Uuid, RustyError>;
    async fn reallocate_budgeted_funds(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        period_id: PeriodId,
        from_actual_id: Uuid,
        to_actual_id: Uuid,
        amount: Money,
    ) -> Result<Uuid, RustyError>;
    async fn adjust_budgeted_amount(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        period_id: PeriodId,
        budgeted_amount: Money,
    ) -> Result<Uuid, RustyError>;
    async fn add_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        transaction_key: Vec<String>,
        item_key: Vec<String>,
        always_apply: bool,
        tag_id: Option<Uuid>,
    ) -> Result<Uuid, RustyError>;
    async fn tag_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
        tag_id: Uuid,
    ) -> Result<Uuid, RustyError>;
    async fn untag_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
    ) -> Result<Uuid, RustyError>;
    async fn reject_transfer_pair(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        outgoing_tx_id: Uuid,
        incoming_tx_id: Uuid,
    ) -> Result<Uuid, RustyError>;
    async fn modify_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        rule_id: Uuid,
        transaction_key: Vec<String>,
    ) -> Result<Uuid, RustyError>;
    async fn delete_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        rule_id: Uuid,
    ) -> Result<Uuid, RustyError>;
    async fn set_item_buffer(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        buffer_target: Option<Money>,
    ) -> Result<Uuid, RustyError>;
    async fn create_allocation(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        transaction_id: Uuid,
        actual_id: Uuid,
        amount: Money,
        tag: String,
    ) -> Result<Uuid, RustyError>;
    async fn delete_allocation(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        allocation_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<Uuid, RustyError>;

    async fn user_exists(&self, email: &str) -> Result<bool, RustyError>;
    async fn get_default_user(&self) -> Result<User, RustyError>;
    async fn get_default_budget(&self, user_id: Uuid) -> Result<Budget, RustyError>;
    async fn list_budgets(&self, user_id: Uuid) -> Result<Vec<BudgetSummary>, RustyError>;
    async fn add_budget_to_user(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        default: bool,
    ) -> Result<Uuid, RustyError>;
    async fn create_user(
        &self,
        user_name: &str,
        email: &str,
        first_name: &str,
        last_name: &str,
        phone: Option<String>,
        birthday: Option<NaiveDate>,
    ) -> Result<User, RustyError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, JoyModel)]
pub struct UserBudgets {
    pub id: Uuid,
    pub budgets: Vec<(Uuid, bool)>,
}

joydb::state! {
    AppState,
    models: [StoredBudgetEvent, Budget, User, UserBudgets],
}

pub type StoredBudgetEvent = StoredEvent<Budget, BudgetEvent>;

impl JoyModel for StoredBudgetEvent {
    type Id = Uuid;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn model_name() -> &'static str {
        "budget_event"
    }
}

pub type Db = Joydb<AppState, JsonAdapter>;

pub struct JoyDbBudgetRuntime {
    pub db: Db,
}

#[cfg(feature = "server")]
pub struct PgRuntime {
    client: Box<dyn Client>,
}

/// # Panics
/// Panics if `DATABASE_URL` is not set or the database connection cannot be established.
#[cfg(feature = "server")]
pub async fn create_runtime() -> PgRuntime {
    dotenvy::dotenv().ok();
    // Fall back to the workspace root, which is one level up from this crate
    // (`<workspace>/api`), so the server works regardless of the directory it
    // was launched from.
    let workspace_env = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join(".env"));
    if let Some(env_path) = workspace_env {
        debug!("Trying to load .env from: {:?}", env_path);
        dotenvy::from_path(&env_path).ok();
    }
    let url = env::var("DATABASE_URL").expect(
        "DATABASE_URL is not set. Copy .env.example to .env in the workspace root \
         (e.g. DATABASE_URL=sqlite://data.sqlite?mode=rwc), then run the migrations \
         with `cargo run -p api --bin api --features server`.",
    );
    let client = welds::connections::connect(&url)
        .await
        .unwrap_or_else(|e| panic!("Could not connect to DATABASE_URL `{url}`: {e}"));
    // Apply the schema before anything queries it. Idempotent (welds tracks
    // applied migrations in `_welds_migrations`), and necessary because a
    // SQLite URL with `mode=rwc` silently creates an *empty* database file, so
    // without this the first query fails with a bare "no such table: users".
    // Only the schema is automatic — importing a JoyDB file into SQL stays an
    // explicit step in the `api` binary, since that overwrites real data.
    crate::migrations::up(&client)
        .await
        .unwrap_or_else(|e| panic!("Could not apply migrations to `{url}`: {e}"));
    PgRuntime::new(client)
}

#[cfg(feature = "server")]
impl PgRuntime {
    pub fn new(client: AnyClient) -> Self {
        Self {
            client: Box::new(client),
        }
    }

    /// The underlying `welds` client, for callers (e.g. integration tests)
    /// that need to run queries `PgRuntime`'s own API doesn't expose.
    pub fn client(&self) -> &dyn Client {
        self.client.as_ref()
    }

    async fn cmd<F, E>(&self, user_id: Uuid, id: Uuid, command: F) -> Result<Uuid, RustyError>
    where
        F: FnOnce(&Budget) -> Result<E, CommandError>,
        E: Into<BudgetEvent>,
    {
        self.execute(user_id, id, |aggregate| {
            command(aggregate).map(std::convert::Into::into)
        })
        .await
    }
}

impl JoyDbBudgetRuntime {
    /// # Panics
    /// Panics if the database cannot be opened at the given path.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let adapter = JsonAdapter::from_path(path);
        let config = JoydbConfig {
            mode: JoydbMode::Persistent {
                adapter,
                sync_policy: SyncPolicy::Periodic(Duration::from_secs(30)),
            },
        };
        Self {
            db: Db::open_with_config(config).unwrap(),
        }
    }

    /// # Panics
    /// Panics if the in-memory database cannot be created.
    pub fn new_in_memory() -> Self {
        Self {
            db: Db::new_in_memory().unwrap(),
        }
    }

    /// Ergonomic command execution - eliminates all the boilerplate!
    /// Usage: rt.cmd(id, |budget| `budget.create_budget(name`, `user_id`, default))
    fn cmd<F, E>(&self, user_id: Uuid, id: Uuid, command: F) -> Result<Uuid, RustyError>
    where
        F: FnOnce(&Budget) -> Result<E, CommandError>,
        E: Into<BudgetEvent>,
    {
        self.execute(user_id, id, |aggregate| {
            command(aggregate).map(std::convert::Into::into)
        })
    }
}

impl Runtime<Budget, BudgetEvent> for JoyDbBudgetRuntime {
    fn load(&self, id: Uuid) -> Result<Budget, RustyError> {
        let t = std::time::Instant::now();
        let budget = self.db.get::<Budget>(&id)?;

        tracing::debug!("Loaded budget is some: {}", budget.is_some());
        let mut budget = budget.unwrap_or(Budget::new(id));
        let version = budget.version;
        tracing::debug!(
            "Loaded budget has version {} and last event at {}",
            version,
            budget.last_event
        );
        let events = self.fetch_events(id, budget.last_event)?;
        let event_count = events.len();
        for ev in events {
            ev.apply(&mut budget);
        }
        info!(
            "[perf] load: replayed {} events in {:?}",
            event_count,
            t.elapsed()
        );
        if event_count > 0 {
            self.snapshot(&budget)?;
        }
        Ok(budget)
    }

    fn snapshot(&self, agg: &Budget) -> Result<(), RustyError> {
        self.db.upsert(agg)?;
        Ok(())
    }

    fn append(&self, user_id: Uuid, ev: BudgetEvent) -> Result<(), RustyError> {
        let stored_event = StoredEvent::new(ev, user_id);
        self.db.insert(&stored_event)?;
        Ok(())
    }

    fn fetch_events(
        &self,
        id: Uuid,
        last_timestamp: i64,
    ) -> Result<Vec<StoredBudgetEvent>, RustyError> {
        let mut events: Vec<StoredBudgetEvent> = self.db.get_all_by(|e: &StoredBudgetEvent| {
            e.aggregate_id == id && e.timestamp > last_timestamp
        })?;
        events.sort_by_key(|e| e.timestamp);
        Ok(events)
    }
    fn get_budget(&self, id: Uuid) -> Result<Option<Budget>, RustyError> {
        let budget = self.db.get::<Budget>(&id)?;
        Ok(budget)
    }
    fn undo_last(&self, budget_id: Uuid) -> Result<bool, RustyError> {
        let mut events = self.events(budget_id)?;
        if events.is_empty() {
            return Ok(false);
        }
        events.sort_by_key(|e| e.timestamp);
        let last_event_id = events.last().unwrap().id;
        self.db.delete::<StoredBudgetEvent>(&last_event_id)?;
        Ok(true)
    }
    fn events(&self, id: Uuid) -> Result<Vec<StoredBudgetEvent>, RustyError> {
        self.fetch_events(id, 0)
    }
}

/// Rows per delete/insert batch in `PgRuntime::snapshot`'s transaction sync,
/// comfortably under Postgres's 65535-bind-parameter-per-statement limit.
#[cfg(feature = "server")]
const TRANSACTION_SYNC_CHUNK_SIZE: usize = 1000;

#[cfg(feature = "server")]
impl AsyncRuntime<Budget, BudgetEvent> for PgRuntime {
    async fn load(
        &self,
        id: <Budget as cqrs::framework::Aggregate>::Id,
    ) -> Result<Budget, RustyError> {
        let t = std::time::Instant::now();
        let pg_budget = PgBudget::find_by_id(self.client.as_ref(), id).await?;
        tracing::debug!("Loaded budget is some: {}", pg_budget.is_some());
        let mut budget: Budget = match pg_budget {
            None => Budget::new(id),
            Some(pg_budget) => pg_budget.into(),
        };

        // Transactions live in their own table (see `snapshot`), not inline
        // in the JSON blob above — merge them in before replaying trailing
        // events, so an event like `TransactionTagged` still finds its
        // target. Harmless no-op for a legacy row whose transactions are
        // still embedded in `data` (nothing to find in the table yet).
        let tx_rows: Vec<PgBankTransaction> = PgBankTransaction::where_col(|t| t.budget_id.equal(id))
            .run(self.client.as_ref())
            .await?
            .into_iter()
            .map(welds::state::DbState::into_inner)
            .collect();
        if !tx_rows.is_empty() {
            budget.load_transactions(tx_rows.into_iter().map(Into::into).collect());
        }

        let version = budget.version;
        tracing::debug!(
            "Loaded budget has version {} and last event at {}",
            version,
            budget.last_event
        );
        let events = self.fetch_events(id, budget.last_event).await?;
        let event_count = events.len();
        for ev in events {
            ev.apply(&mut budget);
        }
        if event_count > 10 {
          info!(
            "[perf] load: replayed {} events in {:?}, budget version {}, saving",
            event_count,
            t.elapsed(),
            version
        );
            self.snapshot(&budget).await?;
        }
        Ok(budget)
    }

    async fn snapshot(&self, agg: &Budget) -> Result<(), RustyError> {
        // Transactions are persisted separately (below) rather than embedded
        // in this blob — strip them from a cloned copy before serializing so
        // every snapshot write stays small regardless of transaction volume.
        let mut slim = agg.clone();
        for period in &mut slim.periods {
            period.transactions.clear();
        }
        slim.transaction_hashes.clear();
        let slim_data = serde_json::to_value(&slim).expect("Budget must be serializable");

        let mut pg_budget: DbState<PgBudget> =
            match PgBudget::find_by_id(self.client.as_ref(), agg.id).await? {
                None => {
                    let mut fresh = DbState::<PgBudget>::from(agg);
                    fresh.data = slim_data;
                    fresh
                }
                Some(mut existing) => {
                    existing.last_event = agg.last_event;
                    existing.version = agg.version;
                    existing.data = slim_data;
                    existing
                }
            };
        pg_budget.save(self.client.as_ref()).await?;

        // Diff against what's already stored and only touch rows that
        // actually changed. A blind delete-all-then-reinsert-all here (the
        // first version of this) meant tagging a single transaction out of
        // ~10k rewrote the entire table on every call — reintroducing, on
        // the SQL side, the exact "every write pays for the whole history"
        // problem this table was built to get off the JSON blob. Bank
        // transactions are never hard-deleted anywhere in the domain, so a
        // row present here but absent from `agg` can't happen — no delete
        // path is needed for that case.
        let existing_by_id: std::collections::HashMap<Uuid, PgBankTransaction> =
            PgBankTransaction::where_col(|t| t.budget_id.equal(agg.id))
                .run(self.client.as_ref())
                .await?
                .into_iter()
                .map(|row| {
                    let row = row.into_inner();
                    (row.id, row)
                })
                .collect();

        let mut changed_ids: Vec<Uuid> = Vec::new();
        let mut changed_rows: Vec<PgBankTransaction> = Vec::new();
        for tx in agg.periods.iter().flat_map(|p| p.transactions.iter()) {
            let candidate = PgBankTransaction::from_domain(agg.id, tx).into_inner();
            if existing_by_id.get(&tx.id) != Some(&candidate) {
                changed_ids.push(tx.id);
                changed_rows.push(candidate);
            }
        }

        // Chunked, and every delete is scoped to `agg.id` as well as the
        // touched ids — never just the id. Transaction ids are otherwise
        // globally unique in this table, but scoping by budget as well means
        // a bug elsewhere (e.g. an id collision from importing a budget that
        // forgot to remint transaction ids) can corrupt at most this budget's
        // own rows, never another budget's.
        for (id_chunk, row_chunk) in changed_ids.chunks(TRANSACTION_SYNC_CHUNK_SIZE).zip(changed_rows.chunks(TRANSACTION_SYNC_CHUNK_SIZE)) {
            PgBankTransaction::where_col(|t| t.budget_id.equal(agg.id))
                .where_col(|t| t.id.in_list(id_chunk))
                .delete(self.client.as_ref())
                .await?;
            welds::query::insert::bulk_insert_with_ids(self.client.as_ref(), row_chunk).await?;
        }

        Ok(())
    }

    async fn append(&self, user_id: Uuid, ev: BudgetEvent) -> Result<(), RustyError> {
        let mut stored_event: DbState<PgStoredBudgetEvent> = StoredEvent::new(ev, user_id).into();
        stored_event.save(self.client.as_ref()).await?;
        Ok(())
    }

    async fn append_many(&self, user_id: Uuid, events: Vec<BudgetEvent>) -> Result<(), RustyError> {
        if events.is_empty() {
            return Ok(());
        }
        let rows: Vec<PgStoredBudgetEvent> = events
            .into_iter()
            .map(|ev| {
                let stored = StoredEvent::new(ev, user_id);
                let mut row = PgStoredBudgetEvent::new();
                row.id = stored.id;
                row.aggregate_id = stored.aggregate_id;
                row.timestamp = stored.timestamp;
                row.created_at = stored.created_at;
                row.user_id = stored.user_id;
                row.data = serde_json::to_value(stored.data).expect("BudgetEvent must be serializable");
                row.into_inner()
            })
            .collect();
        welds::query::insert::bulk_insert_with_ids(self.client.as_ref(), &rows).await?;
        Ok(())
    }

    async fn fetch_events(
        &self,
        id: Uuid,
        last_timestamp: i64,
    ) -> Result<Vec<StoredEvent<Budget, BudgetEvent>>, RustyError> {
        let stored_events: Vec<StoredEvent<Budget, BudgetEvent>> =
            PgStoredBudgetEvent::where_col(|ev| ev.aggregate_id.equal(id))
                .where_col(|ev| ev.timestamp.gt(last_timestamp))
                .order_by_asc(|ev| ev.timestamp)
                .run(self.client.as_ref())
                .await?
                .into_iter()
                .map(std::convert::Into::into)
                .collect();

        Ok(stored_events)
    }

    async fn get_budget(&self, id: Uuid) -> Result<Option<Budget>, RustyError> {
        match PgBudget::find_by_id(self.client.as_ref(), id).await? {
            None => Ok(None),
            Some(pg_budget) => Ok(Some(pg_budget.into())),
        }
    }

    async fn undo_last(&self, budget_id: Uuid) -> Result<bool, RustyError> {
        let id_s: Vec<EventId> =
            PgStoredBudgetEvent::where_col(|ev| ev.aggregate_id.equal(budget_id))
                .order_by_asc(|ev| ev.timestamp)
                .select_as(|ev| ev.id, "event_id")
                .limit(1)
                .run(self.client.as_ref())
                .await?
                .collect_into()?;

        if let Some(id) = id_s.first()
            && let Some(mut event) =
                PgStoredBudgetEvent::find_by_id(self.client.as_ref(), id.event_id).await?
        {
            event.delete(self.client.as_ref()).await?;
        }
        Ok(true)
    }

    async fn events(
        &self,
        id: <Budget as cqrs::framework::Aggregate>::Id,
    ) -> Result<Vec<StoredEvent<Budget, BudgetEvent>>, RustyError> {
        self.fetch_events(id, 0).await
    }
}
#[cfg(feature = "server")]
impl AsyncBudgetCommandsTrait for PgRuntime {
    async fn create_budget(
        &self,
        user_id: Uuid,
        budget_name: &str,
        default_budget: bool,
        month_begins_on: MonthBeginsOn,
        currency: Currency,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, Uuid::default(), |budget| {
            budget.create_budget(
                budget_name.to_string(),
                user_id,
                month_begins_on,
                default_budget,
                currency,
            )
        })
        .await
    }
    async fn add_item(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_name: String,
        item_type: BudgetingType,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.add_item(item_name.clone(), item_type)
        })
        .await
    }
    async fn add_actual(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        amount: Money,
        period_id: PeriodId,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.add_actual(item_id, period_id, amount)
        })
        .await
    }
    #[allow(clippy::too_many_arguments)]
    async fn modify_item(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        name: Option<String>,
        item_type: Option<BudgetingType>,
        tag_ids: Option<Vec<Uuid>>,
        periodicity: Option<Periodicity>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.modify_item(item_id, name, item_type, tag_ids, periodicity)
        })
        .await
    }
    async fn create_tag(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        name: String,
        periodicity: Periodicity,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.create_tag(name, periodicity)
        })
        .await
    }
    async fn modify_tag(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tag_id: Uuid,
        name: Option<String>,
        periodicity: Option<Periodicity>,
        deleted: Option<bool>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.modify_tag(tag_id, name, periodicity, deleted)
        })
        .await
    }
    async fn modify_bank_account(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        account_id: Uuid,
        account_type: Option<BankAccountType>,
        description: Option<String>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.modify_bank_account(account_id, account_type, description)
        })
        .await
    }
    async fn normalize_account_numbers(&self, user_id: Uuid, budget_id: Uuid) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, Budget::normalize_account_numbers)
            .await
    }
    async fn classify_tag(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tag_id: Uuid,
        cost_kind: CostKind,
        matching: Matching,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.classify_tag(tag_id, cost_kind, matching)
        })
        .await
    }
    async fn configure_carryover(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        from_period: Option<PeriodId>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.configure_carryover(from_period)
        })
        .await
    }
    #[allow(clippy::too_many_arguments)]
    async fn modify_actual(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        period_id: PeriodId,
        budgeted_amount: Option<Money>,
        actual_amount: Option<Money>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.modify_actual(
                actual_id,
                period_id,
                budgeted_amount,
                actual_amount,
                None,
                None,
            )
        })
        .await
    }
    #[allow(clippy::too_many_arguments)]
    async fn add_and_connect_tx(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        bank_account_number: &str,
        amount: Money,
        balance: Money,
        description: &str,
        date: DateTime<Utc>,
    ) -> Result<Uuid, RustyError> {
        let tx_id = self
            .add_transaction(
                user_id,
                budget_id,
                bank_account_number,
                amount,
                balance,
                description,
                date,
            )
            .await?;
        self.connect_transaction(user_id, budget_id, tx_id, actual_id)
            .await
    }
    #[allow(clippy::too_many_arguments)]
    async fn add_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        bank_account_number: &str,
        amount: Money,
        balance: Money,
        description: &str,
        date: DateTime<Utc>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.add_transaction(
                bank_account_number.to_string(),
                amount,
                balance,
                description.to_string(),
                date,
            )
        })
        .await
    }
    async fn connect_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
        actual_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        let (amount, existing_allocations) = {
            let budget = self.load(budget_id).await?;
            let amount = budget
                .get_transaction(tx_id)
                .map(|tx| tx.amount)
                .ok_or_else(|| {
                    RustyError::ItemNotFound(tx_id.to_string(), "Transaction not found".to_string())
                })?;
            let existing = budget
                .allocations_for_transaction(tx_id)
                .iter()
                .map(|a| (a.id, a.transaction_id))
                .collect::<Vec<_>>();
            (amount, existing)
        };
        for (alloc_id, transaction_id) in existing_allocations {
            self.delete_allocation(user_id, budget_id, alloc_id, transaction_id)
                .await?;
        }
        self.create_allocation(user_id, budget_id, tx_id, actual_id, amount, String::new())
            .await
    }
    async fn ensure_account(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        account_number: &str,
        description: &str,
    ) -> Result<Uuid, RustyError> {
        let budget = self.load(budget_id).await?;
        if let Some(existing) = budget.get_account(account_number) {
            return Ok(existing.id);
        }
        self.cmd(user_id, budget_id, |budget| {
            budget.create_bank_account(account_number.to_string(), description.to_string())
        })
        .await
    }
    async fn ignore_transaction(
        &self,
        budget_id: Uuid,
        tx_id: Uuid,
        user_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.ignore_transaction(tx_id)
        })
        .await
    }
    async fn reallocate_budgeted_funds(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        period_id: PeriodId,
        from_actual_id: Uuid,
        to_actual_id: Uuid,
        amount: Money,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.reallocate_budgeted_funds(period_id, from_actual_id, to_actual_id, amount)
        })
        .await
    }
    async fn adjust_budgeted_amount(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        period_id: PeriodId,
        budgeted_amount: Money,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.adjust_actual_budgeted_funds(actual_id, period_id, budgeted_amount)
        })
        .await
    }
    async fn add_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        transaction_key: Vec<String>,
        item_key: Vec<String>,
        always_apply: bool,
        tag_id: Option<Uuid>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.add_rule(transaction_key, item_key, always_apply, tag_id)
        })
        .await
    }
    async fn tag_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
        tag_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.do_transaction_tagged(tx_id, tag_id)
        })
        .await
    }
    async fn untag_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.do_transaction_untagged(tx_id)
        })
        .await
    }
    async fn reject_transfer_pair(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        outgoing_tx_id: Uuid,
        incoming_tx_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.reject_transfer_pair(outgoing_tx_id, incoming_tx_id)
        })
        .await
    }
    async fn modify_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        rule_id: Uuid,
        transaction_key: Vec<String>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.modify_rule(rule_id, transaction_key)
        })
        .await
    }
    async fn delete_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        rule_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| budget.delete_rule(rule_id))
            .await
    }
    async fn set_item_buffer(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        buffer_target: Option<Money>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.set_item_buffer(item_id, buffer_target)
        })
        .await
    }
    async fn create_allocation(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        transaction_id: Uuid,
        actual_id: Uuid,
        amount: Money,
        tag: String,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.create_allocation(transaction_id, actual_id, amount, tag)
        })
        .await
    }
    async fn delete_allocation(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        allocation_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.delete_allocation(allocation_id, transaction_id)
        })
        .await
    }

    async fn user_exists(&self, email: &str) -> Result<bool, RustyError> {
        Ok(!PgUser::where_col(|u| u.email.equal(email))
            .run(self.client.as_ref())
            .await?
            .is_empty())
    }

    async fn get_default_user(&self) -> Result<User, RustyError> {
        match PgUser::where_col(|u| u.email.equal(DEFAULT_USER_EMAIL))
            .run(self.client.as_ref())
            .await
        {
            Ok(users) => {
                if users.is_empty() {
                    self.create_user(
                        "tommie",
                        DEFAULT_USER_EMAIL,
                        "Tommie",
                        "Nygren",
                        Some("0704382781".to_string()),
                        Some(
                            NaiveDate::parse_from_str("1973-05-12", "%Y-%m-%d").unwrap_or_default(),
                        ),
                    )
                    .await
                } else {
                    Ok(users.first().unwrap().into())
                }
            }
            Err(e) => {
                error!(error = %e, "Could not get default user");
                Err(RustyError::WeldsError(e))
            }
        }
    }

    async fn get_default_budget(&self, user_id: Uuid) -> Result<Budget, RustyError> {
        match PgUserBudgets::find_by_id(self.client.as_ref(), user_id).await? {
            None => {
                info!("User has no budgets");
                Err(RustyError::DefaultBudgetNotFound)
            }
            Some(b) => {
                let ub: UserBudgets = b.into();
                if let Some(budget) = ub.budgets.iter().find(|(_, default)| *default) {
                    self.load(budget.0).await
                } else {
                    info!("User has no default budget");
                    Err(RustyError::DefaultBudgetNotFound)
                }
            }
        }
    }

    async fn list_budgets(&self, user_id: Uuid) -> Result<Vec<BudgetSummary>, RustyError> {
        let Some(pg_ub) = PgUserBudgets::find_by_id(self.client.as_ref(), user_id).await? else {
            return Ok(Vec::new());
        };
        let ub: UserBudgets = pg_ub.into();
        let mut summaries = Vec::with_capacity(ub.budgets.len());
        for (budget_id, default) in ub.budgets {
            let budget = self.load(budget_id).await?;
            summaries.push(BudgetSummary {
                id: budget.id,
                name: budget.name,
                default,
            });
        }
        Ok(summaries)
    }

    async fn add_budget_to_user(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        default: bool,
    ) -> Result<Uuid, RustyError> {
        match PgUserBudgets::find_by_id(self.client.as_ref(), user_id).await {
            Ok(ub) => {
                let mut pg_ub = match ub {
                    None => {
                        let mut n_pg_ub = PgUserBudgets::new();
                        n_pg_ub.id = user_id;
                        // The `budgets` column stores just the `Vec<(Uuid, bool)>`
                        // list (see `From<PgUserBudgets> for UserBudgets`), not the
                        // whole `UserBudgets` struct — otherwise the row round-trips
                        // as a map and deserialisation fails with
                        // "invalid type: map, expected a sequence".
                        n_pg_ub.budgets = serde_json::to_value(Vec::<(Uuid, bool)>::new())
                            .expect("Could not serialize user budgets");
                        n_pg_ub
                    }
                    Some(pg_ub) => pg_ub,
                };
                let mut ub: UserBudgets = pg_ub.clone().into();
                // Drop any existing entry for this budget first — its
                // `default` flag may differ from the one being set now, and
                // a plain `contains` check would miss that and leave a
                // stale duplicate entry behind — then clear every other
                // default before adding this one back.
                ub.budgets.retain(|(id, _)| *id != budget_id);
                if default {
                    for entry in &mut ub.budgets {
                        entry.1 = false;
                    }
                }
                ub.budgets.push((budget_id, default));
                pg_ub.budgets =
                    serde_json::to_value(&ub.budgets).expect("Could not serialize user budgets");
                pg_ub.save(self.client.as_ref()).await?;
                Ok(user_id)
            }
            Err(e) => Err(RustyError::WeldsError(e)),
        }
    }

    async fn create_user(
        &self,
        user_name: &str,
        email: &str,
        first_name: &str,
        last_name: &str,
        phone: Option<String>,
        birthday: Option<NaiveDate>,
    ) -> Result<User, RustyError> {
        let mut pg_user: DbState<PgUser> = User::new(user_name, email, first_name, last_name, phone, birthday).into();
        pg_user.save(self.client.as_ref()).await?;
        Ok(pg_user.into())
    }
}

#[cfg(feature = "server")]
mod event_id_model {
    #![allow(clippy::unused_async_trait_impl)]
    use super::{Uuid, WeldsModel};

    #[derive(Debug, WeldsModel)]
    pub struct EventId {
        pub event_id: Uuid,
    }
}
#[cfg(feature = "server")]
pub use event_id_model::EventId;
