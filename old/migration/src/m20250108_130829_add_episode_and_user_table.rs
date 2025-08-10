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
                    .table(Users::Table)
                    .if_not_exists()
                    .col(pk_uuid(Users::Id))
                    .col(string(Users::Email))
                    .col(string(Users::Name))
                    .col(string(Users::Role).default("user"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(foreign_key_auto(
                &mut Table::create()
                    .table(Episodes::Table)
                    .if_not_exists()
                    .col(pk_uuid(Episodes::Id))
                    .col(string(Episodes::Title))
                    .col(string(Episodes::Summary))
                    .col(string(Episodes::Tags))
                    .col(string_null(Episodes::Url))
                    .to_owned(),
                Episodes::Table,
                Episodes::UserId,
                Users::Table,
                Users::Id,
                true,
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Episodes::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden, Copy, Clone)]
enum Episodes {
    Table,
    Id,
    UserId,
    Title,
    Summary,
    Tags,
    Url,
}
impl Display for Episodes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Episodes::Table => write!(f, "episodes"),
            Episodes::Id => write!(f, "episode_id"),
            Episodes::UserId => write!(f, "episode_user_id"),
            Episodes::Title => write!(f, "episode_title"),
            Episodes::Summary => write!(f, "episode_summary"),
            Episodes::Tags => write!(f, "episode_tags"),
            Episodes::Url => write!(f, "episode_url"),
        }
    }
}

#[derive(DeriveIden, Copy, Clone)]
enum Users {
    Table,
    Id,
    Email,
    Name,
    Role,
}

impl Display for Users {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Users::Table => write!(f, "users"),
            Users::Id => write!(f, "user_id"),
            Users::Name => write!(f, "user_name"),
            Users::Email => write!(f, "user_email"),
            Users::Role => write!(f, "user_role"),
        }
    }
}
