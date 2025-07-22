use welds::errors::Result;
use welds::migrations::prelude::*;

pub(super) fn step(_state: &TableState) -> Result<MigrationStep> {
    let m = create_table("add_transactions")
        .id(|c| c("id", Type::String))
        .column(|c| c("text", Type::String));
    Ok(MigrationStep::new(
        "m20250721110153_create_table_add_transactions",
        m,
    ))
}
