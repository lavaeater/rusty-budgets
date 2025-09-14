use dioxus::prelude::*;
use dioxus_primitives::collapsible;

/// The props for the [`dioxus_primitives::collapsible::Collapsible`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CollapsibleProps {
    /// Keep [`dioxus_primitives::collapsible::CollapsibleContent`] mounted in the DOM when the collapsible is closed.
    ///
    /// This does not apply any special ARIA or other attributes.
    #[props(default)]
    pub keep_mounted: ReadOnlySignal<bool>,

    /// The default `open` state.
    ///
    /// This will be overridden if the component is controlled.
    #[props(default)]
    pub default_open: bool,

    /// The disabled state of the collapsible.
    #[props(default)]
    pub disabled: ReadOnlySignal<bool>,

    /// The controlled `open` state of the collapsible.
    ///
    /// If this is provided, you must use `on_open_change`.
    pub open: ReadOnlySignal<Option<bool>>,

    /// A callback for when the open state changes.
    ///
    /// The provided argument is a bool of whether the collapsible is open or closed.
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// Additional attributes for the collapsible element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the collapsible component.
    pub children: Element,
}

#[component]
pub fn Collapsible(props: CollapsibleProps) -> Element {
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: asset!("/src/budget_components/collapsible/style.css"),
        }
        collapsible::Collapsible {
            keep_mounted: props.keep_mounted,
            default_open: props.default_open,
            disabled: props.disabled,
            open: props.open,
            on_open_change: props.on_open_change,
            attributes: props.attributes,
            class: "collapsible",
            {props.children}
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct CollapsibleContentProps {
    /// The ID of the collapsible content element.
    pub id: ReadOnlySignal<Option<String>>,

    /// Additional attributes for the collapsible content element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the collapsible content.
    pub children: Element,
}

/// The props for the [`dioxus_primitives::collapsible::CollapsibleTrigger`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CollapsibleTriggerProps {
    /// Additional attributes for the collapsible trigger element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the collapsible trigger.
    pub children: Element,
}


#[component]
pub fn CollapsibleTrigger(props: CollapsibleTriggerProps) -> Element {
    rsx! {
        collapsible::CollapsibleTrigger { class: "collapsible-trigger",
            attributes: props.attributes,
            {props.children}
            svg {
                class: "collapsible-expand-icon",
                view_box: "0 0 24 24",
                xmlns: "http://www.w3.org/2000/svg",
                // shifted up by 6 polyline { points: "6 9 12 15 18 9" }
                polyline { points: "6 15 12 21 18 15" }
                // shifted down by 6 polyline { points: "6 15 12 9 18 15" }
                polyline { points: "6 9 12 3 18 9" }
            }
        }
    }
}

#[component]
pub fn CollapsibleContent(props: CollapsibleContentProps) -> Element {
    rsx! {
        collapsible::CollapsibleContent { class: "collapsible-content",
            id: props.id,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn CollapsibleItem(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            border: "1px solid var(--primary-color-6)",
            border_radius: "0.5rem",
            padding: "1rem",
            ..attributes,
            {children}
        }
    }
}

#[component]
pub fn CollapsibleList(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            display: "flex",
            flex_direction: "column",
            gap: "0.5rem",
            max_width: "20rem",
            color: "var(--secondary-color-3)",
            ..attributes,
            {children}
        }
    }
}
