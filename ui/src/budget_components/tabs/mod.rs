use dioxus::prelude::*;
use dioxus_primitives::tabs::{self};
/// The props for the [`dioxus_primitives::tabs::TabTrigger`] component
#[derive(Props, Clone, PartialEq)]
pub struct TabTriggerProps {
    /// The value of the tab trigger, which is used to identify the corresponding tab content. This
    /// must match the `value` prop of the corresponding [`dioxus_primitives::tabs::TabContent`].
    pub value: String,
    /// The index of the tab trigger. This is used to define the focus order for keyboard navigation.
    pub index: ReadOnlySignal<usize>,

    /// Whether the tab trigger is disabled.
    #[props(default)]
    pub disabled: ReadOnlySignal<bool>,

    /// The ID of the tab trigger element.
    pub id: Option<String>,
    /// The class of the tab trigger element.
    pub class: Option<String>,

    /// Additional attributes to apply to the tab trigger element.
    #[props(extends = GlobalAttributes)]
    #[props(extends = button)]
    attributes: Vec<Attribute>,

    /// The children of the tab trigger component.
    children: Element,
}
#[derive(Props, Clone, PartialEq)]
pub struct TabContentProps {
    /// The value of the tab content, which must match the `value` prop of the corresponding [`dioxus_primitives::tabs::TabTrigger`].
    pub value: String,

    /// The ID of the tab content element.
    pub id: ReadOnlySignal<Option<String>>,
    /// The class of the tab content element.
    pub class: Option<String>,

    /// The index of the tab content. This is used to define the focus order for keyboard navigation.
    pub index: ReadOnlySignal<usize>,

    /// Additional attributes to apply to the tab content element.
    #[props(extends = GlobalAttributes)]
    #[props(extends = div)]
    attributes: Vec<Attribute>,

    /// The children of the tab content element.
    children: Element,
}

#[derive(Props, Clone, PartialEq)]
pub struct TabListProps {
    /// Additional attributes to apply to the tab list element.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,

    /// The children of the tab list component.
    children: Element,
}

/// The props for the [`Tabs`] component.
#[derive(Props, Clone, PartialEq)]
pub struct TabsProps {
    /// The class of the tabs component.
    #[props(default)]
    pub class: String,

    /// The controlled value of the active tab.
    pub value: ReadSignal<Option<String>>,

    /// The default active tab value when uncontrolled.
    #[props(default)]
    pub default_value: String,

    /// Callback fired when the active tab changes.
    #[props(default)]
    pub on_value_change: Callback<String>,

    /// Whether the tabs are disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Whether the tabs are horizontal.
    #[props(default)]
    pub horizontal: ReadSignal<bool>,

    /// Whether focus should loop around when reaching the end.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub roving_loop: ReadSignal<bool>,

    /// The variant of the tabs component.
    #[props(default)]
    pub variant: TabsVariant,

    /// Additional attributes to apply to the tabs element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the tabs component.
    pub children: Element,
}

/// The variant of the tabs component.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum TabsVariant {
    /// The default variant.
    #[default]
    Default,
    /// The ghost variant.
    Ghost,
}

impl TabsVariant {
    /// Convert the variant to a string for use in class names
    fn to_class(self) -> &'static str {
        match self {
            TabsVariant::Default => "default",
            TabsVariant::Ghost => "ghost",
        }
    }
}

#[component]
pub fn Tabs(props: TabsProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/budget_components/tabs/style.css") }
        tabs::Tabs {
            class: props.class + " tabs",
            "data-variant": props.variant.to_class(),
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            disabled: props.disabled,
            horizontal: props.horizontal,
            roving_loop: props.roving_loop,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn TabList(props: TabListProps) -> Element {
    rsx! {
        tabs::TabList { class: "tabs-list", attributes: props.attributes, {props.children} }
    }
}

#[component]
pub fn TabTrigger(props: TabTriggerProps) -> Element {
    rsx! {
        tabs::TabTrigger {
            class: "tabs-trigger",
            id: props.id,
            value: props.value,
            index: props.index,
            disabled: props.disabled,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn TabContent(props: TabContentProps) -> Element {
    rsx! {
        tabs::TabContent {
            class: props.class.unwrap_or_default() + " tabs-content tabs-content-themed",
            value: props.value,
            id: props.id,
            index: props.index,
            attributes: props.attributes,
            {props.children}
        }
    }
}
