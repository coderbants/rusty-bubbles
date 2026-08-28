//! Cleanroom user documentation source for the generic Bubbles widgets.
//!
//! <user-docs>
//! rusty-bubbles provides typed model/update/view components for common
//! terminal interfaces. Components are independent and deterministic: callers
//! own the event loop and pass messages to the component they compose.
//!
//! The table component renders each declared column in order. Short rows yield
//! empty cells and surplus row values are ignored, so malformed input cannot
//! change the table shape or panic the renderer:
//!
//! ```
//! use rusty_bubbles::table::{self, Column};
//!
//! let table = table::new(vec![
//!     table::with_width(16),
//!     table::with_columns(&[
//!         Column { title: "Name".into(), width: 8 },
//!         Column { title: "State".into(), width: 8 },
//!     ]),
//!     table::with_rows(&[vec!["Bubbles".into()]]),
//! ]);
//! assert_eq!(table.selected_row().unwrap().len(), 1);
//! assert!(table.view().contains("Bubbles"));
//! ```
//! </user-docs>
//!
//! Internal maintainer note: this source is the documentation-owned projection
//! for the BUI-012 target. Keep the example synchronized with the public table
//! facade and its deterministic boundary behavior.
