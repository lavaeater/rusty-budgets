use crate::foreign_key_auto;
use sea_orm_migration::{prelude::*, schema::*};
use std::fmt;
use std::fmt::Display;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Imports::Table)
                    .if_not_exists()
                    .col(pk_auto(Imports::Id))
                    .col(string(Imports::Title))
                    .col(string(Imports::Text))
                    .col(binary(Imports::Data))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                foreign_key_auto(
                    Table::create()
                        .table(ImportRows::Table)
                        .if_not_exists()
                        .col(pk_uuid(ImportRows::Id))
                        .col(string(ImportRows::Data))
                        .col(string(ImportRows::Hash)),
                    ImportRows::Table,
                    ImportRows::ImportId,
                    Imports::Table,
                    Imports::Id,
                    true,
                )
                .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Imports::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden, Copy, Clone, Debug, Hash)]
enum Imports {
    Table,
    Id,
    Title,
    Text,
    Data,
}

impl Display for Imports {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Imports::Table => write!(f, "import_table"),
            Imports::Id => write!(f, "import_id"),
            Imports::Title => write!(f, "import_title"),
            Imports::Text => write!(f, "import_text"),
            Imports::Data => write!(f, "import_data"),
        }
    }
}

#[derive(DeriveIden, Copy, Clone, Debug, Hash)]
enum ImportRows {
    Table,
    Id,
    ImportId,
    Data,
    Hash,
}

impl Display for ImportRows {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportRows::Table => write!(f, "import_rows"),
            ImportRows::Id => write!(f, "import_row_id"),
            ImportRows::ImportId => write!(f, "import_row_import_id"),
            ImportRows::Data => write!(f, "import_row_data"),
            ImportRows::Hash => {
                write!(f, "import_row_hash")
            }
        }
    }
}
