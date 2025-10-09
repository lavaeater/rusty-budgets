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

#[component]
fn FileChooser() -> Element {
    // Build cool things ✌️
    let mut files = use_signal(Files::new);

    rsx! {
        div {
            link {
                href: "https://fonts.googleapis.com/icon?family=Material+Icons",
                rel: "stylesheet",
            }
            // link { rel: "stylesheet", href: "main.css" }
            header {
                i { class: "material-icons icon-menu", "menu" }
                h1 { "Files: {files.read().current()}" }
                span {}
                i {
                    class: "material-icons",
                    onclick: move |_| files.write().go_up(),
                    "tracingout"
                }
            }
            main {
                for (dir_id , path) in files.read().path_names.iter().enumerate() {
                    div { class: "folder", key: "{path.name}",
                        i {
                            class: "material-icons",
                            onclick: move |_| files.write().enter_dir(dir_id),
                            // Change the icon
                            if path.is_directory {
                                "folder"
                            } else {
                                "description"
                            }
                            // The tooltip
                            p { class: "cooltip", "0 folders / 0 files" }
                        }
                        h1 { "{path.name}" }
                    }
                }
                if let Some(err) = files.read().err.as_ref() {
                    div {
                        code { "{err}" }
                        button { onclick: move |_| files.write().clear_err(), "x" }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct File {
    is_directory: bool,
    name: String,
}

struct Files {
    path_stack: Vec<String>,
    path_names: Vec<File>,
    err: Option<String>,
}

impl Files {
    fn new() -> Self {
        let mut files = Self {
            path_stack: vec![".".to_string()],
            path_names: vec![],
            err: None,
        };

        files.reload_path_list();

        files
    }

    fn reload_path_list(&mut self) {
        let cur_path = self.path_stack.join("/");
        tracing::info!("Reloading path list for {:?}", cur_path);
        let paths = match std::fs::read_dir(&cur_path) {
            Ok(e) => e,
            Err(err) => {
                // Likely we're trying to open a file, so let's open it!
                if let Ok(_) = open::that(cur_path) {
                    tracing::info!("Opened file");
                    return;
                } else {
                    let err = format!("An error occurred: {:?}", err);
                    self.err = Some(err);
                    self.path_stack.pop();
                    return;
                }
            }
        };

        let collected = paths.collect::<Vec<_>>();
        tracing::info!("Path list reloaded {:#?}", collected);

        // clear the current state
        self.clear_err();
        self.path_names.clear();

        for path in collected {
            let file = path.unwrap();
            self.path_names.push(File {
                name: file.file_name().to_str().unwrap().to_string(),
                is_directory: file.file_type().unwrap().is_dir(),
            });
        }
        tracing::info!("path names are {:#?}", self.path_names);
    }

    fn go_up(&mut self) {
        if self.path_stack.len() > 1 {
            self.path_stack.pop();
        }
        self.reload_path_list();
    }

    fn enter_dir(&mut self, dir_id: usize) {
        let path = &self.path_names[dir_id];
        self.path_stack.push(path.name.to_string());
        self.reload_path_list();
    }

    fn current(&self) -> String {
        self.path_stack.join("/")
    }

    fn clear_err(&mut self) {
        self.err = None;
    }
}