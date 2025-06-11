use poem::{delete, get, post, put, Route};
use crate::handlers::budget::budget_item::{create_budget_item, delete_budget_item, edit_budget_item_form, list_budget_items, new_budget_item_form, update_budget_item};
use crate::handlers::budget::budget_plan::{create_budget_plan, create_plan_item, delete_budget_plan, delete_plan_item, edit_budget_plan_form, edit_plan_item_form, list_budget_plans, new_budget_plan_form, new_plan_item_form, update_budget_plan};

pub (crate) mod budget_item;
pub (crate) mod budget_plan;

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
