//! This crate contains all shared UI for the workspace.
#![allow(unused_imports)]
#![allow(dead_code)]
mod hero;
pub use hero::Hero;

// Original budget module (one-page layout)
pub mod budget;

mod components;
mod file_chooser;

pub use components::*;

// Re-export BudgetHero from the original budget module for backward compatibility
// To use alternative variants, import from budget_a or budget_b modules explicitly
pub use budget::BudgetHero;