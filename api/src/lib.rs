//! This crate contains all shared fullstack server functions.
#![allow(unused_imports)]
#![allow(dead_code)]
pub mod cqrs;
pub mod events;
pub mod holidays;
pub mod import;
pub mod models;
pub mod time_delta;
pub mod view_models;

use crate::models::*;
use crate::view_models::BudgetItemViewModel;
use crate::view_models::BudgetViewModel;
use crate::view_models::TransactionViewModel;
use chrono::Utc;

#[cfg(feature = "server")]
use dioxus::logger::tracing;
use dioxus::prelude::*;
use models::*;
use uuid::Uuid;

#[cfg(feature = "server")]
const DEFAULT_USER_EMAIL: &str = "tommie.nygren@gmail.com";

#[cfg(feature = "server")]
pub mod db {
    use crate::cqrs::framework::{CommandError, Runtime};
    use crate::cqrs::runtime::{Db, JoyDbBudgetRuntime, UserBudgets};
    use crate::events::TransactionConnected;
    use crate::import::{import_from_path, import_from_skandia_excel};
    use crate::models::*;
    use crate::models::*;
    use crate::DEFAULT_USER_EMAIL;
    use anyhow::Error;
    use chrono::NaiveDate;
    use dioxus::logger::tracing;
    use joydb::JoydbError;
    use once_cell::sync::Lazy;
    use uuid::Uuid;

    pub static CLIENT: Lazy<JoyDbBudgetRuntime> = Lazy::new(|| {
        tracing::info!("Init DB Client");
        let client = JoyDbBudgetRuntime::new("data.json");
        // Run migrations
        tracing::info!("Insert Default Data");
        match get_default_user(Some(&client.db)) {
            Ok(_) => {
                tracing::info!("Default user exists");
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not get default user");
                panic!("Could not get default user");
            }
        }
        client
    });

    fn with_client(client: Option<&Db>) -> &Db {
        if let Some(c) = client {
            c
        } else {
            &CLIENT.db
        }
    }

    fn with_runtime(client: Option<&JoyDbBudgetRuntime>) -> &JoyDbBudgetRuntime {
        if let Some(c) = client {
            c
        } else {
            &CLIENT
        }
    }

    pub fn user_exists(email: &str, client: Option<&Db>) -> anyhow::Result<bool> {
        match with_client(client).get_all_by(|u: &User| u.email == email) {
            Ok(users) => Ok(!users.is_empty()),
            Err(e) => {
                tracing::error!(error = %e, "Could not get default user");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn get_default_user(client: Option<&Db>) -> anyhow::Result<User> {
        match with_client(client).get_all_by(|u: &User| u.email == DEFAULT_USER_EMAIL) {
            Ok(mut users) => {
                if users.is_empty() {
                    create_user(
                        "tommie",
                        DEFAULT_USER_EMAIL,
                        "Tommie",
                        "Nygren",
                        Some("0704382781".to_string()),
                        Some(
                            NaiveDate::parse_from_str("1973-05-12", "%Y-%m-%d").unwrap_or_default(),
                        ),
                        client,
                    )
                } else {
                    Ok(users.remove(0))
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not get default user");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn get_default_budget(user_id: Uuid) -> anyhow::Result<Option<Budget>> {
        match with_client(None).get::<UserBudgets>(&user_id) {
            Ok(b) => match b {
                None => {
                    tracing::info!("User has no budgets");
                    Ok(None)
                }
                Some(b) => match b.budgets.iter().find(|(_, default)| *default) {
                    Some((budget_id, _)) => match with_runtime(None).load(*budget_id) {
                        Ok(budget) => Ok(budget),
                        Err(_) => Err(anyhow::anyhow!("Could not load default budget")),
                    },
                    None => {
                        tracing::info!("User had budgets but none were default");
                        Ok(None)
                    }
                },
            },
            Err(_) => Err(anyhow::anyhow!("Could not get default budget")),
        }
    }

    pub fn get_budget(budget_id: Uuid) -> anyhow::Result<Budget> {
        match with_runtime(None).load(budget_id) {
            Ok(budget) => match budget {
                None => Err(anyhow::anyhow!("Could not load budget")),
                Some(budget) => Ok(budget),
            },
            Err(_) => Err(anyhow::anyhow!("Could not load budget")),
        }
    }

    pub fn add_budget_to_user(user_id: Uuid, budget_id: Uuid, default: bool) -> anyhow::Result<()> {
        match with_client(None).get::<UserBudgets>(&user_id) {
            Ok(list) => match list {
                None => {
                    match with_client(None).insert(&UserBudgets {
                        id: user_id,
                        budgets: vec![(budget_id, default)],
                    }) {
                        Ok(_) => Ok(()),
                        Err(_) => Err(anyhow::anyhow!("Could not add budget to user")),
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
                        match with_client(None).upsert(&list) {
                            Ok(_) => Ok(()),
                            Err(_) => Err(anyhow::anyhow!("Could not add budget to user")),
                        }
                    } else {
                        Ok(())
                    }
                }
            },
            Err(_) => Err(anyhow::anyhow!("Could not add budget to user")),
        }
    }

    pub fn create_budget(
        user_id: Uuid,
        name: &str,
        default_budget: bool,
    ) -> anyhow::Result<Budget> {
        match with_runtime(None).create_budget(user_id, name, default_budget, Currency::SEK) {
            Ok((budget, budget_id)) => {
                add_budget_to_user(user_id, budget_id, default_budget)?;
                Ok(budget)
            }
            Err(_) => Err(anyhow::anyhow!("Could not create budget")),
        }
    }

    pub fn import_transactions(
        user_id: Uuid,
        budget_id: Uuid,
        file_name: &str,
    ) -> anyhow::Result<Budget> {
        let runtime = with_runtime(None);
        let _ = import_from_path(file_name, user_id, budget_id, runtime)?;
        get_budget(budget_id)
    }

    pub fn add_item(
        user_id: Uuid,
        budget_id: Uuid,
        name: String,
        item_type: BudgetingType,
    ) -> anyhow::Result<(Budget, Uuid)> {
        with_runtime(None).add_item(user_id, budget_id, name, item_type)
    }

    pub fn add_actual(
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        budget_amount: Money,
        period_id: PeriodId,
    ) -> anyhow::Result<(Budget, Uuid)> {
        with_runtime(None).add_actual(user_id, budget_id, item_id, budget_amount, period_id)
    }

    pub fn modify_item(
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        name: Option<String>,
        item_type: Option<BudgetingType>,
    ) -> anyhow::Result<Budget> {
        with_runtime(None)
            .modify_item(user_id, budget_id, item_id, name, item_type)
            .map(|(b, _)| b)
    }

    pub fn connect_transaction(
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
        actual_id: Option<Uuid>,
        item_id: Uuid,
        period_id: PeriodId,
    ) -> anyhow::Result<Budget> {
        let actual_id = match actual_id {
            None => {
                let (_, actual_id) = with_runtime(None).add_actual(
                    user_id,
                    budget_id,
                    item_id,
                    Money::zero(Currency::default()),
                    period_id,
                )?;
                actual_id
            }
            Some(actual_id) => actual_id,
        };
        match with_runtime(None).connect_transaction(user_id, budget_id, tx_id, actual_id) {
            Ok((budget, _)) => Ok(budget),
            Err(err) => Err(err),
        }
    }

    pub fn ignore_transaction(
        budget_id: Uuid,
        user_id: Uuid,
        tx_id: Uuid,
    ) -> anyhow::Result<Budget> {
        with_runtime(None)
            .ignore_transaction(budget_id, tx_id, user_id)
            .map(|(b, _)| b)
    }

    pub fn adjust_actual_funds(
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        period_id: PeriodId,
        amount: Money,
    ) -> anyhow::Result<Budget> {
        with_runtime(None)
            .adjust_actual_funds(user_id, budget_id, actual_id, period_id, amount)
            .map(|(b, _)| b)
    }

    pub fn create_rule(
        budget: &Budget,
        user_id: Uuid,
        tx_id: Uuid,
        actual_id: Uuid,
    ) -> anyhow::Result<Budget> {
        let transaction = budget.get_transaction(tx_id).unwrap();
        let period_id = PeriodId::from_date(transaction.date, budget.month_begins_on());
        if let Some(period) = budget.get_period(period_id) {
            if let Some(item) = period.get_actual(actual_id) {
                let transaction_key = MatchRule::create_transaction_key(transaction);
                let item_key = MatchRule::create_item_key(item);
                let always_apply = true;

                with_runtime(None)
                    .add_rule(user_id, budget.id, transaction_key, item_key, always_apply)
                    .map(|(budget, _)| budget)
            } else {
                Err(anyhow::anyhow!("Actual item not found"))
            }
        } else {
            Err(anyhow::anyhow!("Period not found"))
        }
        
        
    }

    pub fn create_user(
        user_name: &str,
        email: &str,
        first_name: &str,
        last_name: &str,
        phone: Option<String>,
        birthday: Option<NaiveDate>,
        client: Option<&Db>,
    ) -> anyhow::Result<User> {
        let user = User::new(user_name, email, first_name, last_name, phone, birthday);
        match with_client(client).insert(&user) {
            Ok(_) => Ok(user),
            Err(e) => {
                tracing::error!(error = %e, "Could not create user");
                Err(anyhow::Error::from(e))
            }
        }
    }
}

#[server]
pub async fn create_budget(
    name: String,
    period_id: PeriodId,
    default_budget: Option<bool>,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user(None).expect("Could not get default user");
    match db::create_budget(user.id, &name, default_budget.unwrap_or(true)) {
        Ok(b) => Ok(BudgetViewModel::from_budget(&b, period_id)),
        Err(e) => {
            error!(error = %e, "Could not get default budget");
            Err(ServerFnError::new(
                "Could not get default budget".to_string(),
            ))
        }
    }
}

#[server]
pub async fn add_new_actual_item(
    budget_id: Uuid,
    name: String,
    item_type: BudgetingType,
    budgeted_amount: Money,
    tx_id: Option<Uuid>,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    let (_, item_id) = match db::add_item(user.id, budget_id, name, item_type) {
        Ok((b, item_id)) => (b, item_id),
        Err(e) => {
            error!(error = %e, "Could not add new item");
            return Err(ServerFnError::new(e.to_string()));
        }
    };
    info!("We have a new item with Id: {}", item_id);

    let (budget, actual_id) = match db::add_actual(user.id, budget_id, item_id, budgeted_amount, period_id) {
        Ok((b, actual_id)) => (b, actual_id),
        Err(e) => {
            error!(error = %e, "Could not add actual item");
            return Err(ServerFnError::new(e.to_string()));
        }
    };
    
    match tx_id {
        Some(tx_id) => {
            match db::connect_transaction(user.id, budget_id, tx_id, Some(actual_id), item_id, period_id) {
                Ok(b) => Ok(BudgetViewModel::from_budget(&b, period_id)),
                Err(e) => {
                    error!(error = %e, "Could not connect transaction");
                    Err(ServerFnError::new(e.to_string()))
                }
            }
        }
        None => Ok(BudgetViewModel::from_budget(&budget, period_id)),
    }
}

#[server]
pub async fn modify_item(
    budget_id: Uuid,
    item_id: Uuid,
    name: Option<String>,
    item_type: Option<BudgetingType>,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    match db::modify_item(user.id, budget_id, item_id, name, item_type) {
        Ok(b) => Ok(BudgetViewModel::from_budget(&b, period_id)),
        Err(e) => {
            error!(error = %e, "Could not modify item");
            Err(ServerFnError::new(e.to_string()))
        }
    }
}

#[server]
pub async fn get_default_user() -> Result<User, ServerFnError> {
    match db::get_default_user(None) {
        Ok(b) => Ok(b),
        Err(e) => {
            error!(error = %e, "Could not get default User");
            Err(ServerFnError::new(e.to_string()))
        }
    }
}

#[server]
pub async fn get_budget(
    budget_id: Option<Uuid>,
    period_id: PeriodId,
) -> Result<Option<BudgetViewModel>, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    if let Some(budget_id) = budget_id {
        match db::get_budget(budget_id) {
            Ok(b) => Ok(Some(BudgetViewModel::from_budget(&b, period_id))),
            Err(e) => {
                error!(error = %e, "Could not get budget");
                Err(ServerFnError::new(e.to_string()))
            }
        }
    } else {
        match db::get_default_budget(user.id) {
            Ok(b) => Ok(b.map(|b| BudgetViewModel::from_budget(&b, period_id))),
            Err(e) => {
                error!(error = %e, "Could not get default budget");
                Err(ServerFnError::new(e.to_string()))
            }
        }
    }
}

#[server]
pub async fn import_transactions(
    budget_id: Uuid,
    file_name: String,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    match db::import_transactions(user.id, budget_id, &file_name) {
        Ok(b) => Ok(BudgetViewModel::from_budget(&b, period_id)),
        Err(e) => {
            error!(error = %e, "Could not import transactions");
            Err(ServerFnError::new(e.to_string()))
        }
    }
}

#[server]
pub async fn connect_transaction(
    budget_id: Uuid,
    tx_id: Uuid,
    actual_id: Option<Uuid>,
    budget_item_id: Uuid,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    match db::connect_transaction(
        user.id,
        budget_id,
        tx_id,
        actual_id,
        budget_item_id,
        period_id,
    ) {
        Ok(b) => Ok(BudgetViewModel::from_budget(&b, period_id)),
        Err(e) => {
            error!(error = %e, "Could not connect transaction to item.");
            Err(ServerFnError::new(e.to_string()))
        }
    }
}

#[server]
pub async fn ignore_transaction(
    budget_id: Uuid,
    tx_id: Uuid,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    match db::ignore_transaction( budget_id, user.id, tx_id) {
        Ok(b) => Ok(BudgetViewModel::from_budget(&b, period_id)),
        Err(e) => {
            error!(error = %e, "Could not ignore transaction.");
            Err(ServerFnError::new(e.to_string()))
        }
    }
}

#[server]
pub async fn adjust_actual_funds(
    budget_id: Uuid,
    actual_id: Uuid,
    amount: Money,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    match db::adjust_actual_funds(user.id, budget_id, actual_id, period_id, amount) {
        Ok(b) => Ok(BudgetViewModel::from_budget(&b, period_id)),
        Err(e) => {
            error!(error = %e, "Could not adjust actual item funds");
            Err(ServerFnError::new(e.to_string()))
        }
    }
}
