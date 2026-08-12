//! Cleanroom Rust port of upstream Go source file: `bubbles.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! <public-docs>
//! # Bubbles
//!
//! Components for Bubble Tea applications. These components are used in
//! production in Glow, Charm and many other applications.
//!
//! Ported packages:
//! - [`key`] — user-definable keymappings
//! - [`cursor`] — cursor state management
//! - [`stopwatch`] — stopwatch component
//! - [`timer`] — timeout component
//! - [`spinner`] — spinner component
//! - [`help`] — help view
//! - [`paginator`] — paginator component
//! - [`progress`] — progress bar component
//! - [`textinput`] — text input component
//! - [`list`] — list component
//! - [`table`] — table component
//! - [`viewport`] — viewport component
//! - [`textarea`] — multi-line text area component
//! - [`filepicker`] — file picker component
//! </public-docs>

pub mod cursor;
pub mod filepicker;
pub mod help;
pub mod internal;
pub mod key;
pub mod list;
pub mod paginator;
pub mod progress;
pub mod spinner;
pub mod stopwatch;
pub mod table;
pub mod textarea;
pub mod textinput;
pub mod timer;
pub mod viewport;
