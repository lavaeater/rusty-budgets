//! This crate contains all shared UI for the workspace.
#![allow(unused_imports)]
#![allow(dead_code)]
// Original budget module (one-page layout)
pub mod budget;

mod components;
mod file_chooser;
mod version;

pub use components::*;
pub use version::{VersionBadge, BUILD_TIME, GIT_HASH, GIT_VERSION};

// Re-export BudgetHero from the original budget module for backward compatibility
// To use alternative variants, import from budget_a or budget_b modules explicitly
pub use budget::BudgetHero;
