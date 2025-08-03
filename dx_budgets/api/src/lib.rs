//! This crate contains all shared fullstack server functions.
pub mod models;

use crate::models::budget::*;
use crate::models::user::User;
use chrono::NaiveDate;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use joydb::adapters::{JsonAdapter, RonAdapter};
use joydb::Joydb;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const DEFAULT_USER_EMAIL: &str = "tommie.nygren@gmail.com";
// Define the state
joydb::state! {
    AppState,
    models: [User, Budget, BudgetItem, BudgetTransaction, BankTransaction],
}

// Define the database (combination of state and adapter)
#[cfg(feature = "server")]
type Db = Joydb<AppState, RonAdapter>;
#[cfg(feature = "server")]
pub mod db {
    use crate::models::budget::*;
    use crate::models::user::User;
    use crate::{BudgetItemView, BudgetOverview, Db, DEFAULT_USER_EMAIL};
    use chrono::NaiveDate;
    use dioxus::fullstack::once_cell::sync::Lazy;
    use dioxus::logger::tracing;
    use dioxus::prelude::{Signal, UnsyncStorage};
    use joydb::{JoydbConfig, JoydbError};
    use uuid::Uuid;
    use Default;

    pub static CLIENT: Lazy<Db> = Lazy::new(|| {
        tracing::info!("Init DB Client");
        let client = Db::open("./data.ron").unwrap();
        // Run migrations
        tracing::info!("Insert Default Data");
        match get_default_user(Some(&client)) {
            Ok(user) => {
                tracing::info!("Default user exists");
                match get_default_budget_for_user(user.id, Some(&client)) {
                    Ok(_) => {
                        tracing::info!("Default budget exists");
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Could not get default budget for user");
                        panic!("Could not get default budget for user");
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not get default user");
                panic!("Could not get default user");
            }
        }
        client
    });

    fn client_from_option(client: Option<&Db>) -> &Db {
        if let Some(c) = client {
            c
        } else {
            &CLIENT
        }
    }

    pub fn get_budget_overview(id: Uuid, client: Option<&Db>) -> anyhow::Result<BudgetOverview> {
        if let Some(budget) = client_from_option(client).get::<Budget>(&id)? {
            let budget_items =
                client_from_option(client).get_all_by(|bi: &BudgetItem| bi.budget_id == id)?;
            let budget_items_view = budget_items
                .iter()
                .map(|bi| {
                    let incoming_budget_transactions = client_from_option(client)
                        .get_all_by(|bt: &BudgetTransaction| bt.to_budget_item == bi.id)
                        .unwrap();
                    let outgoing_budget_transactions = client_from_option(client)
                        .get_all_by(|bt: &BudgetTransaction| bt.from_budget_item == Some(bi.id))
                        .unwrap();

                    let aggregate_amount = incoming_budget_transactions
                        .iter()
                        .map(|bt| bt.amount)
                        .sum::<f32>()
                        - outgoing_budget_transactions
                            .iter()
                            .map(|bt| bt.amount)
                            .sum::<f32>();

                    let is_balanced = aggregate_amount == 0.0;
                    let money_needs_job = aggregate_amount > 0.0;
                    let too_much_job = aggregate_amount < 0.0;

                    BudgetItemView {
                        id: bi.id,
                        name: bi.budget_category.to_string(),
                        item_type: bi.budget_category.to_string(),
                        aggregate_amount,
                        is_balanced,
                        money_needs_job,
                        expected_at: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                        too_much_job,
                        incoming_budget_transactions,
                        outgoing_budget_transactions,
                    }
                })
                .collect::<Vec<BudgetItemView>>();

            Ok(BudgetOverview {
                id,
                default_budget: budget.default_budget,
                name: budget.name,
                incomes: budget_items_view
                    .iter()
                    .filter(|bi| bi.item_type == "income")
                    .cloned()
                    .collect(),
                expenses: budget_items_view
                    .iter()
                    .filter(|bi| bi.item_type == "expense")
                    .cloned()
                    .collect(),
                savings: budget_items_view
                    .iter()
                    .filter(|bi| bi.item_type == "savings")
                    .cloned()
                    .collect(),
            })
        } else {
            Err(anyhow::Error::from(JoydbError::NotFound {
                id: id.to_string(),
                model: "Budget".to_string(),
            }))
        }
    }

    pub fn list_users(client: Option<&Db>) -> anyhow::Result<Vec<User>> {
        match client_from_option(client).get_all::<User>() {
            Ok(users) => Ok(users),
            Err(e) => {
                tracing::error!(error = %e, "Could not list users");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn user_exists(email: &str, client: Option<&Db>) -> anyhow::Result<bool> {
        match client_from_option(client).get_all_by(|u: &User| u.email == email) {
            Ok(users) => Ok(!users.is_empty()),
            Err(e) => {
                tracing::error!(error = %e, "Could not get default user");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn get_default_user(client: Option<&Db>) -> anyhow::Result<User> {
        match client_from_option(client).get_all_by(|u: &User| u.email == DEFAULT_USER_EMAIL) {
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

    pub fn save_budget(budget: Budget) -> anyhow::Result<()> {
        match client_from_option(None).update(&budget) {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(error = %e, "Could not save budget");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn add_budget_transaction(
        text: &str,
        from_budget_item: Option<Uuid>,
        to_budget_item: Uuid,
        amount: f32,
    ) -> anyhow::Result<()> {
        let user = get_default_user(None)?;
        let budget_transaction_to_save = BudgetTransaction::new(
            text,
            BudgetTransactionType::default(),
            amount,
            from_budget_item,
            to_budget_item,
            user.id,
        );
        match client_from_option(None).insert(&budget_transaction_to_save) {
            Ok(_) => {
                tracing::info!("Saved budget transaction");
                Ok(())
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not save budget transaction");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn add_budget_item(
        budget_id: Uuid,
        name: String,
        first_item: &str,
        amount: f32,
        expected_at: NaiveDate,
    ) -> anyhow::Result<()> {
        let user = get_default_user(None)?;
        let budget_item_to_save = BudgetItem::new(
            budget_id,
            &name,
            &BudgetCategory::Expense(name.clone()),
            user.id,
        );
        match client_from_option(None).insert(&budget_item_to_save) {
            Ok(_) => {
                tracing::info!("Saved budget item");
                add_budget_transaction(first_item, None, budget_item_to_save.id, amount)
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not save budget item");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn get_default_budget_for_user(
        user_id: Uuid,
        client: Option<&Db>,
    ) -> anyhow::Result<Budget> {
        match client_from_option(client)
            .get_all_by(|b: &Budget| b.user_id == user_id && b.default_budget)
        {
            Ok(mut budgets) => {
                if budgets.is_empty() {
                    tracing::info!("No default budget exists, time to create one");
                    let _ = create_test_budget(user_id, client);
                    create_budget("Default", user_id, true, client)
                } else {
                    Ok(budgets.remove(0))
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not get default budget for user");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn create_test_budget(user_id: Uuid, client: Option<&Db>) -> anyhow::Result<Budget> {
        let mut budget = Budget::new("Default test budget", true, user_id);
        //Create budget items
        let category = budget.new_income_category("Löner");
        //Incomes
        let mut salary_one = BudgetItem::new(
            budget.id,
            "Lön Tommie",
            &category,
            user_id,
        );
        
        let bt = BudgetTransaction::new(
            "Lön Tommie",
            BudgetTransactionType::default(),
            39500.0,
            None,
            salary_one.id,
            user_id,
        );
        
        salary_one.store_incoming_transaction(&bt);
        budget.store_budget_item(&salary_one);
        
        let mut salary_two = BudgetItem::new(
            budget.id,
            "Lön Lisa",
            &category,
            user_id,
        );
        
        let bt = BudgetTransaction::new(
            "Lön Lisa",
            BudgetTransactionType::default(),
            19500.0,
            None,
            salary_two.id,
            user_id,
        );
        salary_two.store_incoming_transaction(&bt);
        budget.store_budget_item(&salary_two);
        
        let category = budget.new_expense_category("Fasta Utgifter");
        
        let mut mortgage = BudgetItem::new(
            budget.id,
            "Huslånet",
            &category,
            user_id,
        );
        
        budget.spend_money_on(&mut mortgage, 5660.0);
        budget.store_budget_item(&mortgage);

        let mut utgift = BudgetItem::new(
            budget.id,
            "Hyra lägenheten",
            &category,
            user_id,
        );

        budget.spend_money_on(&mut utgift, 7500.0);
        budget.store_budget_item(&utgift);
        //Expenses

        //Savings

        match client_from_option(client).insert(&budget) {
            Ok(_) => Ok(budget.clone()),
            Err(e) => {
                tracing::error!(error = %e, "Could not create test budget");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn create_budget(
        name: &str,
        user_id: Uuid,
        default_budget: bool,
        client: Option<&Db>,
    ) -> anyhow::Result<Budget> {
        let mut budget = Budget::new(name, default_budget, user_id);
        match client_from_option(client).insert(&budget) {
            Ok(_) => Ok(budget.clone()),
            Err(e) => {
                tracing::error!(error = %e, "Could not create budget");
                Err(anyhow::Error::from(e))
            }
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
        match client_from_option(client).insert(&user) {
            Ok(_) => Ok(user),
            Err(e) => {
                tracing::error!(error = %e, "Could not create user");
                Err(anyhow::Error::from(e))
            }
        }
    }
}

/// Echo the user input on the server.
#[server]
pub async fn list_users() -> Result<Vec<User>, ServerFnError> {
    match db::list_users(None) {
        Ok(users) => Ok(users),
        Err(e) => {
            tracing::error!(error = %e, "Could not list users");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn get_default_budget() -> Result<Budget, ServerFnError> {
    match db::get_default_budget_for_user(db::get_default_user(None).unwrap().id, None) {
        Ok(budget) => Ok(budget),
        Err(e) => {
            tracing::error!(error = %e, "Could not get default budget");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn save_budget(budget: Budget) -> Result<(), ServerFnError> {
    match db::save_budget(budget) {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::error!(error = %e, "Could not get default budget");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn add_budget_item(
    budget_id: Uuid,
    name: String,
    item_type: String,
    first_item: String,
    amount: f32,
    expected_at: NaiveDate,
) -> Result<(), ServerFnError> {
    tracing::info!(
        "add_budget_item: {}, {}, {}, {}",
        budget_id,
        name,
        first_item,
        amount
    );
    match db::add_budget_item(budget_id, name, &first_item, amount, expected_at) {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::error!(error = %e, "Could not save new budget item");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BudgetItemView {
    pub id: Uuid,
    pub name: String,
    pub item_type: String,
    pub aggregate_amount: f32,
    pub is_balanced: bool,
    pub money_needs_job: bool,
    pub too_much_job: bool,
    pub expected_at: NaiveDate,
    pub incoming_budget_transactions: Vec<BudgetTransaction>,
    pub outgoing_budget_transactions: Vec<BudgetTransaction>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BudgetOverview {
    pub id: Uuid,
    pub name: String,
    pub default_budget: bool,
    pub incomes: Vec<BudgetItemView>,
    pub expenses: Vec<BudgetItemView>,
    pub savings: Vec<BudgetItemView>,
}

#[server]
pub async fn get_budget_overview(id: Uuid) -> Result<BudgetOverview, ServerFnError> {
    match db::get_budget_overview(id, None) {
        Ok(overview) => Ok(overview),
        Err(e) => {
            tracing::error!(error = %e, "Could not get budget overview");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}
