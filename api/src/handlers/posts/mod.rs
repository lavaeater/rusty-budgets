use crate::handlers::auth::login_required_middleware::login_required_middleware;
use crate::handlers::auth::required_role_middleware::RequiredRoleMiddleware;
use crate::{AppState, PaginationParams, DEFAULT_ITEMS_PER_PAGE};
use entities::{post, post::Model as Post};
use poem::error::InternalServerError;
use poem::http::StatusCode;
use poem::web::{Data, Form, Html, Path, Query};
use poem::{get, handler, post, EndpointExt, Error, IntoResponse, Route};
use sea_orm::prelude::Uuid;
use service::{MutationCore as MutationCore, QueryCore as QueryCore};

#[handler]
pub async fn create(state: Data<&AppState>, form: Form<Post>) -> poem::Result<impl IntoResponse> {
    let form = form.0;
    let conn = &state.conn;

    MutationCore::create_post(conn, form)
        .await
        .map_err(InternalServerError)?;

    Ok(StatusCode::ACCEPTED.with_header("HX-Redirect", "/posts"))
}

#[handler]
pub async fn list(
    state: Data<&AppState>,
    Query(params): Query<PaginationParams>,
) -> poem::Result<impl IntoResponse> {
    let conn = &state.conn;
    let page = params.page.unwrap_or(1);
    let posts_per_page = params.items_per_page.unwrap_or(DEFAULT_ITEMS_PER_PAGE);

    let (posts, num_pages) = QueryCore::find_posts_in_page(conn, page, posts_per_page)
        .await
        .map_err(InternalServerError)?;

    let mut ctx = tera::Context::new();
    ctx.insert("posts", &posts);
    ctx.insert("page", &page);
    ctx.insert("posts_per_page", &posts_per_page);
    ctx.insert("num_pages", &num_pages);

    let body = state
        .templates
        .render("posts/list.html.tera", &ctx)
        .map_err(InternalServerError)?;
    Ok(Html(body))
}

#[handler]
pub async fn new(state: Data<&AppState>) -> poem::Result<impl IntoResponse> {
    let ctx = tera::Context::new();
    let body = state
        .templates
        .render("posts/new.html.tera", &ctx)
        .map_err(InternalServerError)?;
    Ok(Html(body))
}

#[handler]
pub async fn edit(state: Data<&AppState>, Path(id): Path<Uuid>) -> poem::Result<impl IntoResponse> {
    let conn = &state.conn;

    let post: post::Model = QueryCore::find_post_by_id(conn, id)
        .await
        .map_err(InternalServerError)?
        .ok_or_else(|| Error::from_status(StatusCode::NOT_FOUND))?;

    let mut ctx = tera::Context::new();
    ctx.insert("post", &post);

    let body = state
        .templates
        .render("posts/edit.html.tera", &ctx)
        .map_err(InternalServerError)?;
    Ok(Html(body))
}

#[handler]
pub async fn update(
    state: Data<&AppState>,
    Path(id): Path<Uuid>,
    form: Form<post::Model>,
) -> poem::Result<impl IntoResponse> {
    let conn = &state.conn;
    let form = form.0;

    let post = MutationCore::update_post_by_id(conn, id, form)
        .await
        .map_err(InternalServerError)?;

    let mut ctx = tera::Context::new();
    ctx.insert("post", &post);

    let body = state
        .templates
        .render("posts/post_row.html.tera", &ctx)
        .map_err(InternalServerError)?;
    Ok(Html(body))
}

#[handler]
pub async fn destroy(
    state: Data<&AppState>,
    Path(id): Path<Uuid>,
) -> poem::Result<impl IntoResponse> {
    let conn = &state.conn;

    MutationCore::delete_post(conn, id)
        .await
        .map_err(InternalServerError)?;

    Ok(StatusCode::ACCEPTED.with_header("HX-Redirect", "/posts"))
}

pub fn post_routes() -> Route {
    Route::new()
        .at("/", get(list).around(login_required_middleware))
        .at(
            "/create",
            post(create).with(RequiredRoleMiddleware::new("super_admin")),
        )
        .at(
            "/new",
            get(new).with(RequiredRoleMiddleware::new("super_admin")),
        )
        .at(
            "/:id",
            get(edit)
                .patch(update)
                .delete(destroy)
                .with(RequiredRoleMiddleware::new("super_admin")),
        )
}
