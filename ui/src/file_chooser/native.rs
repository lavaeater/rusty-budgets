use crate::Button;
use crate::file_chooser::FileData;
use dioxus::logger::tracing::info;
use dioxus::prelude::*;

/// Saves `contents` to a file the user picks via a native/browser save
/// dialog. Returns `Err` if the user cancelled or the write failed.
pub async fn save_bytes_to_file(title: &str, file_name: &str, contents: &[u8]) -> Result<(), String> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title(title)
        .set_file_name(file_name)
        .save_file()
        .await
        .ok_or_else(|| "cancelled".to_string())?;

    handle.write(contents).await.map_err(|e| e.to_string())
}

#[component]
pub fn FileDialog(
    on_chosen: EventHandler<FileData>,
    #[props(default = "Importera från bank".to_string())] label: String,
    #[props(default = "Välj en fil att importera".to_string())] title: String,
    #[props(default = "Excel".to_string())] filter_name: String,
    #[props(default = vec!["xlsx".to_string(), "xls".to_string()])] filter_extensions: Vec<String>,
) -> Element {
    let pick_file = move |_| {
        let title = title.clone();
        let filter_name = filter_name.clone();
        let filter_extensions = filter_extensions.clone();
        spawn(async move {
            let extensions: Vec<&str> = filter_extensions.iter().map(String::as_str).collect();
            let file = rfd::AsyncFileDialog::new()
                .add_filter(&filter_name, &extensions)
                .set_title(&title)
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
        Button { class: "primary", onclick: pick_file, {label} }
    }
}

/// Lets the user save `contents` to a file of their choosing — a native save
/// dialog on desktop, a browser download on web.
#[component]
pub fn SaveFileDialog(
    contents: Vec<u8>,
    #[props(default = "Spara fil".to_string())] label: String,
    #[props(default = "Spara fil".to_string())] title: String,
    #[props(default = "export.json".to_string())] file_name: String,
) -> Element {
    let save_file = move |_| {
        let contents = contents.clone();
        let title = title.clone();
        let file_name = file_name.clone();
        spawn(async move {
            match save_bytes_to_file(&title, &file_name, &contents).await {
                Ok(()) => info!("File saved: {} ({} bytes)", file_name, contents.len()),
                Err(e) => info!("Failed to save file: {}", e),
            }
        });
    };

    rsx! {
        Button { class: "primary", onclick: save_file, {label} }
    }
}
