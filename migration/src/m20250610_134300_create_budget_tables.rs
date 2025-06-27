use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        
        //Budget
        manager.create_table(
            Table::create()
                .table(Budget::Table)
                .if_not_exists()
                .col(ColumnDef::new(Budget::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(Budget::Name).string().not_null())
                .col(ColumnDef::new(Budget::UserId).uuid().not_null())
                .col(ColumnDef::new(Budget::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                .foreign_key(
                    ForeignKey::create()
                        .from(Budget::Table, Budget::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;
        
        //BudgetYear
        manager.create_table(
            Table::create()
                .table(BudgetYear::Table)
                .if_not_exists()
                .col(ColumnDef::new(BudgetYear::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(BudgetYear::Year).integer().not_null())
                .col(ColumnDef::new(BudgetYear::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                .foreign_key(
                    ForeignKey::create()
                        .from(BudgetYear::Table, BudgetYear::BudgetId)
                        .to(Budget::Table, Budget::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;
        
        //BudgetMonth
        manager.create_table(
            Table::create()
                .table(BudgetMonth::Table)
                .if_not_exists()
                .col(ColumnDef::new(BudgetMonth::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(BudgetMonth::Month).integer().not_null())
                .col(ColumnDef::new(BudgetMonth::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                .foreign_key(
                    ForeignKey::create()
                        .from(BudgetMonth::Table, BudgetMonth::BudgetYearId)
                        .to(BudgetYear::Table, BudgetYear::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;
        
        //BudgetItem
        manager.create_table(
            Table::create()
                .table(BudgetItem::Table)
                .if_not_exists()
                .col(ColumnDef::new(BudgetItem::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(BudgetItem::BudgetCategoryId).integer().not_null())
                .col(ColumnDef::new(BudgetItem::BudgetMonthId).integer().not_null())
                .col(ColumnDef::new(BudgetItem::BudgetYearId).integer().not_null())
                .col(ColumnDef::new(BudgetItem::Amount).integer().not_null())
                .col(ColumnDef::new(BudgetItem::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                .foreign_key(
                    ForeignKey::create()
                        .from(BudgetItem::Table, BudgetItem::BudgetCategoryId)
                        .to(BudgetCategory::Table, BudgetCategory::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(BudgetItem::Table, BudgetItem::BudgetMonthId)
                        .to(BudgetMonth::Table, BudgetMonth::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(BudgetItem::Table, BudgetItem::BudgetYearId)
                        .to(BudgetYear::Table, BudgetYear::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;
        
        //BudgetCategory
        manager.create_table(
            Table::create()
                .table(BudgetCategory::Table)
                .if_not_exists()
                .col(ColumnDef::new(BudgetCategory::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(BudgetCategory::Name).string().not_null())
                .col(ColumnDef::new(BudgetCategory::IsIncome).boolean().not_null())
                .col(ColumnDef::new(BudgetCategory::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(BudgetMonth::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(BudgetYear::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(BudgetItem::Table).to_owned()).await?;
        // manager.drop_table(Table::drop().table(User::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(Iden)]
enum BudgetCategory {
    Table,
    Id,
    Name,
    IsIncome,
    CreatedAt,
}

#[derive(Iden)]
enum BudgetItem {
    Table,
    Id,
    BudgetCategoryId,
    BudgetMonthId,
    BudgetYearId,
    Amount,
    CreatedAt,
}

#[derive(Iden)]
enum Budget {
    Table,
    Id,
    UserId,
    Name,
    CreatedAt,
}


#[derive(Iden)]
enum BudgetYear {
    Table,
    Id,
    BudgetId,
    UserId,
    Year,
    CreatedAt,
}

#[derive(Iden)]
enum BudgetMonth {
    Table,
    Id,
    UserId,
    BudgetYearId,
    Month,
    CreatedAt,
}

#[derive(DeriveIden, Copy, Clone)]
enum Users {
    Table,
    Id,
}
