use std::fmt::Display;
use crate::foreign_key_auto;
use sea_orm_migration::{prelude::*, schema::*};
use crate::m20241205_170802_create_member_table::Members;
use crate::sea_orm::{EnumIter, Iterable};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                foreign_key_auto(
                    &mut Table::create()
                        .table(MemberEvents::Table)
                        .if_not_exists()
                        .col(pk_uuid(MemberEvents::Id))
                        .col(enumeration(MemberEvents::Type, Alias::new("member_event_type"), MemberEventType::iter()))
                        .col(json(MemberEvents::Data))
                        .col(date(MemberEvents::HappenedAt))
                        .to_owned(),
                    MemberEvents::Table,
                    MemberEvents::MemberId,
                    Members::Table,
                    Members::Id,
                    true,
            ))
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MemberEvents::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden, Copy, Clone)]
enum MemberEvents {
    Table,
    Id,
    Type,
    Data,
    HappenedAt,
    MemberId,
}

impl Display for MemberEvents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberEvents::Table => write!(f, "member_events"),
            MemberEvents::Id => write!(f, "member_event_id"),
            MemberEvents::Type => write!(f, "member_event_type"),
            MemberEvents::Data => write!(f, "member_event_data"),
            MemberEvents::HappenedAt => write!(f, "member_event_happened_at"),
            MemberEvents::MemberId => write!(f, "member_event_member_id"),
        }
    }
}

#[derive(Iden, EnumIter, Copy, Clone)]
enum MemberEventType {
    #[iden = "payment"]
    Payment,
    #[iden = "other"]
    Other,
}

impl Display for MemberEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberEventType::Payment => write!(f, "payment"),
            MemberEventType::Other => write!(f, "other"),
        }
    }
}