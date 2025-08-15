use std::fmt::Display;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .create_table(
                Table::create()
                    .table(Members::Table)
                    .if_not_exists()
                    .col(pk_uuid(Members::Id))
                    .col(string(Members::FirstName))
                    .col(string(Members::LastName))
                    .col(string_null(Members::Email))
                    .col(string_null(Members::MobilePhone))
                    .col(date_null(Members::BirthDate))
                    .col(string(Members::Hash))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .drop_table(Table::drop().table(Members::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden, Copy, Clone)]
pub enum Members {
    Table,
    Id,
    FirstName,
    LastName,
    Email,
    MobilePhone,
    BirthDate,
    Hash,
}

impl Display for Members {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Members::Table => write!(f, "members"),
            Members::Id => write!(f, "member_id"),
            Members::FirstName => write!(f, "member_first_name"),
            Members::LastName => write!(f, "member_last_name"),
            Members::Email => write!(f, "member_email"),
            Members::MobilePhone => write!(f, "member_mobile_phone"),
            Members::BirthDate => write!(f, "member_birth_date"),
            Members::Hash => write!(f, "member_hash"),
        }
    }
}
