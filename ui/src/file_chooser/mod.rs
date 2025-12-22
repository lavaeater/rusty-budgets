use dioxus::logger::tracing::info;
use dioxus::prelude::*;
use rfd::AsyncFileDialog;
use crate::Button;

#[derive(Clone, Debug)]
pub struct FileData {
    pub name: String,
    pub contents: Vec<u8>,
}

#[component]
pub fn FileDialog(on_chosen: EventHandler<FileData>) -> Element {
    let pick_file = move |_| {
        spawn(async move {
            let file = AsyncFileDialog::new()
                .add_filter("Excel", &["xlsx", "xls"])
                .set_title("Välj en fil att importera")
                .pick_file()
                .await;

            if let Some(handle) = file {
                let name = handle.file_name();
                let contents = handle.read().await;
                info!("File picked: {} ({} bytes)", name, contents.len());
                on_chosen.call(FileData { name, contents });
            }
        });
    };

    rsx! {
        Button { class: "primary", onclick: pick_file, "Importera från bank" }
    }
}