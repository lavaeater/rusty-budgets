#[cfg(feature = "server")]
use welds::errors::Result;
#[cfg(feature = "server")]
use welds::migrations::prelude::*;
#[cfg(feature = "server")]
mod m20250701150318_create_table_users;
#[cfg(feature = "server")]
mod m20250710145549_create_table_budgets;
#[cfg(feature = "server")]
mod m20250710145616_create_table_budget_items;
#[cfg(feature = "server")]
mod m20250721110153_create_table_budget_transactions;

#[cfg(feature = "server")]
pub async fn up(client: &dyn welds::TransactStart) -> Result<()> {
    let list: Vec<MigrationFn> = vec![
        m20250701150318_create_table_users::step,
        m20250710145549_create_table_budgets::step,
        m20250710145616_create_table_budget_items::step,
        m20250721110153_create_table_budget_transactions::step,
    ];
    welds::migrations::up(client, list.as_slice()).await?;
    Ok(())
}

