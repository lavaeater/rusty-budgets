pub use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{integer, uuid};
use std::fmt::Display;

mod m20220120_000001_create_post_table;
mod m20241205_170802_create_member_table;
mod m20250108_130829_add_episode_and_user_table;
mod m20250405_063228_create_import;
mod m20250410_123829_create_transactions;
mod m20250410_195329_create_member_events;
mod m20250608_123514_add_message_to_transactions;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220120_000001_create_post_table::Migration),
            Box::new(m20241205_170802_create_member_table::Migration),
            Box::new(m20250108_130829_add_episode_and_user_table::Migration),
            Box::new(m20250405_063228_create_import::Migration),
            Box::new(m20250410_123829_create_transactions::Migration),
            Box::new(m20250410_195329_create_member_events::Migration),
            Box::new(m20250608_123514_add_message_to_transactions::Migration),
        ]
    }
}

/// Adds a foreign key to a table.
///
/// Adds a foreign key to a table, and if `use_uuid` is true, the foreign key is a UUID, otherwise it is an integer.
///
/// # Arguments
///
/// * `table_create_statement`: A mutable `TableCreateStatement` to add the foreign key to.
/// * `from_table`: The table this foreign key is on.
/// * `fk_column`: The column on `from_table` that is the foreign key.
/// * `to_table`: The table that `fk_column` points to.
/// * `to_id_column`: The column on `to_table` that `fk_column` points to.
/// * `use_uuid`: A boolean indicating whether the foreign key is a UUID or an integer.
///
/// # Returns
///
/// The `table_create_statement` passed in, but with the foreign key added.
pub fn foreign_key_auto<T, U>(
    table_create_statement: &mut TableCreateStatement,
    from_table: T,
    fk_column: T,
    to_table: U,
    to_id_column: U,
    use_uuid: bool,
) -> TableCreateStatement
where
    T: IntoIden + Copy + Display + 'static,
    U: IntoIden + Copy + Display + 'static,
{
    if use_uuid {
        table_create_statement.col(uuid(fk_column).not_null());
    } else {
        table_create_statement.col(integer(fk_column).not_null());
    }
    table_create_statement.foreign_key(&mut fk_auto(from_table, fk_column, to_table, to_id_column));
    table_create_statement.to_owned()
}

pub fn fk_auto<T, U>(
    from_table: T,
    fk_column: T,
    to_table: U,
    to_id_column: U,
) -> ForeignKeyCreateStatement
where
    T: IntoIden + Copy + Display + 'static,
    U: IntoIden + Copy + Display + 'static,
{
    ForeignKey::create()
        .name(format!("fk_{}_{}", from_table, to_table))
        .from(from_table, fk_column)
        .to(to_table, to_id_column)
        .on_delete(ForeignKeyAction::Cascade)
        .on_update(ForeignKeyAction::Cascade)
        .take()
}
