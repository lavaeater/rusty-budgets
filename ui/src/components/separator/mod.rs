use dioxus::prelude::*;
use dioxus_primitives::separator::{self};

#[derive(Props, Clone, PartialEq)]
pub struct SeparatorProps {
    /// Horizontal if true, vertical if false.
    #[props(default = true)]
    pub horizontal: bool,

    /// If the separator is decorative and should not be classified
    /// as a separator to the ARIA standard.
    #[props(default = false)]
    pub decorative: bool,

    /// Additional attributes to apply to the separator element.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,

    /// The children of the separator component.
    children: Element,
}

#[component]
pub fn Separator(props: SeparatorProps) -> Element {
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: asset!("/src/components/separator/style.css"),
        }
        separator::Separator {
            class: "separator",
            horizontal: props.horizontal,
            decorative: props.decorative,
            attributes: props.attributes,
            {props.children}
        }
    }
}
