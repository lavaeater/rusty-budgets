use dioxus::prelude::*;
use uuid::Uuid;
use api::models::budget::Budget;

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");

pub struct BudgetSignal {
    id: Uuid,
    name: Signal<String, UnsyncStorage>,
    edit_name: Signal<bool, UnsyncStorage>,
    default_budget: Signal<bool, UnsyncStorage>,
    edit_default_budget: Signal<bool, UnsyncStorage>,
    user_id: Uuid
}

impl BudgetSignal {
    pub fn from(budget: &Budget) -> Self {
        Self {
            id: budget.id,
            name: use_signal(|| budget.name.clone()),
            edit_name: use_signal(|| false),
            default_budget: use_signal(|| budget.default_budget),
            edit_default_budget: use_signal(|| false),
            user_id: budget.user_id
        }
    }
    
    pub fn into_budget(self) -> Budget {
        Budget {
            id: self.id,
            name: self.name.read().to_string(),
            user_id: self.user_id,
            default_budget: *self.default_budget.read()
        }
    }
}

#[component]
pub fn BudgetHero() -> Element {
    let budget = use_server_future(|| api::get_default_budget())?().unwrap().unwrap();
    let mut budget_vm = BudgetSignal::from(&budget);
    
    let on_key_down = move |e: KeyboardEvent| {
        if e.code() == Code::Enter {
            &budget_vm.edit_name.set(false);
            let m = use_server_future(|| api::save_budget(budget_vm.into_budget()));
        }
    };
    
    rsx! {
        document::Link { rel: "stylesheet", href: BUDGET_CSS }
        div {
            id: "budget_hero",
            if *budget_vm.edit_name.read() {
                input {
                    oninput: move |e| {
                        budget_vm.name.set(e.value().clone());
                    },
                    onkeydown: on_key_down,
                    type: "text",
                    value: "{budget_vm.name.read()}",
                }                
            } else {
                h4 {
                    onclick: move |_| {
                        budget_vm.edit_name.set(true);
                    },
                    "{budget_vm.name.read()}",
                }
            }
        }
    }
}
