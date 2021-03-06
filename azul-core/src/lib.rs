//! Shared datatypes for azul-* crates

pub mod app;
pub mod app_resources;
pub mod r#async;
pub mod callbacks;
pub mod diff;
pub mod display_list;
pub mod dom;
pub mod gl;
pub mod id_tree;
pub mod style;
pub mod ui_description;
pub mod ui_solver;
pub mod ui_state;
pub mod window;

mod stack_checked_pointer;

// Typedef for possible faster implementation of hashing
pub type FastHashMap<T, U> = ::std::collections::HashMap<T, U>;
pub type FastHashSet<T> = ::std::collections::HashSet<T>;
