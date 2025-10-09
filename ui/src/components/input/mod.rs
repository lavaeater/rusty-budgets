use dioxus::prelude::*;

#[component]
pub fn Input(
    #[props(default)]
    id: Option<String>,
    #[props(default)]
    placeholder: Option<String>,
    #[props(default)]
    value: Option<String>,
    #[props(default)]
    oninput: Option<EventHandler<FormEvent>>,
    #[props(default)]
    onchange: Option<EventHandler<FormEvent>>,
    #[props(default)]
    class: Option<String>,
    #[props(default)]
    r#type: Option<String>,
    children: Element,
) -> Element {
    let combined_class = match class {
        Some(custom_class) => format!("input {}", custom_class),
        None => "input".to_string(),
    };

    rsx! {
        document::Link {
            rel: "stylesheet",
            href: asset!("/src/components/input/style.css"),
        }
        input {
            class: combined_class,
            id,
            placeholder,
            value,
            r#type: r#type.unwrap_or_else(|| "text".to_string()),
            oninput: move |evt| {
                if let Some(handler) = &oninput {
                    handler.call(evt);
                }
            },
            onchange: move |evt| {
                if let Some(handler) = &onchange {
                    handler.call(evt);
                }
            },
            {children}
        }
    }
}
