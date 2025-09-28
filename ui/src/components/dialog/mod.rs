use dioxus::prelude::*;
use dioxus_primitives::dialog;

#[derive(Props, Clone, PartialEq)]
pub struct DialogProps {
    /// The ID of the dialog content element.
    pub id: ReadOnlySignal<Option<String>>,

    /// The class to apply to the dialog content element.
    #[props(default)]
    pub class: Option<String>,

    /// Additional attributes to apply to the dialog content element.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// The children of the dialog content.
    children: Element,
}

#[derive(Props, Clone, PartialEq)]
pub struct DialogTitleProps {
    /// The ID of the dialog title element.
    pub id: ReadOnlySignal<Option<String>>,
    /// Additional attributes for the dialog title element.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// The children of the dialog title.
    children: Element,
}

#[derive(Props, Clone, PartialEq)]
pub struct DialogDescriptionProps {
    /// The ID of the dialog description element.
    pub id: ReadOnlySignal<Option<String>>,
    /// Additional attributes for the dialog description element.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// The children of the dialog description.
    children: Element,
}


#[derive(Props, Clone, PartialEq)]
pub struct DialogRootProps {
    /// The ID of the dialog root element.
    pub id: ReadOnlySignal<Option<String>>,

    /// Whether the dialog is modal. If true, it will trap focus within the dialog when open.
    #[props(default = ReadOnlySignal::new(Signal::new(true)))]
    pub is_modal: ReadOnlySignal<bool>,

    /// The controlled `open` state of the dialog.
    pub open: ReadOnlySignal<Option<bool>>,

    /// The default `open` state of the dialog if it is not controlled.
    #[props(default)]
    pub default_open: bool,

    /// A callback that is called when the open state changes.
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// Additional attributes to apply to the dialog root element.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,

    /// The children of the dialog root component.
    children: Element,
}

#[component]
pub fn DialogRoot(props: DialogRootProps) -> Element {
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: asset!("/src/components/dialog/style.css"),
        }
        dialog::DialogRoot {
            class: "dialog-backdrop",
            id: props.id,
            is_modal: props.is_modal,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn DialogContent(props: DialogProps) -> Element {
    rsx! {
        dialog::DialogContent { class: "dialog", id: props.id, attributes: props.attributes, {props.children} }
    }
}

#[component]
pub fn DialogTitle(props: DialogTitleProps) -> Element {
    rsx! {
        dialog::DialogTitle {
            class: "dialog-title",
            id: props.id,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn DialogDescription(props: DialogDescriptionProps) -> Element {
    rsx! {
        dialog::DialogDescription {
            class: "dialog-description",
            id: props.id,
            attributes: props.attributes,
            {props.children}
        }
    }
}