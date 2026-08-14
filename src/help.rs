//! Cleanroom Rust port of upstream Go source file: `help/help.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! <public-docs>
//! # Help
//!
//! A simple help view for Bubble Tea applications.
//! </public-docs>

use crate::key::Binding;
use rusty_bubbletea::model::{Cmd, Msg};
use rusty_lipgloss::{color::Color, style::Style, TOP};

/// KeyMap is a map of keybindings used to generate help. Since it's an
/// interface it can be any type, though a struct or a map of bindings are
/// likely candidates.
///
/// Note that if a key is disabled (via `key.Binding::set_enabled`) it will
/// not be rendered in the help view, so in theory generated help should
/// self-manage.
pub trait KeyMap {
    /// ShortHelp returns a slice of bindings to be displayed in the short
    /// version of the help. The help bubble will render help in the order in
    /// which the help items are returned here.
    fn short_help(&self) -> Vec<Binding>;

    /// FullHelp returns an extended group of help items, grouped by columns.
    /// The help bubble will render the help in the order in which the help
    /// items are returned here.
    fn full_help(&self) -> Vec<Vec<Binding>>;
}

/// Styles is a set of available style definitions for the Help bubble.
#[derive(Debug, Clone)]
pub struct Styles {
    /// Styling for the ellipsis indicator.
    pub ellipsis: Style,

    /// Styling for the short help
    pub short_key: Style,
    /// Styling for the short help
    pub short_desc: Style,
    /// Styling for the short help
    pub short_separator: Style,

    /// Styling for the full help
    pub full_key: Style,
    /// Styling for the full help
    pub full_desc: Style,
    /// Styling for the full help
    pub full_separator: Style,
}

/// DefaultStyles returns a set of default styles for the help bubble. Light
/// or dark styles can be selected by passing `is_dark`.
pub fn default_styles(is_dark: bool) -> Styles {
    let light_dark = rusty_lipgloss::color::light_dark(is_dark);

    let key_style =
        Style::new().foreground_color(light_dark(Color::parse("#909090"), Color::parse("#626262")));
    let desc_style =
        Style::new().foreground_color(light_dark(Color::parse("#B2B2B2"), Color::parse("#4A4A4A")));
    let sep_style =
        Style::new().foreground_color(light_dark(Color::parse("#DADADA"), Color::parse("#3C3C3C")));

    Styles {
        short_key: key_style.clone(),
        short_desc: desc_style.clone(),
        short_separator: sep_style.clone(),
        ellipsis: sep_style.clone(),
        full_key: key_style,
        full_desc: desc_style,
        full_separator: sep_style,
    }
}

/// DefaultDarkStyles returns a set of default styles for dark backgrounds.
pub fn default_dark_styles() -> Styles {
    default_styles(true)
}

/// DefaultLightStyles returns a set of default styles for light backgrounds.
pub fn default_light_styles() -> Styles {
    default_styles(false)
}

/// Model contains the state of the help view.
#[derive(Debug, Clone)]
pub struct Model {
    /// if true, render the "full" help menu
    pub show_all: bool,

    /// The separator used in the short help.
    pub short_separator: String,
    /// The separator used in the full help.
    pub full_separator: String,

    /// The symbol we use in the short help when help items have been
    /// truncated due to width. Periods of ellipsis by default.
    pub ellipsis: String,

    /// The styles used by the help view.
    pub styles: Styles,

    width: usize,
}

/// New creates a new help view with some useful defaults.
pub fn new() -> Model {
    Model {
        show_all: false,
        short_separator: " • ".to_string(),
        full_separator: "    ".to_string(),
        ellipsis: "…".to_string(),
        styles: default_dark_styles(),
        width: 0,
    }
}

impl Model {
    /// Update helps satisfy the Bubble Tea Model interface. It's a no-op.
    pub fn update(&mut self, _msg: &dyn Msg) -> Cmd {
        None
    }

    /// View renders the help view's current state.
    pub fn view(&self, k: &dyn KeyMap) -> String {
        if self.show_all {
            return self.full_help_view(&k.full_help());
        }
        self.short_help_view(&k.short_help())
    }

    /// SetWidth sets the maximum width for the help view.
    pub fn set_width(&mut self, w: usize) {
        self.width = w;
    }

    /// Width returns the maximum width for the help view.
    pub fn width(&self) -> usize {
        self.width
    }

    /// ShortHelpView renders a single line help view from a slice of
    /// keybindings. If the line is longer than the maximum width it will be
    /// gracefully truncated, showing only as many help items as possible.
    pub fn short_help_view(&self, bindings: &[Binding]) -> String {
        if bindings.is_empty() {
            return String::new();
        }

        let mut b = String::new();
        let mut total_width = 0;
        let separator = self
            .styles
            .short_separator
            .clone()
            .inline(true)
            .render(&self.short_separator);

        for (i, kb) in bindings.iter().enumerate() {
            if !kb.enabled() {
                continue;
            }

            // Sep
            let sep = if total_width > 0 && i < bindings.len() {
                separator.clone()
            } else {
                String::new()
            };

            // Item
            let str = sep
                + &self
                    .styles
                    .short_key
                    .clone()
                    .inline(true)
                    .render(&kb.help().key)
                + " "
                + &self
                    .styles
                    .short_desc
                    .clone()
                    .inline(true)
                    .render(&kb.help().desc);
            let w = rusty_lipgloss::size::width(&str);

            // Tail
            if let (tail, false) = self.should_add_item(total_width, w) {
                if !tail.is_empty() {
                    b.push_str(&tail);
                }
                break;
            }

            total_width += w;
            b.push_str(&str);
        }

        b
    }

    /// FullHelpView renders help columns from a slice of key binding slices.
    /// Each top level slice entry renders into a column.
    pub fn full_help_view(&self, groups: &[Vec<Binding>]) -> String {
        if groups.is_empty() {
            return String::new();
        }

        let mut out: Vec<String> = Vec::new();

        let mut total_width = 0;
        let separator = self
            .styles
            .full_separator
            .clone()
            .inline(true)
            .render(&self.full_separator);

        // Iterate over groups to build columns
        for (i, group) in groups.iter().enumerate() {
            if group.is_empty() || !should_render_column(group) {
                continue;
            }
            let mut keys: Vec<String> = Vec::new();
            let mut descriptions: Vec<String> = Vec::new();

            // Sep
            let sep = if total_width > 0 && i < groups.len() {
                separator.clone()
            } else {
                String::new()
            };

            // Separate keys and descriptions into different slices
            for kb in group {
                if !kb.enabled() {
                    continue;
                }
                keys.push(kb.help().key);
                descriptions.push(kb.help().desc);
            }

            // Column
            let key_col = self.styles.full_key.clone().render(&keys.join("\n"));
            let desc_col = self
                .styles
                .full_desc
                .clone()
                .render(&descriptions.join("\n"));
            let col = rusty_lipgloss::join::join_horizontal(TOP, &[&sep, &key_col, " ", &desc_col]);
            let w = rusty_lipgloss::size::width(&col);

            // Tail
            if let (tail, false) = self.should_add_item(total_width, w) {
                if !tail.is_empty() {
                    out.push(tail);
                }
                break;
            }

            total_width += w;
            out.push(col);
        }

        let refs: Vec<&str> = out.iter().map(|s| s.as_str()).collect();
        rusty_lipgloss::join::join_horizontal(TOP, &refs)
    }

    fn should_add_item(&self, total_width: usize, width: usize) -> (String, bool) {
        // If there's room for an ellipsis, print that.
        if self.width > 0 && total_width + width > self.width {
            let tail = String::from(" ")
                + &self
                    .styles
                    .ellipsis
                    .clone()
                    .inline(true)
                    .render(&self.ellipsis);

            if total_width + rusty_lipgloss::size::width(&tail) < self.width {
                return (tail, false);
            }
        }
        (String::new(), true)
    }
}

fn should_render_column(b: &[Binding]) -> bool {
    b.iter().any(|v| v.enabled())
}
