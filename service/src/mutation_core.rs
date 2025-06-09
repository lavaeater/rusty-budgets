use entities::{bank_transaction, episode, member, member::Entity as Member, RecordHash};
use entities::{post, post::Entity as Post};

use sea_orm::prelude::Uuid;
use sea_orm::*;

pub struct MutationCore;

impl MutationCore {
    pub async fn create_episode(
        db: &DatabaseConnection,
        form_data: episode::Model,
    ) -> Result<episode::ActiveModel, DbErr> {
        form_data.into_active_model().save(db).await
    }

    pub async fn create_member(
        db: &DbConn,
        form_data: member::Model,
    ) -> Result<member::ActiveModel, DbErr> {
        member::ActiveModel {
            first_name: Set(form_data.first_name.to_owned()),
            last_name: Set(form_data.last_name.to_owned()),
            email: Set(form_data.email.to_owned()),
            mobile_phone: Set(form_data.mobile_phone.to_owned()),
            birth_date: Set(form_data.birth_date.to_owned()),
            hash: Set(form_data.hash()),
            ..Default::default()
        }
        .save(db)
        .await
    }

    pub async fn create_bank_transaction(
        db: &DbConn,
        transaction_data: bank_transaction::Model,
    ) -> Result<bank_transaction::ActiveModel, DbErr> {
        bank_transaction::ActiveModel {
            bookkeeping_date: Set(transaction_data.bookkeeping_date.to_owned()),
            transaction_text: Set(transaction_data.transaction_text.to_owned()),
            reference: Set(transaction_data.reference.to_owned()),
            other_fields: Set(transaction_data.other_fields.to_owned()),
            amount: Set(transaction_data.amount.to_owned()),
            hash: Set(transaction_data.hash()),
            ..Default::default()
        }
        .save(db)
        .await
    }

    pub async fn update_member_by_id(
        db: &DbConn,
        id: Uuid,
        form_data: member::Model,
    ) -> Result<member::Model, DbErr> {
        let member: member::ActiveModel = Member::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::Custom("Cannot find member.".to_owned()))
            .map(Into::into)?;

        member::ActiveModel {
            id: member.id,
            first_name: Set(form_data.first_name.to_owned()),
            last_name: Set(form_data.last_name.to_owned()),
            email: Set(form_data.email.to_owned()),
            mobile_phone: Set(form_data.mobile_phone.to_owned()),
            birth_date: Set(form_data.birth_date.to_owned()),
            hash: Set(form_data.hash()),
        }
        .update(db)
        .await
    }

    pub async fn delete_member(db: &DbConn, id: Uuid) -> Result<DeleteResult, DbErr> {
        let member: member::ActiveModel = Member::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::Custom("Cannot find member.".to_owned()))
            .map(Into::into)?;

        member.delete(db).await
    }

    #[allow(dead_code)]
    pub async fn delete_all_members(db: &DbConn) -> Result<DeleteResult, DbErr> {
        Member::delete_many().exec(db).await
    }

    pub async fn create_post(
        db: &DbConn,
        form_data: post::Model,
    ) -> Result<post::ActiveModel, DbErr> {
        post::ActiveModel {
            title: Set(form_data.title.to_owned()),
            text: Set(form_data.text.to_owned()),
            ..Default::default()
        }
        .save(db)
        .await
    }

    pub async fn update_post_by_id(
        db: &DbConn,
        id: Uuid,
        form_data: post::Model,
    ) -> Result<post::Model, DbErr> {
        let post: post::ActiveModel = Post::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::Custom("Cannot find post.".to_owned()))
            .map(Into::into)?;

        post::ActiveModel {
            id: post.id,
            title: Set(form_data.title.to_owned()),
            text: Set(form_data.text.to_owned()),
        }
        .update(db)
        .await
    }

    pub async fn delete_post(db: &DbConn, id: Uuid) -> Result<DeleteResult, DbErr> {
        let post: post::ActiveModel = Post::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::Custom("Cannot find post.".to_owned()))
            .map(Into::into)?;

        post.delete(db).await
    }

    #[allow(dead_code)]
    pub async fn delete_all_posts(db: &DbConn) -> Result<DeleteResult, DbErr> {
        Post::delete_many().exec(db).await
    }
}
