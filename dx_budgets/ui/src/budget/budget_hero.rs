use dioxus::logger::tracing;
use api::models::budget::Budget;
use dioxus::prelude::*;
use uuid::Uuid;

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");

#[derive(Clone, Debug, PartialEq)]
pub struct BudgetSignal {
    id: Uuid,
    name: Signal<String, UnsyncStorage>,
    edit_name: Signal<bool, UnsyncStorage>,
    default_budget: Signal<bool, UnsyncStorage>,
    edit_default_budget: Signal<bool, UnsyncStorage>,
    user_id: Uuid,
}

impl BudgetSignal {
    pub fn from(budget: &Budget) -> Self {
        Self {
            id: budget.id,
            name: use_signal(|| budget.name.clone()),
            edit_name: use_signal(|| false),
            default_budget: use_signal(|| budget.default_budget),
            edit_default_budget: use_signal(|| false),
            user_id: budget.user_id,
        }
    }

    pub fn to_budget(&self) -> Budget {
        let temp = self.clone();
        let x = Budget {
            id: temp.id,
            name: temp.name.read().to_string(),
            user_id: temp.user_id,
            default_budget: *temp.default_budget.read(),
        };
        x
    }
}

#[component]
pub fn BudgetHero() -> Element {
    // Resource for fetching budget data
    let mut budget_resource = use_resource(|| async move { 
        api::get_default_budget().await 
    });
    
    // Persistent signal for budget data
    let mut budget_signal = use_signal(|| None::<Budget>);
    
    // Local state for editing
    let mut is_editing = use_signal(|| false);
    let mut edit_name = use_signal(|| String::new());
    
    // Update budget signal when resource changes
    use_effect(move || {
        if let Some(Ok(budget)) = budget_resource.read_unchecked().as_ref() {
            budget_signal.set(Some(budget.clone()));
            // Initialize edit_name when budget loads (only if not already set)
            if edit_name.read().is_empty() {
                edit_name.set(budget.name.clone());
            }
        }
    });
    
    // Handle the resource state
    match budget_signal() {
        Some(budget) => {
            let budget_clone = budget.clone();
            
            rsx! {
                document::Link { rel: "stylesheet", href: BUDGET_CSS }
                div {
                    id: "budget_hero",
                    if *is_editing.read() {
                        input {
                            r#type: "text",
                            value: "{edit_name.read()}",
                            oninput: move |e| {
                                edit_name.set(e.value());
                            },
                            onkeydown: move |e| {
                                if e.code() == Code::Enter {
                                    let budget_to_save = Budget {
                                        id: budget_clone.id,
                                        name: edit_name.read().clone(),
                                        default_budget: budget_clone.default_budget,
                                        user_id: budget_clone.user_id,
                                    };
                                    
                                    spawn(async move {
                                        match api::save_budget(budget_to_save).await {
                                            Ok(_) => {
                                                // Update successful, refresh the resource
                                                budget_resource.restart();
                                            }
                                            Err(e) => {
                                                // Handle error (could add error state here)
                                                tracing::error!("Failed to save budget: {}", e);
                                            }
                                        }
                                    });
                                    
                                    is_editing.set(false);
                                }
                            },
                            autofocus: true,
                        }
                    } else {
                        h4 {
                            onclick: move |_| {
                                edit_name.set(budget.name.clone());
                                is_editing.set(true);
                            },
                            "{budget.name}"
                        }
                    }
                }
            }
        }
        None => {
            // Check if we have an error or are still loading
            match budget_resource.read_unchecked().as_ref() {
                Some(Err(e)) => rsx! {
                    div {
                        id: "budget_hero",
                        h4 { "Error loading budget: {e}" }
                    }
                },
                _ => rsx! {
                    div {
                        id: "budget_hero", 
                        h4 { "Loading..." }
                    }
                }
            }
        }
    }
}
