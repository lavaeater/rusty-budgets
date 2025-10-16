// Example: How to create a variant switcher for testing all three UI variants
// Place this in your app's views/home.rs or similar

use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let mut variant = use_signal(|| "original");
    
    rsx! {
        div { style: "display: flex; flex-direction: column; height: 100vh;",
            // Variant selector toolbar
            div { style: "padding: 10px; background: #2d3748; display: flex; gap: 10px; align-items: center; box-shadow: 0 2px 4px rgba(0,0,0,0.1);",
                span { style: "color: white; font-weight: 600; margin-right: 10px;",
                    "UI Variant:"
                }
                button {
                    style: if variant() == "original" { "padding: 8px 16px; background: #667eea; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 600;" } else { "padding: 8px 16px; background: #4a5568; color: white; border: none; border-radius: 4px; cursor: pointer;" },
                    onclick: move |_| variant.set("original"),
                    "Original"
                }
                button {
                    style: if variant() == "variant_a" { "padding: 8px 16px; background: #667eea; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 600;" } else { "padding: 8px 16px; background: #4a5568; color: white; border: none; border-radius: 4px; cursor: pointer;" },
                    onclick: move |_| variant.set("variant_a"),
                    "Variant A (Dashboard)"
                }
                button {
                    style: if variant() == "variant_b" { "padding: 8px 16px; background: #667eea; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 600;" } else { "padding: 8px 16px; background: #4a5568; color: white; border: none; border-radius: 4px; cursor: pointer;" },
                    onclick: move |_| variant.set("variant_b"),
                    "Variant B (Workflow)"
                }
                span { style: "color: #a0aec0; margin-left: auto; font-size: 0.9rem;",
                    match variant() {
                        "variant_a" => "Dashboard-focused with overview cards",
                        "variant_b" => "Workflow-oriented with sidebar",
                        _ => "Original one-page layout",
                    }
                }
            }
            // Main content area
            div { style: "flex: 1; overflow: hidden;",
                match variant() {
                    "variant_a" => rsx! {
                        ui::budget_a::BudgetHero {}
                    },
                    "variant_b" => rsx! {
                        ui::budget_b::BudgetHero {}
                    },
                    _ => rsx! {
                        ui::budget::BudgetHero {}
                    },
                }
            }
        }
    }
}

// Alternative: Simple version without switcher (production use)
#[component]
pub fn HomeSimple() -> Element {
    rsx! {
        // Original
        ui::budget::BudgetHero {}
        // Or Variant B
        // ui::budget_b::BudgetHero {}
    }
}
