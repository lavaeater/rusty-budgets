use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(Table::alter()
                .table(BankTransactions::Table)
                .add_column(ColumnDef::new(BankTransactions::Message).string()).to_owned())
            
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(Table::alter().table(BankTransactions::Table).drop_column(BankTransactions::Message).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum BankTransactions {
    Table,
    Message
}
