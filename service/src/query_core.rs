use entities::prelude::User;
use entities::{
    bank_transaction, episode, episode::Entity as Episode, import, import::Entity as Import,
    member, member::Entity as Member, user,
};
use entities::{post, post::Entity as Post};
use sea_orm::prelude::Uuid;
use sea_orm::*;

pub struct QueryCore;

impl QueryCore {
    pub async fn find_member_by_id(db: &DbConn, id: Uuid) -> Result<Option<member::Model>, DbErr> {
        Member::find_by_id(id).one(db).await
    }
    pub async fn find_episodes(
        db: &DatabaseConnection,
        page: u64,
        episodes_per_page: u64,
    ) -> Result<(Vec<episode::Model>, u64), DbErr> {
        let paginator = Episode::find()
            .order_by_asc(episode::Column::Id)
            .paginate(db, episodes_per_page);
        let num_pages = paginator.num_pages().await?;

        // Fetch paginated members
        paginator.fetch_page(page - 1).await.map(|p| (p, num_pages))
    }

    /// If ok, returns (member models, num pages).
    pub async fn find_members_in_page(
        db: &DbConn,
        page: u64,
        members_per_page: u64,
    ) -> Result<(Vec<member::Model>, u64), DbErr> {
        // Setup paginator
        let paginator = Member::find()
            .order_by_asc(member::Column::Id)
            .paginate(db, members_per_page);
        let num_pages = paginator.num_pages().await?;

        // Fetch paginated members
        paginator.fetch_page(page - 1).await.map(|p| (p, num_pages))
    }
    pub async fn find_post_by_id(db: &DbConn, id: Uuid) -> Result<Option<post::Model>, DbErr> {
        Post::find_by_id(id).one(db).await
    }

    /// If ok, returns (post models, num pages).
    pub async fn find_posts_in_page(
        db: &DbConn,
        page: u64,
        posts_per_page: u64,
    ) -> Result<(Vec<post::Model>, u64), DbErr> {
        // Setup paginator
        let paginator = Post::find()
            .order_by_asc(post::Column::Id)
            .paginate(db, posts_per_page);
        let num_pages = paginator.num_pages().await?;

        // Fetch paginated posts
        paginator.fetch_page(page - 1).await.map(|p| (p, num_pages))
    }

    pub async fn find_user_by_email(
        db: &DbConn,
        email: &str,
    ) -> Result<Option<user::Model>, DbErr> {
        User::find()
            .filter(user::Column::Email.contains(email))
            .one(db)
            .await
    }

    pub async fn list_imports(db: &DbConn) -> Result<Vec<import::Model>, DbErr> {
        Import::find().all(db).await
    }

    pub async fn member_exists_by_hash(conn: &DatabaseConnection, hash: &str) -> bool {
        // Check for existing members with the same email (primary check)
        let hash_match = member::Entity::find()
            .filter(member::Column::Hash.eq(hash.to_string()))
            .one(conn)
            .await;

        match hash_match {
            Ok(m) => match m {
                Some(_) => true,
                None => false,
            },
            Err(_) => false,
        }
    }

    pub async fn bank_transaction_exists_by_hash(conn: &DatabaseConnection, hash: &str) -> bool {
        // Check for existing members with the same email (primary check)
        let hash_match = bank_transaction::Entity::find()
            .filter(bank_transaction::Column::Hash.eq(hash.to_string()))
            .one(conn)
            .await;

        match hash_match {
            Ok(m) => match m {
                Some(_) => true,
                None => false,
            },
            Err(_) => false,
        }
    }

    /// Check if a member with similar data already exists in the database
    #[allow(dead_code)]
    async fn member_exists_by_data(
        conn: &sea_orm::DatabaseConnection,
        first_name: &str,
        last_name: &str,
        email: &str,
    ) -> bool {
        // Check for existing members with the same email (primary check)
        let email_match = member::Entity::find()
            .filter(member::Column::Email.eq(email.to_string()))
            .one(conn)
            .await;

        if let Ok(Some(_)) = email_match {
            return true;
        }

        // Check for members with the same first and last name as a secondary check
        let name_match = member::Entity::find()
            .filter(member::Column::FirstName.eq(first_name.to_string()))
            .filter(member::Column::LastName.eq(last_name.to_string()))
            .one(conn)
            .await;

        matches!(name_match, Ok(Some(_)))
    }
}
