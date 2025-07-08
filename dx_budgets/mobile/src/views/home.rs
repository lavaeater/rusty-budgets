use dioxus::prelude::*;
use ui::{Users, Hero};

#[component]
pub fn Home() -> Element {
    rsx! {
        Hero {}
        Users {}
    }
}
