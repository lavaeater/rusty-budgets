use dioxus::prelude::*;
use dioxus_primitives::accordion::{
    self,
};

#[derive(Props, Clone, PartialEq)]
pub struct AccordionProps {
    /// The id of the accordion root element.
    pub id: Option<String>,
    /// The class of the accordion root element.
    pub class: Option<String>,
    /// The style of the accordion root element.
    pub style: Option<String>,

    /// Whether multiple accordion items are allowed to be open at once.
    ///
    /// Defaults to false.
    #[props(default)]
    pub allow_multiple_open: ReadOnlySignal<bool>,

    /// Set whether the accordion is disabled.
    #[props(default)]
    pub disabled: ReadOnlySignal<bool>,

    /// Whether the accordion can be fully collapsed.
    ///
    /// Setting this to true will allow all accordion items to close. Defaults to true.
    #[props(default = ReadOnlySignal::new(Signal::new(true)))]
    pub collapsible: ReadOnlySignal<bool>,

    /// Whether the accordion is horizontal.
    ///
    /// Settings this to true will use left/right keybinds for navigation instead of up/down. Defaults to false.
    #[props(default)]
    pub horizontal: ReadOnlySignal<bool>,

    /// Attributes to extend the root element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the accordion, which should contain [`dioxus_primitives::accordion::AccordionItem`] components.
    pub children: Element,
}

#[component]
pub fn Accordion(props: AccordionProps) -> Element {
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: asset!("/src/budget_components/accordion/style.css"),
        }
        accordion::Accordion {
            class: "accordion",
            width: "15rem",
            id: props.id,
            allow_multiple_open: props.allow_multiple_open,
            disabled: props.disabled,
            collapsible: props.collapsible,
            horizontal: props.horizontal,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct AccordionItemProps {
    /// Whether the accordion item is disabled.
    #[props(default)]
    pub disabled: ReadOnlySignal<bool>,

    /// Whether this accordion item should be opened by default.
    #[props(default)]
    pub default_open: bool,

    /// Callback for when the accordion's open/closed state changes.
    ///
    /// The new value is provided.
    #[props(default)]
    pub on_change: Callback<bool, ()>,

    /// Callback for when the trigger is clicked.
    #[props(default)]
    pub on_trigger_click: Callback,

    /// The index of the accordion item within the [`dioxus_primitives::accordion::Accordion`].
    ///
    /// This is required to implement keyboard navigation and focus management.
    pub index: usize,

    /// Additional attributes to extend the item element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the accordion item.
    pub children: Element,
}

#[component]
pub fn AccordionItem(props: AccordionItemProps) -> Element {
    rsx! {
        accordion::AccordionItem {
            class: "accordion-item",
            disabled: props.disabled,
            default_open: props.default_open,
            on_change: props.on_change,
            on_trigger_click: props.on_trigger_click,
            index: props.index,
            attributes: props.attributes,
            {props.children}
        }
    }
}


#[derive(Props, Clone, PartialEq)]
pub struct AccordionTriggerProps {
    /// THe id of the accordion trigger element.
    pub id: Option<String>,
    /// Additional attributes to extend the trigger element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the accordion trigger element.
    pub children: Element,
}

#[component]
pub fn AccordionTrigger(props: AccordionTriggerProps) -> Element {
    rsx! {
        accordion::AccordionTrigger {
            class: "accordion-trigger",
            id: props.id,
            attributes: props.attributes,
            {props.children}
            svg {
                class: "accordion-expand-icon",
                view_box: "0 0 24 24",
                xmlns: "http://www.w3.org/2000/svg",
                polyline { points: "6 9 12 15 18 9" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct AccordionContentProps {
    /// The id of the accordion content element.
    pub id: ReadOnlySignal<Option<String>>,
    /// Additional attributes to extend the content element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the accordion content element.
    pub children: Element,
}


#[component]
pub fn AccordionContent(props: AccordionContentProps) -> Element {
    rsx! {
        accordion::AccordionContent {
            class: "accordion-content",
            style: "--collapsible-content-width: 140px",
            id: props.id,
            attributes: props.attributes,
            {props.children}
        }
    }
}
