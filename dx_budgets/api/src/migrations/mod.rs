use welds::errors::Result;
use welds::migrations::prelude::*;
mod m20250701150318_create_table_users;

pub async fn up(client: &dyn welds::TransactStart) -> Result<()> {
    let list: Vec<MigrationFn> = vec![
        m20250701150318_create_table_users::step,
    ];
    welds::migrations::up(client, list.as_slice()).await?;
    Ok(())
}

