use std::rc::Rc;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::dialog::{DialogContent, DialogDescription, DialogRoot, DialogTitle};
use crate::{Button, Input};

pub type FileChosen = Event<String>;

#[component]
pub fn FileDialog( on_chosen: EventHandler<FileChosen>) -> Element {
    let mut open = use_signal(|| false);
    let mut file_name = use_signal(|| "".to_string());

    rsx! {
        Button { class: "primary", onclick: move |_| open.set(true), "Importera från bank" }
        DialogRoot { open: open(), on_open_change: move |v| open.set(v),
            DialogContent {
                Input {
                    id: "file_name",
                    placeholder: "Filnamn",
                    value: file_name(),
                    oninput: move |e: FormEvent| { file_name.set(e.value().clone()) },
                }
                Button {
                    class: "secondary",
                    aria_label: "Avbryt",
                    tabindex: if open() { "2" } else { "-1" },
                    onclick: move |_| open.set(false),
                    "Avbryt"
                }
                Button {
                    class: "primary",
                    r#type: "button",
                    aria_label: "Importera",
                    tabindex: if open() { "1" } else { "-1" },
                    onclick: move |_| {
                        open.set(false);
                        on_chosen.call(FileChosen::new(Rc::new(file_name()), false));
                    },
                    "Importera"
                }
                DialogTitle { "Välj en fil att importera" }
                DialogDescription { "Just nu finns stöd för excel-ark från Skandiabanken." }
            }
        }
    }
}