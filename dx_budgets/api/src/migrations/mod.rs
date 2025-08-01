
use welds::errors::Result;

use welds::migrations::prelude::*;

mod m20250701150318_create_table_users;

mod m20250710145549_create_table_budgets;

mod m20250710145616_create_table_budget_items;

mod m20250721110153_create_table_budget_transactions;

mod m20250726123221_create_table_add_item_type_to_budget_items;


pub async fn up(client: &dyn welds::TransactStart) -> Result<()> {
    let list: Vec<MigrationFn> = vec![
        m20250701150318_create_table_users::step,
        m20250710145549_create_table_budgets::step,
        m20250710145616_create_table_budget_items::step,
        m20250721110153_create_table_budget_transactions::step,
        m20250726123221_create_table_add_item_type_to_budget_items::step,
    ];
    welds::migrations::up(client, list.as_slice()).await?;
    Ok(())
}

