use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // users
        // manager.create_table(
        //     Table::create()
        //         .table(User::Table)
        //         .if_not_exists()
        //         .col(ColumnDef::new(User::Id).integer().not_null().auto_increment().primary_key())
        //         .col(ColumnDef::new(User::Email).string().not_null().unique_key())
        //         .col(ColumnDef::new(User::Name).string().not_null())
        //         .col(ColumnDef::new(User::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
        //         .to_owned()
        // ).await?;

        // budget_item
        manager.create_table(
            Table::create()
                .table(BudgetItem::Table)
                .if_not_exists()
                .col(ColumnDef::new(BudgetItem::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(BudgetItem::UserId).uuid().not_null())
                .col(ColumnDef::new(BudgetItem::Name).string().not_null())
                .col(ColumnDef::new(BudgetItem::IsIncome).boolean().not_null().default(false))
                .col(ColumnDef::new(BudgetItem::IsActive).boolean().not_null().default(true))
                .col(ColumnDef::new(BudgetItem::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                .foreign_key(
                    ForeignKey::create()
                        .from(BudgetItem::Table, BudgetItem::UserId)
                        .to(User::Table, User::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;

        // budget_plan
        manager.create_table(
            Table::create()
                .table(BudgetPlan::Table)
                .if_not_exists()
                .col(ColumnDef::new(BudgetPlan::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(BudgetPlan::UserId).uuid().not_null())
                .col(ColumnDef::new(BudgetPlan::Year).integer().not_null())
                .col(ColumnDef::new(BudgetPlan::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                .foreign_key(
                    ForeignKey::create()
                        .from(BudgetPlan::Table, BudgetPlan::UserId)
                        .to(User::Table, User::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;

        // budget_plan_item
        manager.create_table(
            Table::create()
                .table(BudgetPlanItem::Table)
                .if_not_exists()
                .col(ColumnDef::new(BudgetPlanItem::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(BudgetPlanItem::BudgetPlanId).integer().not_null())
                .col(ColumnDef::new(BudgetPlanItem::BudgetItemId).integer().not_null())
                .col(ColumnDef::new(BudgetPlanItem::Month).integer().not_null())
                .col(ColumnDef::new(BudgetPlanItem::PlannedAmount).decimal().not_null())
                .col(ColumnDef::new(BudgetPlanItem::Note).string())
                .foreign_key(
                    ForeignKey::create()
                        .from(BudgetPlanItem::Table, BudgetPlanItem::BudgetPlanId)
                        .to(BudgetPlan::Table, BudgetPlan::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(BudgetPlanItem::Table, BudgetPlanItem::BudgetItemId)
                        .to(BudgetItem::Table, BudgetItem::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(BudgetPlanItem::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(BudgetPlan::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(BudgetItem::Table).to_owned()).await?;
        // manager.drop_table(Table::drop().table(User::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}

#[derive(Iden)]
enum BudgetItem {
    Table,
    Id,
    UserId,
    Name,
    IsIncome,
    IsActive,
    CreatedAt,
}

#[derive(Iden)]
enum BudgetPlan {
    Table,
    Id,
    UserId,
    Year,
    CreatedAt,
}

#[derive(Iden)]
enum BudgetPlanItem {
    Table,
    Id,
    BudgetPlanId,
    BudgetItemId,
    Month,
    PlannedAmount,
    Note,
}
