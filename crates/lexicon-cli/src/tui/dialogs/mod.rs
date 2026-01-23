//! TUI Dialog Components
//!
//! Modal dialogs for various TUI interactions.

mod confirm;
mod save;

pub use confirm::{ConfirmAction, ConfirmDialog};
pub use save::{DialogAction, SaveDialog};
