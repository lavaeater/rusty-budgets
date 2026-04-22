use crate::api_error::RustyError;
use crate::cqrs::framework::{AsyncRuntime, CommandError, Runtime, StoredEvent};
use crate::db::{DEFAULT_USER_EMAIL, create_user};
use crate::models::*;
use crate::pg_models::{PgBudget, PgStoredBudgetEvent, PgUser, PgUserBudgets};
use crate::{cqrs, models};
use chrono::{DateTime, NaiveDate, Utc};
use dioxus::logger::tracing;
use dioxus::prelude::{error, info};
use joydb::Model as JoyModel;
use joydb::adapters::{FromPath, JsonAdapter};
use joydb::{Joydb, JoydbConfig, JoydbMode, SyncPolicy};
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use sqlx::Any;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;
use welds::Client;
use welds::connections::any::AnyClient;
use welds::{Syntax, WeldsError, prelude::*};

fn get_data_file() -> PathBuf {
    env::var("DATA_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            info!("DATA_FILE not set, using default data.json");
            PathBuf::from("data.json")
        })
}

pub async fn migrate_to_postgres() -> Result<(), RustyError> {
    let jr = JoyDbBudgetRuntime::new(get_data_file());
    let pr = create_runtime().await;
    /*
    StoredBudgetEvent, Budget, User, UserBudgets
     */

    let users = jr.db.get_all::<User>()?;
    for user in users {
        let mut pg_user: DbState<PgUser> = user.into();
        pg_user.save(pr.client.as_ref()).await?;
    }

    let events = jr.db.get_all::<StoredBudgetEvent>()?;
    for event in events {
        let mut pg_event: DbState<PgStoredBudgetEvent> = event.into();
        pg_event.save(pr.client.as_ref()).await?;
    }

    let budgets = jr.db.get_all::<Budget>()?;
    for budget in budgets {
        let mut pg_budget: DbState<PgBudget> = budget.into();
        pg_budget.save(pr.client.as_ref()).await?;
    }

    let user_budgets = jr.db.get_all::<UserBudgets>()?;
    for budget in user_budgets {
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
            budget.add_item(item_name.to_string(), item_type)
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
            Some(b) => match b.budgets.iter().find(|(_, default)| *default) {
                Some((budget_id, _)) => Ok(self.load(*budget_id)?),
                None => {
                    info!("User had budgets but none were default");
                    Err(RustyError::DefaultBudgetNotFound)
                }
            },
        }
    }

    fn add_budget_to_user(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        default: bool,
    ) -> Result<Uuid, RustyError> {
        let user_budgets = self.db.get::<UserBudgets>(&user_id)?;
        match user_budgets {
            None => {
                match self
                    .db
                    .insert(&UserBudgets {
                        id: user_id,
                        budgets: vec![(budget_id, default)],
                    })
                    .map(|_| user_id)
                {
                    Ok(_) => Ok(user_id),
                    Err(e) => Err(RustyError::JoydbError(e)),
                }
            }
            Some(list) => {
                if !list.budgets.contains(&(budget_id, default)) {
                    let mut budgets = list.budgets.clone();
                    budgets.push((budget_id, default));
                    let list = UserBudgets {
                        id: user_id,
                        budgets,
                    };
                    match self.db.upsert(&list) {
                        Ok(_) => Ok(user_id),
                        Err(e) => Err(RustyError::JoydbError(e)),
                    }
                } else {
                    Ok(user_id)
                }
            }
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

pub struct PgRuntime {
    client: Box<dyn Client>,
}

pub async fn create_runtime() -> PgRuntime {
    let url = env::var("DATABASE_URL").unwrap();
    let client = welds::connections::connect(url).await.unwrap();
    PgRuntime::new(client)
}

impl PgRuntime {
    pub fn new(client: AnyClient) -> Self {
        Self {
            client: Box::new(client),
        }
    }

    async fn cmd<F, E>(&self, user_id: Uuid, id: Uuid, command: F) -> Result<Uuid, RustyError>
    where
        F: FnOnce(&Budget) -> Result<E, CommandError>,
        E: Into<BudgetEvent>,
    {
        self.execute(user_id, id, |aggregate| {
            command(aggregate).map(|event| event.into())
        })
        .await
    }
}

impl JoyDbBudgetRuntime {
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

    pub fn new_in_memory() -> Self {
        Self {
            db: Db::new_in_memory().unwrap(),
        }
    }

    /// Ergonomic command execution - eliminates all the boilerplate!
    /// Usage: rt.cmd(id, |budget| budget.create_budget(name, user_id, default))
    fn cmd<F, E>(&self, user_id: Uuid, id: Uuid, command: F) -> Result<Uuid, RustyError>
    where
        F: FnOnce(&Budget) -> Result<E, CommandError>,
        E: Into<BudgetEvent>,
    {
        self.execute(user_id, id, |aggregate| {
            command(aggregate).map(|event| event.into())
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
        info!(
            "[perf] load: replayed {} events in {:?}",
            event_count,
            t.elapsed()
        );
        if event_count > 0 {
            self.snapshot(&budget).await?;
        }
        Ok(budget)
    }

    async fn snapshot(&self, agg: &Budget) -> Result<(), RustyError> {
        let mut pg_budget: DbState<PgBudget> =
            match PgBudget::find_by_id(self.client.as_ref(), agg.id).await? {
                None => DbState::<PgBudget>::from(agg),
                Some(mut existing) => {
                    existing.last_event = agg.last_event;
                    existing.version = agg.version;
                    existing.data = serde_json::to_value(agg).expect("Budget must be serializable");
                    existing
                }
            };
        pg_budget.save(self.client.as_ref()).await?;
        Ok(())
    }

    async fn append(&self, user_id: Uuid, ev: BudgetEvent) -> Result<(), RustyError> {
        let mut stored_event: DbState<PgStoredBudgetEvent> = StoredEvent::new(ev, user_id).into();
        stored_event.save(self.client.as_ref()).await?;
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
                .map(|ev| ev.into())
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
            budget.add_item(item_name.to_string(), item_type)
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
                        n_pg_ub.budgets = serde_json::to_value(UserBudgets {
                            id: user_id,
                            budgets: vec![],
                        })
                        .expect("Could not serialize user budgets");
                        n_pg_ub
                    }
                    Some(pg_ub) => pg_ub,
                };
                let mut ub: UserBudgets = pg_ub.clone().into();
                if !ub.budgets.contains(&(budget_id, default)) {
                    if default && let Some(budget) = ub.budgets.iter_mut().find(|(_, default)| *default)
                    {
                        budget.1 = false;
                    }
                    ub.budgets.push((budget_id, default));
                    pg_ub.budgets = serde_json::to_value(ub).expect("Could not serialize user budgets");
                    pg_ub.save(self.client.as_ref()).await?;
                }
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

/// Define a struct that we want to put the combined selected data into
/// NOTE: This struct doesn't have a table linked to it.
#[derive(Debug, WeldsModel)]
pub struct EventId {
    pub event_id: Uuid,
}
