use dioxus::prelude::*;

const BUDGET_CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn NewBudgetItem() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: BUDGET_CSS}

        div {
            id: "blog",

            // Content
            h1 { "This is blog!" }
            p { "In blog, we show how the Dioxus router works and how URL parameters can be passed as props to our route components." }

            span { " <---> " }
        }
    }
}