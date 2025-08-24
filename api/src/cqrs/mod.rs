// Minimal, generic CQRS + Event Sourcing framework in one file.
// No external crates. `rustc` stable compatible.
// ---------------------------------------------------------------
// This file contains:
// 1) A tiny generic framework (traits + in-memory runtime)
// 2) A small demo domain (bank account) showing commands/events
// 3) A `main` that exercises the framework

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

mod budgets;
mod framework;
