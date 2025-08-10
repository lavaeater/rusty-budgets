use poem::{delete, get, handler, post, put, IntoResponse, Route};
use poem::error::InternalServerError;
use poem::web::{Data, Form, Html, Path, Query};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use tera::Tera;
use entities::{budget_item, budget_plan_item};
use service::{MutationCore, QueryCore};
use crate::{redirect, AppState, PaginationParams, DEFAULT_ITEMS_PER_PAGE};


pub fn budget_routes() -> Route {
    Route::new()
        .at("/items", get(list_budget_items).post(create_budget_item))
        .at("/items/new", get(new_budget_item_form))
        .at("/items/:id/edit", get(edit_budget_item_form))
        .at("/items/:id", delete(delete_budget_item).put(update_budget_item))
        .at("/plans", get(list_budget_plans).post(create_budget_plan))
        .at("/plans/new", get(new_budget_plan_form))
        .at("/plans/:id/edit", get(edit_budget_plan_form))
        .at("/plans/:id", put(update_budget_plan).delete(delete_budget_plan))
        // Plan items
        .at("/plans/:plan_id/items/new", get(new_plan_item_form))
        .at("/plans/:plan_id/items", post(create_plan_item))
        .at("/plans/:plan_id/items/:id/edit", get(edit_plan_item_form))
        .at("/plans/:plan_id/items/:id", delete(delete_plan_item))
}


#[handler]
pub async fn list_budget_items(db: Data<&DatabaseConnection>, tera: Data<&Tera>) -> Html<String> {
    let items = entities::budget_item::Entity::find().all(db.0).await.unwrap_or_default();
    let mut ctx = tera::Context::new();
    ctx.insert("items", &items);
    Html(tera.render("budget/items.html", &ctx).unwrap())
}

#[handler]
pub async fn new_budget_item_form(tera: Data<&Tera>) -> Html<String> {
    Html(tera.render("budget/item_form.html", &tera::Context::new()).unwrap())
}

#[handler]
pub async fn create_budget_item(db: Data<&DatabaseConnection>, Form(form): Form<entities::budget_item::Model>) -> poem::Result<impl IntoResponse> {
    let mut new_item = entities::budget_item::ActiveModel {
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
    let item = entities::budget_item::Entity::find_by_id(id).one(db.0).await.unwrap();
    let mut ctx = tera::Context::new();
    ctx.insert("item", &item);
    Html(tera.render("budget/item_form.html", &ctx).unwrap())
}

#[handler]
pub async fn update_budget_item(db: Data<&DatabaseConnection>, Path(id): Path<i32>, Form(form): Form<entities::budget_item::Model>) -> poem::Result<impl IntoResponse> {

    if let Some(mut item) = entities::budget_item::Entity::find_by_id(id)
        .one(db.0).await.unwrap().map(Into::<entities::budget_item::ActiveModel>::into) {
        item.name = Set(form.name);
        item.is_income = Set(form.is_income);
        item.is_active = Set(form.is_active);
        item.update(db.0).await.ok();
    }
    redirect("/items")
}

#[handler]
pub async fn delete_budget_item(db: Data<&DatabaseConnection>, Path(id): Path<i32>) -> poem::Result<impl IntoResponse> {
    entities::budget_item::Entity::delete_by_id(id).exec(db.0).await.ok();
    redirect("/items")
}


#[handler]
pub async fn list_budget_plans(
    state: Data<&AppState>,
    Query(params): Query<PaginationParams>,
) -> poem::Result<impl IntoResponse> {
    let conn = &state.conn;
    let page = params.page.unwrap_or(1);
    let plans_per_page = params.items_per_page.unwrap_or(DEFAULT_ITEMS_PER_PAGE);

    let (plans, num_pages) = QueryCore::list_budget_plans_at_page(conn, page, plans_per_page)
        .await
        .map_err(InternalServerError)?;

    let mut ctx = tera::Context::new();
    ctx.insert("plans", &plans);

    let mut ctx = tera::Context::new();
    ctx.insert("page", &page);
    ctx.insert("plans_per_page", &plans_per_page);
    ctx.insert("num_pages", &num_pages);

    let body = state
        .templates
        .render("budget/plans.html.tera", &ctx)
        .map_err(InternalServerError)?;
    Ok(Html(body))
}

#[handler]
pub async fn new_budget_plan_form(tera: Data<&Tera>) -> Html<String> {
    Html(
        tera.render("budget/plan_form.html.tera", &tera::Context::new())
            .unwrap(),
    )
}

#[handler]
pub async fn create_budget_plan(
    db: Data<&DatabaseConnection>,
    Form(form): Form<entities::budget_plan::Model>,
) -> poem::Result<impl IntoResponse> {
    let mut new_plan = entities::budget_plan::ActiveModel {
        user_id: Set(form.user_id),
        year: Set(form.year),
        ..Default::default()
    };
    new_plan.insert(db.0).await.ok();
    redirect("/budget/plans")
}

#[handler]
pub async fn edit_budget_plan_form(
    db: Data<&DatabaseConnection>,
    tera: Data<&Tera>,
    Path(id): Path<i32>,
) -> Html<String> {
    let plan = entities::budget_plan::Entity::find_by_id(id).one(db.0).await.unwrap();
    let mut ctx = tera::Context::new();
    ctx.insert("plan", &plan);
    Html(tera.render("budget/plan_form.html.tera", &ctx).unwrap())
}

#[handler]
pub async fn update_budget_plan(
    state: Data<&AppState>,
    Path(id): Path<i32>,
    form: Form<entities::budget_plan::Model>,
) -> poem::Result<impl IntoResponse> {
    let conn = &state.conn;
    let form = form.0;

    let budget_plan = MutationCore::update_budget_plan_by_id(conn, id, form)
        .await
        .map_err(InternalServerError)?;

    let mut ctx = tera::Context::new();
    ctx.insert("budget_plan", &budget_plan);
    let body = state
        .templates
        .render("budget_plan/budget_plan_row.html.tera", &ctx)
        .map_err(InternalServerError)?;
    Ok(Html(body))
}

#[handler]
pub async fn delete_budget_plan(
    db: Data<&DatabaseConnection>,
    Path(id): Path<i32>,
) -> poem::Result<impl IntoResponse> {
    entities::budget_plan::Entity::delete_by_id(id).exec(db.0).await.ok();
    redirect("/budget/plans")
}
#[handler]
pub async fn new_plan_item_form(
    db: Data<&DatabaseConnection>,
    tera: Data<&Tera>,
    Path(plan_id): Path<i32>,
) -> Html<String> {
    let all_budget_items = budget_item::Entity::find()
        .all(db.0)
        .await
        .unwrap_or_default();
    let mut ctx = tera::Context::new();
    ctx.insert("budget_items", &all_budget_items);
    ctx.insert("plan_id", &plan_id);
    Html(
        tera.render("budget/plan_item_form.html.tera", &ctx)
            .unwrap(),
    )
}

#[handler]
pub async fn create_plan_item(
    db: Data<&DatabaseConnection>,
    Path(plan_id): Path<i32>,
    Form(form): Form<budget_plan_item::Model>,
) -> poem::Result<impl IntoResponse> {
    let mut new_item = budget_plan_item::ActiveModel {
        budget_plan_id: Set(plan_id),
        budget_item_id: Set(form.budget_item_id),
        month: Set(form.month),
        planned_amount: Set(form.planned_amount),
        note: Set(form.note),
        ..Default::default()
    };
    new_item.insert(db.0).await.ok();
    redirect(format!("/budget/plans/{}/items", plan_id).as_str())
}

#[handler]
pub async fn edit_plan_item_form(
    db: Data<&DatabaseConnection>,
    tera: Data<&Tera>,
    Path((plan_id, id)): Path<(i32, i32)>,
) -> Html<String> {
    let item = budget_plan_item::Entity::find_by_id(id)
        .one(db.0)
        .await
        .unwrap();
    let all_budget_items = budget_item::Entity::find()
        .all(db.0)
        .await
        .unwrap_or_default();
    let mut ctx = tera::Context::new();
    ctx.insert("item", &item);
    ctx.insert("budget_items", &all_budget_items);
    ctx.insert("plan_id", &plan_id);
    Html(
        tera.render("budget/plan_item_form.html.tera", &ctx)
            .unwrap(),
    )
}

#[handler]
pub async fn delete_plan_item(
    db: Data<&DatabaseConnection>,
    Path((plan_id, id)): Path<(i32, i32)>,
) -> poem::Result<impl IntoResponse> {
    budget_plan_item::Entity::delete_by_id(id)
        .exec(db.0)
        .await
        .ok();
    redirect(format!("/budget/plans/{}/items", plan_id).as_str())
}


