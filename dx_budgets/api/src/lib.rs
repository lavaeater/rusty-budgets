//! This crate contains all shared fullstack server functions.
mod migrations;
pub mod models;

use crate::models::budget::Budget;
use crate::models::budget_transaction::BudgetTransaction;
use crate::models::user::User;
use chrono::NaiveDate;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const DEFAULT_USER_EMAIL: &str = "tommie.nygren@gmail.com";

pub mod db {
    use crate::models::budget::Budget;
    use crate::models::budget_item::BudgetItem;
    use crate::models::budget_transaction::BudgetTransaction;
    use crate::models::user::User;
    use crate::{migrations, BudgetItemView, BudgetOverview, DEFAULT_USER_EMAIL};
    use dioxus::logger::tracing;
    use dioxus::prelude::{Signal, UnsyncStorage};
    use once_cell::sync::Lazy;
    use uuid::Uuid;
    use Default;
    use chrono::NaiveDate;
    use joydb::adapters::JsonAdapter;
    use joydb::{Joydb, JoydbError};

    // Define the state
    joydb::state! {
        AppState,
        models: [User, Budget, BudgetItem, BudgetTransaction],
    }

    // Define the database (combination of state and adapter)
    type Db = Joydb<AppState, JsonAdapter>;

    pub static CLIENT: Lazy<Db> = Lazy::new(|| {
        tracing::info!("Init DB Client");
        let client = Db::open("./data.json").unwrap();        
        
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

    pub async fn get_budget_overview(
        id: Uuid,
        client: Option<&Db>,
    ) -> anyhow::Result<BudgetOverview> {
        let budget: Budget = Budget::all()
            .where_col(|b| b.id.equal(id))
            .fetch_one(client_from_option(client))
            .await?
            .into_inner();

        let budget_item_set: DataSet<BudgetItem> =
            BudgetItem::where_col(|bi| bi.budget_id.equal(id))
                .where_col(|bi| bi.item_type.equal("income"))
                .include(|bi| bi.incoming_budget_transactions)
                .include(|bi| bi.outgoing_budget_transactions)
                .run(client_from_option(client))
                .await?;

        let budget_items_view = budget_item_set
            .iter()
            .map(|bi| {
                let incoming_budget_transactions =
                    bi.get_owned(|bi| bi.incoming_budget_transactions);
                let outgoing_budget_transactions =
                    bi.get_owned(|bi| bi.outgoing_budget_transactions);

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
                let too_much_job= aggregate_amount < 0.0;
                BudgetItemView {
                    id: bi.id,
                    name: bi.name.clone(),
                    item_type: bi.item_type.clone(),
                    aggregate_amount,
                    is_balanced,
                    money_needs_job,
                    too_much_job,
                    expected_at: bi.expected_at,
                    incoming_budget_transactions,
                    outgoing_budget_transactions,
                }
            })
            .collect::<Vec<_>>();

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
    }

    pub async fn list_users(client: Option<&Db>) -> anyhow::Result<Vec<User>> {
        match User::all().run(client_from_option(client)).await {
            Ok(users) => Ok(users.into_iter().map(|u| u.into_inner()).collect()),
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }

    pub async fn user_exists(email: &str, client: Option<&Db>) -> bool {
        tracing::info!("user_exists");
        if let Ok(res) = User::all()
            .where_col(|u| u.email.equal(email))
            .run(client_from_option(client))
            .await
        {
            tracing::info!("user_exists: {}", !res.is_empty());
            !res.is_empty()
        } else {
            tracing::info!("user_exists: false, an error occurred");
            false
        }
    }

    pub fn get_default_user(client: Option<&Db>) -> anyhow::Result<User> {
        match client_from_option(client).get_all_by(|u: &User| u.email == DEFAULT_USER_EMAIL) {
            Ok(users) => {
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
                    Ok(users[0].clone())
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not get default user");
                Err(anyhow::Error::from(e))
            }
        }
    }

    /***
    I am totally done with this: we need to load the budget and modify
    it OR return tracked entities to the ui, which might be cool as well.

    We'll figure it out, bro
     */

    pub async fn save_budget(budget: Budget) -> anyhow::Result<()> {
        let mut budget_to_save = DbState::db_loaded(Budget::default());
        budget_to_save.replace_inner(budget);
        match budget_to_save.save(client_from_option(None)).await {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(error = %e, "Could not save budget");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub async fn add_budget_transaction(
        text: String,
        from_budget_item: Option<Uuid>,
        to_budget_item: Uuid,
        amount: f32,
    ) -> anyhow::Result<()> {
        let user = get_default_user(None).await?;
        let mut budget_transaction_to_save =
            DbState::new_uncreated(BudgetTransaction::new_from_user(
                &text,
                amount,
                from_budget_item,
                to_budget_item,
                user.id,
            ));
        match budget_transaction_to_save
            .save(client_from_option(None))
            .await
        {
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

    pub async fn add_budget_item(
        budget_id: Uuid,
        name: String,
        item_type: String,
        first_item: String,
        amount: f32,
        expected_at: NaiveDate,
    ) -> anyhow::Result<()> {
        let user = get_default_user(None).await?;
        let mut budget_item_to_save = DbState::new_uncreated(BudgetItem::new_from_user(
            budget_id,
            &name,
            &item_type,
            expected_at,
            user.id,
        ));
        match budget_item_to_save.save(client_from_option(None)).await {
            Ok(_) => {
                tracing::info!("Saved budget item");
                add_budget_transaction(first_item, None, budget_item_to_save.id, amount).await
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
        match client_from_option(client).get_all_by(|b: &Budget| b.user_id == user_id && b.default_budget) {
            Ok(budgets) => {
                if budgets.is_empty() {
                    tracing::info!("No default budget exists, time to create one");
                    create_budget("Default", user_id, true, client)
                } else {
                    Ok(budgets[0].clone())
                } else {
                    Err(anyhow::Error::from(WeldsError::RowNotFound))
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not get default budget for user");
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
        match client_from_option(client).insert(&mut budget) {
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
        let mut user = DbState::new_uncreated(User {
            id: uuid::Uuid::new_v4(),
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            phone,
            email: email.to_string(),
            user_name: user_name.to_string(),
            birthday,
        });
        match user.save(client_from_option(client)).await {
            Ok(_) => Ok(user.into_inner()),
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
    match db::list_users(None).await {
        Ok(users) => Ok(users),
        Err(e) => {
            tracing::error!(error = %e, "Could not list users");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn get_default_budget() -> Result<Budget, ServerFnError> {
    match db::get_default_budget_for_user(db::get_default_user(None).await.unwrap().id, None).await
    {
        Ok(budget) => Ok(budget),
        Err(e) => {
            tracing::error!(error = %e, "Could not get default budget");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn save_budget(budget: Budget) -> Result<(), ServerFnError> {
    match db::save_budget(budget).await {
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
    match db::add_budget_item(budget_id, name, item_type, first_item, amount, expected_at).await {
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
    match db::get_budget_overview(id, None).await {
        Ok(overview) => Ok(overview),
        Err(e) => {
            tracing::error!(error = %e, "Could not get budget overview");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}
