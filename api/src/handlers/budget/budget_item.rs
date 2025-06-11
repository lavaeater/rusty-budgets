use poem::{delete, get, handler, post, put, web::{Form, Html, Path}, IntoResponse, Route};
use poem::web::Data;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use tera::Tera;
use entities::{budget_item};
use crate::redirect;

#[handler]
pub async fn list_budget_items(db: Data<&DatabaseConnection>, tera: Data<&Tera>) -> Html<String> {
    let items = budget_item::Entity::find().all(db.0).await.unwrap_or_default();
    let mut ctx = tera::Context::new();
    ctx.insert("items", &items);
    Html(tera.render("budget/items.html", &ctx).unwrap())
}

#[handler]
pub async fn new_budget_item_form(tera: Data<&Tera>) -> Html<String> {
    Html(tera.render("budget/item_form.html", &tera::Context::new()).unwrap())
}

#[handler]
pub async fn create_budget_item(db: Data<&DatabaseConnection>, Form(form): Form<budget_item::Model>) -> poem::Result<impl IntoResponse> {
    let mut new_item = budget_item::ActiveModel {
        name: Set(form.name),
        is_income: Set(form.is_income),
        is_active: Set(true),
        user_id: Set(form.user_id),
        ..Default::default()
    };
    new_item.insert(db.0).await.ok();
    redirect("/items")
}

#[handler]
pub async fn edit_budget_item_form(db: Data<&DatabaseConnection>, tera: Data<&Tera>, Path(id): Path<i32>) -> Html<String> {
    let item = budget_item::Entity::find_by_id(id).one(db.0).await.unwrap();
    let mut ctx = tera::Context::new();
    ctx.insert("item", &item);
    Html(tera.render("budget/item_form.html", &ctx).unwrap())
}

#[handler]
pub async fn update_budget_item(db: Data<&DatabaseConnection>, Path(id): Path<i32>, Form(form): Form<budget_item::Model>) -> poem::Result<impl IntoResponse> {
    
    if let Some(mut item) = budget_item::Entity::find_by_id(id)
        .one(db.0).await.unwrap().map(Into::<budget_item::ActiveModel>::into) {
        item.name = Set(form.name);
        item.is_income = Set(form.is_income);
        item.is_active = Set(form.is_active);
        item.update(db.0).await.ok();
    }
    redirect("/items")
}

#[handler]
pub async fn delete_budget_item(db: Data<&DatabaseConnection>, Path(id): Path<i32>) -> poem::Result<impl IntoResponse> {
    budget_item::Entity::delete_by_id(id).exec(db.0).await.ok();
    redirect("/items")
}

// Similar handlers can be created for BudgetPlan and BudgetPlanItem

pub fn budget_routes() -> Route {
    Route::new()
        .at("/items", get(list_budget_items))
        .at("/items/new", get(new_budget_item_form))
        .at("/items", post(create_budget_item))
        .at("/items/:id/edit", get(edit_budget_item_form))
        .at("/items/:id", put(update_budget_item))
        .at("/items/:id", delete(delete_budget_item))
}
