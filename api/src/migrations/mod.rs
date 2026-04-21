use crate::errors::Result;
use welds::migrations::prelude::*;

pub async fn up(db: &dyn welds::TransactStart) -> Result<()> {
    let list: Vec<MigrationFn> = vec![
        /* MIGRATION LIST MARKER */
    ];
    welds::migrations::up(db, list.as_slice()).await?;
    Ok(())
}
