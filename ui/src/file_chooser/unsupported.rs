//! Android has no `rfd` backend, so there is no native file picker yet
//! (would need a Storage Access Framework integration via JNI). These stubs
//! keep the shared `ui` crate compiling for Android and render the same
//! buttons in a disabled state instead of silently doing nothing.
use crate::Button;
use crate::file_chooser::FileData;
use dioxus::logger::tracing::warn;
use dioxus::prelude::*;

const UNSUPPORTED_MSG: &str = "Filimport/export stöds inte än på mobil";

pub async fn save_bytes_to_file(_title: &str, _file_name: &str, _contents: &[u8]) -> Result<(), String> {
    warn!("save_bytes_to_file called on a platform without a file dialog backend");
    Err(UNSUPPORTED_MSG.to_string())
}

#[component]
pub fn FileDialog(
    on_chosen: EventHandler<FileData>,
    #[props(default = "Importera från bank".to_string())] label: String,
    #[props(default = "Välj en fil att importera".to_string())] title: String,
    #[props(default = "Excel".to_string())] filter_name: String,
    #[props(default = vec!["xlsx".to_string(), "xls".to_string()])] filter_extensions: Vec<String>,
) -> Element {
    let _ = (on_chosen, title, filter_name, filter_extensions);
    rsx! {
        Button { class: "primary", disabled: true, title: UNSUPPORTED_MSG, {label} }
    }
}

#[component]
pub fn SaveFileDialog(
    contents: Vec<u8>,
    #[props(default = "Spara fil".to_string())] label: String,
    #[props(default = "Spara fil".to_string())] title: String,
    #[props(default = "export.json".to_string())] file_name: String,
) -> Element {
    let _ = (contents, title, file_name);
    rsx! {
        Button { class: "primary", disabled: true, title: UNSUPPORTED_MSG, {label} }
    }
}
