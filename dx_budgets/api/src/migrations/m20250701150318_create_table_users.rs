#[cfg(feature = "server")]
use welds::errors::Result;
#[cfg(feature = "server")]
use welds::migrations::prelude::*;

#[cfg(feature = "server")]
pub(super) fn step(_state: &TableState) -> Result<MigrationStep> {
    let m = create_table("users")
        .id(|c| c("id", Type::Uuid))
        .column(|c| c("user_name", Type::String))
        .column(|c| c("email", Type::String).create_unique_index())
        .column(|c| c("first_name", Type::String))
        .column(|c| c("last_name", Type::String))
        .column(|c| c("phone", Type::String).is_null())
        .column(|c| c("birthday", Type::Date).is_null());
    Ok(MigrationStep::new("m20250701150318_create_table_users", m))
}
