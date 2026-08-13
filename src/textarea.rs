//! Cleanroom Rust port of upstream Go source file: `textarea/textarea.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! <public-docs>
//! # TextArea
//!
//! A multi-line text input component for Bubble Tea applications.
//!
//! The placeholder word/hard wrapping is a char-based port of the
//! `charmbracelet/x/ansi` `Wordwrap`/`Hardwrap` algorithms.
//! </public-docs>

use crate::cursor;
use crate::internal::clipboard;
use crate::internal::memoization;
use crate::internal::runeutil::{self, Sanitizer};
use crate::key::{self, Binding};
use crate::viewport;
use rusty_bubbletea::cursor::CursorShape;
use rusty_bubbletea::key::KeyPressMsg;
use rusty_bubbletea::model::{Cmd, Msg};
use rusty_bubbletea::paste::PasteMsg;
use rusty_lipgloss::{self, Color, Style};
use std::fmt;
use std::time::Duration;
use unicode_width::UnicodeWidthChar;

const MIN_HEIGHT: usize = 1;
const DEFAULT_HEIGHT: usize = 6;
const DEFAULT_WIDTH: usize = 40;
const DEFAULT_CHAR_LIMIT: usize = 0; // no limit
const DEFAULT_MAX_HEIGHT: usize = 99;
const DEFAULT_MAX_WIDTH: usize = 500;

// XXX: in v2, make max lines dynamic and default max lines configurable.
const MAX_LINES: usize = 10000;

/// Internal messages for clipboard operations.
#[derive(Debug)]
pub struct PasteMsgInternal(pub String);

#[derive(Debug)]
pub struct PasteErrMsg(pub String);

/// KeyMap is the key bindings for different actions within the textarea.
#[derive(Debug, Clone)]
pub struct KeyMap {
    /// CharacterBackward binding.
    pub character_backward: Binding,
    /// CharacterForward binding.
    pub character_forward: Binding,
    /// DeleteAfterCursor binding.
    pub delete_after_cursor: Binding,
    /// DeleteBeforeCursor binding.
    pub delete_before_cursor: Binding,
    /// DeleteCharacterBackward binding.
    pub delete_character_backward: Binding,
    /// DeleteCharacterForward binding.
    pub delete_character_forward: Binding,
    /// DeleteWordBackward binding.
    pub delete_word_backward: Binding,
    /// DeleteWordForward binding.
    pub delete_word_forward: Binding,
    /// InsertNewline binding.
    pub insert_newline: Binding,
    /// LineEnd binding.
    pub line_end: Binding,
    /// LineNext binding.
    pub line_next: Binding,
    /// LinePrevious binding.
    pub line_previous: Binding,
    /// LineStart binding.
    pub line_start: Binding,
    /// PageUp binding.
    pub page_up: Binding,
    /// PageDown binding.
    pub page_down: Binding,
    /// Paste binding.
    pub paste: Binding,
    /// WordBackward binding.
    pub word_backward: Binding,
    /// WordForward binding.
    pub word_forward: Binding,
    /// InputBegin binding.
    pub input_begin: Binding,
    /// InputEnd binding.
    pub input_end: Binding,

    /// UppercaseWordForward binding.
    pub uppercase_word_forward: Binding,
    /// LowercaseWordForward binding.
    pub lowercase_word_forward: Binding,
    /// CapitalizeWordForward binding.
    pub capitalize_word_forward: Binding,

    /// TransposeCharacterBackward binding.
    pub transpose_character_backward: Binding,
}

/// DefaultKeyMap returns the default set of key bindings for navigating and
/// acting upon the textarea.
pub fn default_key_map() -> KeyMap {
    KeyMap {
        character_forward: key::new_binding(vec![
            key::with_keys(&["right", "ctrl+f"]),
            key::with_help("right", "character forward"),
        ]),
        character_backward: key::new_binding(vec![
            key::with_keys(&["left", "ctrl+b"]),
            key::with_help("left", "character backward"),
        ]),
        word_forward: key::new_binding(vec![
            key::with_keys(&["alt+right", "alt+f"]),
            key::with_help("alt+right", "word forward"),
        ]),
        word_backward: key::new_binding(vec![
            key::with_keys(&["alt+left", "alt+b"]),
            key::with_help("alt+left", "word backward"),
        ]),
        line_next: key::new_binding(vec![
            key::with_keys(&["down", "ctrl+n"]),
            key::with_help("down", "next line"),
        ]),
        line_previous: key::new_binding(vec![
            key::with_keys(&["up", "ctrl+p"]),
            key::with_help("up", "previous line"),
        ]),
        delete_word_backward: key::new_binding(vec![
            key::with_keys(&["alt+backspace", "ctrl+w"]),
            key::with_help("alt+backspace", "delete word backward"),
        ]),
        delete_word_forward: key::new_binding(vec![
            key::with_keys(&["alt+delete", "alt+d"]),
            key::with_help("alt+delete", "delete word forward"),
        ]),
        delete_after_cursor: key::new_binding(vec![
            key::with_keys(&["ctrl+k"]),
            key::with_help("ctrl+k", "delete after cursor"),
        ]),
        delete_before_cursor: key::new_binding(vec![
            key::with_keys(&["ctrl+u"]),
            key::with_help("ctrl+u", "delete before cursor"),
        ]),
        insert_newline: key::new_binding(vec![
            key::with_keys(&["enter", "ctrl+m"]),
            key::with_help("enter", "insert newline"),
        ]),
        delete_character_backward: key::new_binding(vec![
            key::with_keys(&["backspace", "ctrl+h"]),
            key::with_help("backspace", "delete character backward"),
        ]),
        delete_character_forward: key::new_binding(vec![
            key::with_keys(&["delete", "ctrl+d"]),
            key::with_help("delete", "delete character forward"),
        ]),
        line_start: key::new_binding(vec![
            key::with_keys(&["home", "ctrl+a"]),
            key::with_help("home", "line start"),
        ]),
        line_end: key::new_binding(vec![
            key::with_keys(&["end", "ctrl+e"]),
            key::with_help("end", "line end"),
        ]),
        page_up: key::new_binding(vec![
            key::with_keys(&["pgup"]),
            key::with_help("pgup", "page up"),
        ]),
        page_down: key::new_binding(vec![
            key::with_keys(&["pgdown"]),
            key::with_help("pgdown", "page down"),
        ]),
        paste: key::new_binding(vec![
            key::with_keys(&["ctrl+v"]),
            key::with_help("ctrl+v", "paste"),
        ]),
        input_begin: key::new_binding(vec![
            key::with_keys(&["alt+<", "ctrl+home"]),
            key::with_help("alt+<", "input begin"),
        ]),
        input_end: key::new_binding(vec![
            key::with_keys(&["alt+>", "ctrl+end"]),
            key::with_help("alt+>", "input end"),
        ]),
        capitalize_word_forward: key::new_binding(vec![
            key::with_keys(&["alt+c"]),
            key::with_help("alt+c", "capitalize word forward"),
        ]),
        lowercase_word_forward: key::new_binding(vec![
            key::with_keys(&["alt+l"]),
            key::with_help("alt+l", "lowercase word forward"),
        ]),
        uppercase_word_forward: key::new_binding(vec![
            key::with_keys(&["alt+u"]),
            key::with_help("alt+u", "uppercase word forward"),
        ]),
        transpose_character_backward: key::new_binding(vec![
            key::with_keys(&["ctrl+t"]),
            key::with_help("ctrl+t", "transpose character backward"),
        ]),
    }
}

/// LineInfo is a helper for keeping track of line information regarding
/// soft-wrapped lines.
#[derive(Debug, Clone, Copy)]
pub struct LineInfo {
    /// Width is the number of columns in the line.
    pub width: usize,

    /// CharWidth is the number of characters in the line to account for
    /// double-width runes.
    pub char_width: usize,

    /// Height is the number of rows in the line.
    pub height: usize,

    /// StartColumn is the index of the first column of the line.
    pub start_column: usize,

    /// ColumnOffset is the number of columns that the cursor is offset from
    /// the start of the line.
    pub column_offset: usize,

    /// RowOffset is the number of rows that the cursor is offset from the
    /// start of the line.
    pub row_offset: usize,

    /// CharOffset is the number of characters that the cursor is offset
    /// from the start of the line. This will generally be equivalent to
    /// ColumnOffset, but will be different if there are double-width runes
    /// before the cursor.
    pub char_offset: usize,
}

/// PromptInfo is a struct that can be used to store information about the
/// prompt.
#[derive(Debug, Clone, Copy)]
pub struct PromptInfo {
    /// The line number of the prompt.
    pub line_number: usize,
    /// Whether the textarea is focused.
    pub focused: bool,
}

/// CursorStyle is the style for real and virtual cursors.
#[derive(Debug, Clone)]
pub struct CursorStyle {
    /// Style styles the cursor block. For real cursors, the foreground
    /// color set here will be used as the cursor color.
    pub color: Color,

    /// Shape is the cursor shape. The following shapes are available:
    ///
    /// - [`CursorShape::CursorBlock`]
    /// - [`CursorShape::CursorUnderline`]
    /// - [`CursorShape::CursorBar`]
    pub shape: CursorShape,

    /// CursorBlink determines whether or not the cursor should blink.
    pub blink: bool,

    /// BlinkSpeed is the speed at which the virtual cursor blinks. This has
    /// no effect on real cursors as well as no effect if the cursor is set
    /// not to blink.
    pub blink_speed: Duration,
}

/// Styles are the styles for the textarea, separated into focused and
/// blurred states. The appropriate styles will be chosen based on the focus
/// state of the textarea.
#[derive(Debug, Clone)]
pub struct Styles {
    /// The styles used when focused.
    pub focused: StyleState,
    /// The styles used when blurred.
    pub blurred: StyleState,
    /// The cursor style.
    pub cursor: CursorStyle,
}

/// StyleState that will be applied to the text area.
///
/// StyleState can be applied to focused and unfocused states to change the
/// styles depending on the focus state.
#[derive(Debug, Clone)]
pub struct StyleState {
    /// The base style.
    pub base: Style,
    /// The text style.
    pub text: Style,
    /// The line number style.
    pub line_number: Style,
    /// The cursor line number style.
    pub cursor_line_number: Style,
    /// The cursor line style.
    pub cursor_line: Style,
    /// The end-of-buffer style.
    pub end_of_buffer: Style,
    /// The placeholder style.
    pub placeholder: Style,
    /// The prompt style.
    pub prompt: Style,
}

impl StyleState {
    fn computed_cursor_line(&self) -> Style {
        self.cursor_line.clone().inherit(&self.base).inline(true)
    }

    fn computed_cursor_line_number(&self) -> Style {
        self.cursor_line_number
            .clone()
            .inherit(&self.cursor_line)
            .inherit(&self.base)
            .inline(true)
    }

    fn computed_end_of_buffer(&self) -> Style {
        self.end_of_buffer.clone().inherit(&self.base).inline(true)
    }

    fn computed_line_number(&self) -> Style {
        self.line_number.clone().inherit(&self.base).inline(true)
    }

    fn computed_placeholder(&self) -> Style {
        self.placeholder.clone().inherit(&self.base).inline(true)
    }

    fn computed_prompt(&self) -> Style {
        self.prompt.clone().inherit(&self.base).inline(true)
    }

    fn computed_text(&self) -> Style {
        self.text.clone().inherit(&self.base).inline(true)
    }
}

/// Model is the Bubble Tea model for this text area element.
pub struct Model {
    /// The validation error, if any.
    pub err: Option<String>,

    /// cache is the memoization cache for wrapped lines.
    cache: memoization::MemoCache<Vec<Vec<char>>>,

    /// Prompt is printed at the beginning of each line.
    pub prompt: String,

    /// Placeholder is the text displayed when the user hasn't entered
    /// anything yet.
    pub placeholder: String,

    /// ShowLineNumbers, if enabled, causes line numbers to be printed after
    /// the prompt.
    pub show_line_numbers: bool,

    /// EndOfBufferCharacter is displayed at the end of the input.
    pub end_of_buffer_character: char,

    /// KeyMap encodes the keybindings recognized by the widget.
    pub key_map: KeyMap,

    /// virtualCursor manages the virtual cursor.
    pub virtual_cursor: cursor::Model,

    /// CharLimit is the maximum number of characters this input element
    /// will accept. If 0 or less, there's no limit.
    pub char_limit: usize,

    /// MaxHeight is the maximum height of the text area in rows. If 0 or
    /// less, there's no limit.
    pub max_height: usize,

    /// MaxWidth is the maximum width of the text area in columns. If 0 or
    /// less, there's no limit.
    pub max_width: usize,

    /// DynamicHeight, when true, causes the textarea to automatically grow
    /// and shrink its height to fit the content. The height is clamped
    /// between MinHeight and MaxHeight.
    pub dynamic_height: bool,

    /// MinHeight is the minimum height of the text area in rows when
    /// DynamicHeight is enabled. If 0 or less, defaults to 1.
    pub min_height: usize,

    /// MaxContentHeight is the maximum content height in visual rows
    /// (accounting for soft wraps). When 0, the content guard falls back to
    /// the legacy MaxHeight behavior.
    pub max_content_height: usize,

    /// Styling. Styles are defined in [`Styles`].
    pub styles: Styles,

    /// useVirtualCursor determines whether or not to use the virtual cursor.
    pub use_virtual_cursor: bool,

    /// If prompt_func is set, it replaces Prompt as a generator for prompt
    /// strings at the beginning of each line.
    pub prompt_func: Option<Box<dyn Fn(PromptInfo) -> String + Send + Sync>>,

    /// prompt_width is the width of the prompt.
    pub prompt_width: usize,

    /// width is the maximum number of characters that can be displayed at
    /// once.
    pub width: usize,

    /// height is the maximum number of lines that can be displayed at once.
    pub height: usize,

    /// Underlying text value.
    value: Vec<Vec<char>>,

    /// focus indicates whether user input focus should be on this input
    /// component.
    pub focus: bool,

    /// Cursor column.
    col: usize,

    /// Cursor row.
    row: usize,

    /// Last character offset, used to maintain state when the cursor is
    /// moved vertically.
    last_char_offset: usize,

    /// viewport is the vertically-scrollable viewport of the multi-line
    /// text input.
    viewport: viewport::Model,

    /// rune sanitizer for input.
    rsan: Option<runeutil::Sanitizer_>,
}

impl fmt::Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("textarea::Model")
            .field("focus", &self.focus)
            .field("row", &self.row)
            .field("col", &self.col)
            .field("lines", &self.value.len())
            .finish()
    }
}

/// New creates a new model with default settings.
pub fn new() -> Model {
    // The upstream textarea disables the viewport's keymap so that typed
    // characters (e.g. 'l', 'h', 'j', 'k') are not interpreted as viewport
    // scrolling.
    let vp = viewport::new(vec![viewport::with_key_map(viewport::KeyMap {
        page_down: key::new_binding(vec![]),
        page_up: key::new_binding(vec![]),
        half_page_up: key::new_binding(vec![]),
        half_page_down: key::new_binding(vec![]),
        up: key::new_binding(vec![]),
        down: key::new_binding(vec![]),
        left: key::new_binding(vec![]),
        right: key::new_binding(vec![]),
    })]);

    let cur = cursor::new();

    let styles = default_dark_styles();

    let mut m = Model {
        char_limit: DEFAULT_CHAR_LIMIT,
        max_height: DEFAULT_MAX_HEIGHT,
        max_width: DEFAULT_MAX_WIDTH,
        prompt: format!("{} ", rusty_lipgloss::border::thick_border().left),
        styles,
        cache: memoization::new_memo_cache(MAX_LINES),
        end_of_buffer_character: ' ',
        show_line_numbers: true,
        use_virtual_cursor: true,
        virtual_cursor: cur,
        key_map: default_key_map(),

        value: vec![vec![]; MIN_HEIGHT],
        focus: false,
        col: 0,
        row: 0,

        viewport: vp,
        err: None,
        placeholder: String::new(),
        dynamic_height: false,
        min_height: 0,
        max_content_height: 0,
        prompt_func: None,
        prompt_width: 0,
        width: 0,
        height: 0,
        last_char_offset: 0,
        rsan: None,
    };

    m.set_height(DEFAULT_HEIGHT);
    m.set_width(DEFAULT_WIDTH);

    m
}

/// DefaultStyles returns the default styles for focused and blurred states
/// for the textarea.
pub fn default_styles(is_dark: bool) -> Styles {
    let light_dark = rusty_lipgloss::color::light_dark(is_dark);

    Styles {
        focused: StyleState {
            base: rusty_lipgloss::new_style(),
            cursor_line: rusty_lipgloss::new_style()
                .background_color(light_dark(Color::parse("255"), Color::parse("0"))),
            cursor_line_number: rusty_lipgloss::new_style()
                .foreground_color(light_dark(Color::parse("240"), Color::parse("240"))),
            end_of_buffer: rusty_lipgloss::new_style()
                .foreground_color(light_dark(Color::parse("254"), Color::parse("0"))),
            line_number: rusty_lipgloss::new_style()
                .foreground_color(light_dark(Color::parse("249"), Color::parse("7"))),
            placeholder: rusty_lipgloss::new_style().foreground_color(Color::parse("240")),
            prompt: rusty_lipgloss::new_style().foreground_color(Color::parse("7")),
            text: rusty_lipgloss::new_style(),
        },
        blurred: StyleState {
            base: rusty_lipgloss::new_style(),
            cursor_line: rusty_lipgloss::new_style()
                .foreground_color(light_dark(Color::parse("245"), Color::parse("7"))),
            cursor_line_number: rusty_lipgloss::new_style()
                .foreground_color(light_dark(Color::parse("249"), Color::parse("7"))),
            end_of_buffer: rusty_lipgloss::new_style()
                .foreground_color(light_dark(Color::parse("254"), Color::parse("0"))),
            line_number: rusty_lipgloss::new_style()
                .foreground_color(light_dark(Color::parse("249"), Color::parse("7"))),
            placeholder: rusty_lipgloss::new_style().foreground_color(Color::parse("240")),
            prompt: rusty_lipgloss::new_style().foreground_color(Color::parse("7")),
            text: rusty_lipgloss::new_style()
                .foreground_color(light_dark(Color::parse("245"), Color::parse("7"))),
        },
        cursor: CursorStyle {
            color: Color::parse("7"),
            shape: CursorShape::CursorBlock,
            blink: true,
            blink_speed: Duration::from_millis(530),
        },
    }
}

/// DefaultLightStyles returns the default styles for a light background.
pub fn default_light_styles() -> Styles {
    default_styles(false)
}

/// DefaultDarkStyles returns the default styles for a dark background.
pub fn default_dark_styles() -> Styles {
    default_styles(true)
}

impl Model {
    /// Styles returns the current styles for the textarea.
    pub fn styles(&self) -> &Styles {
        &self.styles
    }

    /// SetStyles updates styling for the textarea.
    pub fn set_styles(&mut self, s: Styles) {
        self.styles = s;
        self.update_virtual_cursor_style();
    }

    /// VirtualCursor returns whether or not the virtual cursor is enabled.
    pub fn virtual_cursor(&self) -> bool {
        self.use_virtual_cursor
    }

    /// SetVirtualCursor sets whether or not to use the virtual cursor.
    pub fn set_virtual_cursor(&mut self, v: bool) {
        self.use_virtual_cursor = v;
        self.update_virtual_cursor_style();
    }

    /// updateVirtualCursorStyle sets styling on the virtual cursor based on
    /// the textarea's style settings.
    fn update_virtual_cursor_style(&mut self) {
        if !self.use_virtual_cursor {
            self.virtual_cursor.set_mode(cursor::Mode::Hide);
            return;
        }

        self.virtual_cursor.style =
            rusty_lipgloss::new_style().foreground_color(self.styles.cursor.color.clone());

        // By default, the blink speed of the cursor is set to a default
        // internally.
        if self.styles.cursor.blink {
            if self.styles.cursor.blink_speed > Duration::ZERO {
                self.virtual_cursor.blink_speed = self.styles.cursor.blink_speed;
            }
            self.virtual_cursor.set_mode(cursor::Mode::Blink);
            return;
        }
        self.virtual_cursor.set_mode(cursor::Mode::Static);
    }

    /// SetValue sets the value of the text input.
    pub fn set_value(&mut self, s: &str) {
        self.reset();
        self.insert_string(s);
        self.recalculate_height();
    }

    /// InsertString inserts a string at the cursor position.
    pub fn insert_string(&mut self, s: &str) {
        self.insert_runes_from_user_input(&s.chars().collect::<Vec<char>>());
        self.recalculate_height();
    }

    /// InsertRune inserts a rune at the cursor position.
    pub fn insert_rune(&mut self, r: char) {
        self.insert_runes_from_user_input(&[r]);
        self.recalculate_height();
    }

    /// insertRunesFromUserInput inserts runes at the current cursor
    /// position.
    fn insert_runes_from_user_input(&mut self, input: &[char]) {
        // Clean up any special characters in the input provided by the
        // clipboard. This avoids bugs due to e.g. tab characters and whatnot.
        let mut runes = self.san().sanitize(input);

        if self.char_limit > 0 {
            let avail_space = self.char_limit - self.length();
            // If the char limit's been reached, cancel.
            if avail_space == 0 {
                return;
            }
            // If there's not enough space to paste the whole thing cut the
            // pasted runes down so they'll fit.
            if avail_space < runes.len() {
                runes.truncate(avail_space);
            }
        }

        // Split the input into lines.
        let mut lines: Vec<Vec<char>> = vec![];
        let mut lstart = 0;
        for (i, r) in runes.iter().enumerate() {
            if *r == '\n' {
                // Queue a line to become a new row in the text area below.
                lines.push(runes[lstart..i].to_vec());
                lstart = i + 1;
            }
        }
        if lstart <= runes.len() {
            // The last line did not end with a newline character. Take it
            // now.
            lines.push(runes[lstart..].to_vec());
        }

        // Obey the maximum line limit.
        if MAX_LINES > 0 && self.value.len() + lines.len() - 1 > MAX_LINES {
            let allowed_height = MAX_LINES - self.value.len() + 1;
            lines.truncate(allowed_height);
        }

        // Obey MaxContentHeight in visual rows when set.
        if self.max_content_height > 0 {
            let budget = self.max_content_height - self.total_visual_lines();
            // Trim lines from the end until we fit within the budget.
            while lines.len() > 1 && self.visual_lines_for_insert(&lines) > budget {
                lines.truncate(lines.len() - 1);
            }
            if self.visual_lines_for_insert(&lines) > budget {
                return;
            }
        }

        if lines.is_empty() {
            // Nothing left to insert.
            return;
        }

        // Save the remainder of the original line at the current cursor
        // position.
        let tail: Vec<char> = self.value[self.row][self.col..].to_vec();

        // Paste the first line at the current cursor position.
        let mut first = self.value[self.row][..self.col].to_vec();
        first.extend_from_slice(&lines[0]);
        self.value[self.row] = first;
        self.col += lines[0].len();

        let num_extra_lines = lines.len() - 1;
        if num_extra_lines > 0 {
            // Add the new lines.
            let mut new_grid: Vec<Vec<char>> = self.value.clone();
            new_grid.resize(self.value.len() + num_extra_lines, vec![]);
            // Add all the rows that were after the cursor in the original
            // grid at the end of the new grid.
            let shift = self.row + 1 + num_extra_lines;
            for (idx, src) in (self.row + 1..self.value.len()).enumerate() {
                new_grid[shift + idx] = self.value[src].clone();
            }
            self.value = new_grid;
            // Insert all the new lines in the middle.
            for l in &lines[1..] {
                self.row += 1;
                self.value[self.row] = l.clone();
                self.col = l.len();
            }
        }

        // Finally add the tail at the end of the last line inserted.
        self.value[self.row].extend_from_slice(&tail);

        self.set_cursor_column(self.col);
    }

    /// Value returns the value of the text input.
    /// Value returns the value of the textarea.
    pub fn value(&self) -> String {
        if self.value.is_empty() {
            return String::new();
        }

        let mut v = String::new();
        for l in &self.value {
            v.push_str(&String::from_iter(l.iter()));
            v.push('\n');
        }

        v.trim_end_matches('\n').to_string()
    }

    /// Length returns the number of characters currently in the text input.
    pub fn length(&self) -> usize {
        let mut l = 0;
        for row in &self.value {
            l += string_width(&String::from_iter(row.iter()));
        }
        // We add len(value) to include the newline characters.
        l + self.value.len() - 1
    }

    /// LineCount returns the number of lines that are currently in the text
    /// input.
    pub fn line_count(&self) -> usize {
        self.value.len()
    }

    /// Line returns the 0-indexed row position of the cursor.
    pub fn line(&self) -> usize {
        self.row
    }

    /// Column returns the 0-indexed column position of the cursor.
    pub fn column(&self) -> usize {
        self.col
    }

    /// ScrollYOffset returns the Y offset (top row) index of the current
    /// view, which can be used to calculate the current scroll position.
    pub fn scroll_y_offset(&self) -> usize {
        self.viewport.y_offset()
    }

    /// SetScrollYOffset sets the Y offset (top row) index of the current
    /// view, clamping to the viewport's scrollable range.
    ///
    /// This is exposed so integration tests can position the view the same
    /// way the upstream in-package tests use `viewport.SetYOffset`.
    pub fn set_scroll_y_offset(&mut self, offset: usize) {
        self.viewport.set_y_offset(offset);
    }

    /// ScrollPercent returns the amount of the textarea that is currently
    /// scrolled through, clamped between 0 and 1.
    pub fn scroll_percent(&self) -> f64 {
        self.viewport.scroll_percent()
    }

    /// setCursorLineRelative moves the cursor by the given number of lines.
    /// Negative values move the cursor up, positive values move the cursor
    /// down.
    fn set_cursor_line_relative(&mut self, delta: isize) {
        if delta == 0 {
            return;
        }

        let mut li = self.line_info();
        let char_offset = self.last_char_offset.max(li.char_offset);
        self.last_char_offset = char_offset;

        // 2 columns to account for the trailing space wrapping.
        const TRAILING_SPACE: usize = 2;

        if delta > 0 {
            // Moving down.
            for _ in 0..delta {
                if li.row_offset + 1 >= li.height && self.row < self.value.len() - 1 {
                    self.row += 1;
                    self.col = 0;
                } else {
                    // Move the cursor to the start of the next virtual line.
                    self.col = (li.start_column + li.width + TRAILING_SPACE)
                        .min(self.value[self.row].len().saturating_sub(1));
                }
                li = self.line_info();
            }
        } else {
            // Moving up.
            for _ in 0..(-delta) {
                if li.row_offset == 0 && self.row > 0 {
                    self.row -= 1;
                    self.col = self.value[self.row].len();
                } else {
                    // Move the cursor to the end of the previous line.
                    self.col = li.start_column.saturating_sub(TRAILING_SPACE);
                }
                li = self.line_info();
            }
        }

        let nli = self.line_info();
        self.col = nli.start_column;

        if nli.width == 0 {
            self.reposition_view();
            return;
        }

        let mut offset = 0;
        while offset < char_offset {
            if self.row >= self.value.len()
                || self.col >= self.value[self.row].len()
                || offset >= nli.char_width.saturating_sub(1)
            {
                break;
            }
            offset += char_width(self.value[self.row][self.col]);
            self.col += 1;
        }
        self.reposition_view();
    }

    /// CursorDown moves the cursor down by one line.
    pub fn cursor_down(&mut self) {
        self.set_cursor_line_relative(1);
    }

    /// CursorUp moves the cursor up by one line.
    pub fn cursor_up(&mut self) {
        self.set_cursor_line_relative(-1);
    }

    /// CursorPosition returns the current (row, col) of the cursor within
    /// the (soft-wrapped) value.
    ///
    /// This is exposed so integration tests can assert on cursor placement
    /// the same way the upstream in-package tests do.
    pub fn cursor_position(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    /// SetCursorPosition sets the raw (row, col) of the cursor.
    ///
    /// This is exposed so integration tests can position the cursor the
    /// same way the upstream in-package tests do.
    pub fn set_cursor_position(&mut self, row: usize, col: usize) {
        if row >= self.value.len() {
            return;
        }
        self.row = row;
        self.col = clamp(col, 0, self.value[row].len());
        self.last_char_offset = 0;
    }

    /// SetCursorColumn moves the cursor to the given position. If the
    /// position is out of bounds the cursor will be moved to the start or
    /// end accordingly.
    pub fn set_cursor_column(&mut self, col: usize) {
        self.col = clamp(col, 0, self.value[self.row].len());
        // Any time that we move the cursor horizontally we need to reset the
        // last offset so that the horizontal position when navigating is
        // adjusted.
        self.last_char_offset = 0;
    }

    /// CursorStart moves the cursor to the start of the input field.
    pub fn cursor_start(&mut self) {
        self.set_cursor_column(0);
    }

    /// CursorEnd moves the cursor to the end of the input field.
    pub fn cursor_end(&mut self) {
        self.set_cursor_column(self.value[self.row].len());
    }

    /// Focused returns the focus state on the model.
    pub fn focused(&self) -> bool {
        self.focus
    }

    /// activeStyle returns the appropriate set of styles to use depending
    /// on whether the textarea is focused or blurred.
    fn active_style(&self) -> &StyleState {
        if self.focus {
            &self.styles.focused
        } else {
            &self.styles.blurred
        }
    }

    /// Focus sets the focus state on the model. When the model is in focus
    /// it can receive keyboard input and the cursor will be hidden.
    pub fn focus(&mut self) -> Cmd {
        self.focus = true;
        self.virtual_cursor.focus()
    }

    /// Blur removes the focus state on the model. When the model is blurred
    /// it can not receive keyboard input and the cursor will be hidden.
    pub fn blur(&mut self) {
        self.focus = false;
        self.virtual_cursor.blur();
    }

    /// Reset sets the input to its default state with no input.
    pub fn reset(&mut self) {
        self.value = vec![vec![]; MIN_HEIGHT];
        self.col = 0;
        self.row = 0;
        self.viewport.goto_top();
        self.set_cursor_column(0);
        self.recalculate_height();
    }

    /// Word returns the word at the cursor position.
    /// A word is delimited by spaces or line-breaks.
    pub fn word(&self) -> String {
        let line = &self.value[self.row];
        let col = self.col.saturating_sub(1);

        if self.col == 0 {
            return String::new();
        }

        // If cursor is beyond the line, return empty string
        if col >= line.len() {
            return String::new();
        }

        // If cursor is on a space, return empty string
        if line[col].is_whitespace() {
            return String::new();
        }

        // Find the start of the word by moving left
        let mut start = col;
        while start > 0 && !line[start - 1].is_whitespace() {
            start -= 1;
        }

        // Find the end of the word by moving right
        let mut end = col;
        while end < line.len() && !line[end].is_whitespace() {
            end += 1;
        }

        String::from_iter(line[start..end].iter())
    }

    /// san initializes or retrieves the rune sanitizer.
    fn san(&mut self) -> &runeutil::Sanitizer_ {
        if self.rsan.is_none() {
            self.rsan = Some(runeutil::new_sanitizer(vec![]));
        }
        self.rsan.as_ref().unwrap()
    }

    /// deleteBeforeCursor deletes all text before the cursor.
    fn delete_before_cursor(&mut self) {
        self.value[self.row] = self.value[self.row][self.col..].to_vec();
        self.set_cursor_column(0);
    }

    /// deleteAfterCursor deletes all text after the cursor.
    fn delete_after_cursor(&mut self) {
        self.value[self.row] = self.value[self.row][..self.col].to_vec();
        self.set_cursor_column(self.value[self.row].len());
    }

    /// transposeLeft exchanges the runes at the cursor and immediately
    /// before. No-op if the cursor is at the beginning of the line.
    fn transpose_left(&mut self) {
        if self.col == 0 || self.value[self.row].len() < 2 {
            return;
        }
        if self.col >= self.value[self.row].len() {
            self.set_cursor_column(self.col - 1);
        }
        self.value[self.row].swap(self.col - 1, self.col);
        if self.col < self.value[self.row].len() {
            self.set_cursor_column(self.col + 1);
        }
    }

    /// deleteWordLeft deletes the word left to the cursor.
    fn delete_word_left(&mut self) {
        if self.col == 0 || self.value[self.row].is_empty() {
            return;
        }

        // Linter note: it's critical that we acquire the initial cursor
        // position here prior to altering it via SetCursor() below.
        let old_col = self.col;

        self.set_cursor_column(self.col - 1);
        loop {
            if self.col == 0 {
                break;
            }
            if !self.value[self.row][self.col].is_whitespace() {
                break;
            }
            // ignore series of whitespace before cursor
            self.set_cursor_column(self.col - 1);
        }

        while self.col > 0 {
            if !self.value[self.row][self.col].is_whitespace() {
                self.set_cursor_column(self.col - 1);
            } else {
                if self.col > 0 {
                    // keep the previous space
                    self.set_cursor_column(self.col + 1);
                }
                break;
            }
        }

        if old_col > self.value[self.row].len() {
            self.value[self.row] = self.value[self.row][..self.col].to_vec();
        } else {
            let mut v = self.value[self.row][..self.col].to_vec();
            v.extend_from_slice(&self.value[self.row][old_col..]);
            self.value[self.row] = v;
        }
    }

    /// deleteWordRight deletes the word right to the cursor.
    fn delete_word_right(&mut self) {
        if self.col >= self.value[self.row].len() || self.value[self.row].is_empty() {
            return;
        }

        let old_col = self.col;

        while self.col < self.value[self.row].len()
            && self.value[self.row][self.col].is_whitespace()
        {
            // ignore series of whitespace after cursor
            self.set_cursor_column(self.col + 1);
        }

        while self.col < self.value[self.row].len() {
            if !self.value[self.row][self.col].is_whitespace() {
                self.set_cursor_column(self.col + 1);
            } else {
                break;
            }
        }

        if self.col > self.value[self.row].len() {
            self.value[self.row] = self.value[self.row][..old_col].to_vec();
        } else {
            let mut v = self.value[self.row][..old_col].to_vec();
            v.extend_from_slice(&self.value[self.row][self.col..]);
            self.value[self.row] = v;
        }

        self.set_cursor_column(old_col);
    }

    /// characterRight moves the cursor one character to the right.
    fn character_right(&mut self) {
        if self.col < self.value[self.row].len() {
            self.set_cursor_column(self.col + 1);
        } else if self.row < self.value.len() - 1 {
            self.row += 1;
            self.cursor_start();
        }
    }

    /// characterLeft moves the cursor one character to the left.
    fn character_left(&mut self, inside_line: bool) {
        if self.col == 0 && self.row != 0 {
            self.row -= 1;
            self.cursor_end();
            if !inside_line {
                return;
            }
        }
        if self.col > 0 {
            self.set_cursor_column(self.col - 1);
        }
    }

    /// wordLeft moves the cursor one word to the left.
    fn word_left(&mut self) {
        loop {
            self.character_left(true /* insideLine */);
            if self.col < self.value[self.row].len()
                && !self.value[self.row][self.col].is_whitespace()
            {
                break;
            }
        }

        while self.col > 0 {
            if self.value[self.row][self.col - 1].is_whitespace() {
                break;
            }
            self.set_cursor_column(self.col - 1);
        }
    }

    /// wordRight moves the cursor one word to the right.
    fn word_right(&mut self) {
        self.do_word_right(&mut |_, _| {});
    }

    fn do_word_right(&mut self, f: &mut dyn FnMut(usize, usize)) {
        // Skip spaces forward.
        while self.col >= self.value[self.row].len()
            || self.value[self.row][self.col].is_whitespace()
        {
            if self.row == self.value.len() - 1 && self.col == self.value[self.row].len() {
                // End of text.
                break;
            }
            self.character_right();
        }

        let mut char_idx = 0;
        while self.col < self.value[self.row].len() {
            if self.value[self.row][self.col].is_whitespace() {
                break;
            }
            f(char_idx, self.col);
            self.set_cursor_column(self.col + 1);
            char_idx += 1;
        }
    }

    /// uppercaseRight changes the word to the right to uppercase.
    fn uppercase_right(&mut self) {
        let idxs: Vec<usize> = self.collect_word_right_indices();
        for i in idxs {
            self.value[self.row][i] = self.value[self.row][i].to_uppercase().next().unwrap();
        }
    }

    /// lowercaseRight changes the word to the right to lowercase.
    fn lowercase_right(&mut self) {
        let idxs: Vec<usize> = self.collect_word_right_indices();
        for i in idxs {
            self.value[self.row][i] = self.value[self.row][i].to_lowercase().next().unwrap();
        }
    }

    /// capitalizeRight changes the word to the right to title case.
    fn capitalize_right(&mut self) {
        let idxs: Vec<usize> = self.collect_word_right_indices();
        for (char_idx, i) in idxs.iter().enumerate() {
            if char_idx == 0 {
                self.value[self.row][*i] = self.value[self.row][*i].to_uppercase().next().unwrap();
            }
        }
    }

    fn collect_word_right_indices(&mut self) -> Vec<usize> {
        let mut idxs = vec![];
        self.do_word_right(&mut |_, i| idxs.push(i));
        idxs
    }

    /// LineInfo returns the number of characters from the start of the
    /// (soft-wrapped) line and the (soft-wrapped) line width.
    pub fn line_info(&self) -> LineInfo {
        let grid = self.memoized_wrap(&self.value[self.row], self.width);

        // Find out which line we are currently on. This can be determined
        // by the m.col and counting the number of runes that we need to
        // skip.
        let mut counter = 0;
        for (i, line) in grid.iter().enumerate() {
            // We've found the line that we are on
            if counter + line.len() == self.col && i + 1 < grid.len() {
                // We wrap around to the next line if we are at the end of
                // the previous line so that we can be at the very beginning
                // of the row.
                return LineInfo {
                    char_offset: 0,
                    column_offset: 0,
                    height: grid.len(),
                    row_offset: i + 1,
                    start_column: self.col,
                    width: grid[i + 1].len(),
                    char_width: string_width(&String::from_iter(line.iter())),
                };
            }

            if counter + line.len() >= self.col {
                return LineInfo {
                    char_offset: string_width(&String::from_iter(
                        line[..self.col.saturating_sub(counter)].iter(),
                    )),
                    column_offset: self.col - counter,
                    height: grid.len(),
                    row_offset: i,
                    start_column: counter,
                    width: line.len(),
                    char_width: string_width(&String::from_iter(line.iter())),
                };
            }

            counter += line.len();
        }
        LineInfo {
            width: 0,
            char_width: 0,
            height: 0,
            start_column: 0,
            column_offset: 0,
            row_offset: 0,
            char_offset: 0,
        }
    }

    /// repositionView repositions the view of the viewport based on the
    /// defined scrolling behavior.
    fn reposition_view(&mut self) {
        let minimum = self.viewport.y_offset();
        let maximum = minimum + self.viewport.height() - 1;
        let row = self.cursor_line_number();
        if row < minimum {
            self.viewport.scroll_up(minimum - row);
        } else if row > maximum {
            self.viewport.scroll_down(row - maximum);
        }
    }

    /// Width returns the width of the textarea.
    pub fn width(&self) -> usize {
        self.width
    }

    /// MoveToBegin moves the cursor to the beginning of the input.
    pub fn move_to_begin(&mut self) {
        self.row = 0;
        self.set_cursor_column(0);
        self.reposition_view();
    }

    /// MoveToEnd moves the cursor to the end of the input.
    pub fn move_to_end(&mut self) {
        self.row = self.value.len() - 1;
        self.set_cursor_column(self.value[self.row].len());
        self.reposition_view();
    }

    /// PageUp moves the cursor up by one page. First call snaps to the
    /// first visible line, subsequent calls move up by a full page.
    pub fn page_up(&mut self) {
        // If not on the first visible line, snap to it.
        let offset = self.viewport.y_offset() as isize - self.cursor_line_number() as isize;
        if offset < 0 {
            self.set_cursor_line_relative(offset);
            return;
        }

        // Already on first visible line, move up by a full page.
        self.set_cursor_line_relative(-(self.height as isize));
    }

    /// PageDown moves the cursor down by one page. First call snaps to the
    /// last visible line, subsequent calls move down by a full page.
    pub fn page_down(&mut self) {
        // If not on the last visible line, snap to it.
        let offset = self.cursor_line_number() as isize - self.viewport.y_offset() as isize;
        if offset < (self.height - 1) as isize {
            self.set_cursor_line_relative((self.height - 1) as isize - offset);
            return;
        }

        // Already on last visible line, move down by a full page.
        self.set_cursor_line_relative(self.height as isize);
    }

    /// SetWidth sets the width of the textarea to fit exactly within the
    /// given width.
    pub fn set_width(&mut self, w: usize) {
        // Update prompt width only if there is no prompt function.
        if self.prompt_func.is_none() {
            self.prompt_width = string_width(&self.prompt);
        }

        // Add base style borders and padding to reserved outer width.
        let reserved_outer = self.active_style().base.get_horizontal_frame_size();

        // Add prompt width to reserved inner width.
        let mut reserved_inner = self.prompt_width;

        // Add line number width to reserved inner width.
        if self.show_line_numbers {
            // XXX: this was originally documented as needing "1 cell" but
            // was, in practice, effectively hardcoded to 2 cells.
            const GAP: usize = 2;

            // Number of digits plus 1 cell for the margin.
            reserved_inner += num_digits(self.max_height) + GAP;
        }

        // Input width must be at least one more than the reserved inner and
        // outer width. This gives us a minimum input width of 1.
        let min_width = reserved_inner + reserved_outer + 1;
        let mut input_width = w.max(min_width);

        // Input width must be no more than maximum width.
        if self.max_width > 0 {
            input_width = input_width.min(self.max_width);
        }

        // Since the width of the viewport and input area is dependent on the
        // width of borders, prompt and line numbers, we need to calculate it
        // by subtracting the reserved width from them.
        self.viewport.set_width(input_width - reserved_outer);
        self.width = input_width - reserved_outer - reserved_inner;
        self.recalculate_height();
    }

    /// SetPromptFunc supersedes the Prompt field and sets a dynamic prompt
    /// instead.
    pub fn set_prompt_func(
        &mut self,
        prompt_width: usize,
        f: Box<dyn Fn(PromptInfo) -> String + Send + Sync>,
    ) {
        self.prompt_func = Some(f);
        self.prompt_width = prompt_width;
    }

    /// Height returns the current height of the textarea.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Cursor returns a real cursor for rendering in a Bubble Tea program.
    /// This requires that [`use_virtual_cursor`](Self::use_virtual_cursor) is
    /// false and the textarea is focused.
    pub fn cursor(&self) -> Option<rusty_bubbletea::cursor::Cursor> {
        if self.use_virtual_cursor || !self.focus {
            return None;
        }

        let li = self.line_info();
        let base_style = &self.active_style().base;

        let x_offset = li.char_offset
            + self.prompt_width
            + self.line_number_width()
            + base_style.get_margin_left()
            + base_style.get_padding_left()
            + base_style.get_border_left_size();

        let y_offset = self
            .cursor_line_number()
            .saturating_sub(self.viewport.y_offset())
            + base_style.get_margin_top()
            + base_style.get_padding_top()
            + base_style.get_border_top_size();

        let style = &self.styles.cursor;
        let mut c = rusty_bubbletea::cursor::Cursor::new(x_offset, y_offset);
        c.blink = style.blink;
        // The cursor color: upstream stores a color.Color; the bubbletea
        // Cursor expects an RGBColor — convert from the style color.
        let (r, g, b, _) = style.color.rgba_bytes();
        c.color = Some(rusty_x_ansi::color::RGBColor { r, g, b });
        c.shape = style.shape;
        Some(c)
    }

    /// lineNumberWidth returns the width reserved for the line numbers,
    /// mirroring the upstream `LineNumberView` width calculation.
    fn line_number_width(&self) -> usize {
        if !self.show_line_numbers {
            return 0;
        }
        // Number of digits plus one cell for each margin.
        num_digits(self.max_height) + 2
    }

    /// SetHeight sets the height of the textarea.
    pub fn set_height(&mut self, h: usize) {
        if self.max_height > 0 {
            self.height = clamp(h, MIN_HEIGHT, self.max_height);
            self.viewport
                .set_height(clamp(h, MIN_HEIGHT, self.max_height));
        } else {
            self.height = h.max(MIN_HEIGHT);
            self.viewport.set_height(h.max(MIN_HEIGHT));
        }

        self.reposition_view();
    }

    /// Update is the Bubble Tea update loop.
    pub fn update(&mut self, msg: &dyn Msg) -> Cmd {
        if !self.focus {
            self.virtual_cursor.blur();
            return None;
        }

        // Used to determine if the cursor should blink.
        let (old_row, old_col) = (self.cursor_line_number(), self.col);

        let mut cmds: Vec<Cmd> = Vec::new();

        if self.value[self.row].is_empty() && self.value[self.row].is_empty() {
            // (no-op guard; value rows are always allocated)
        }

        if self.max_height > 0 && self.max_height != self.cache.capacity() {
            self.cache = memoization::new_memo_cache(self.max_height);
        }

        if let Some(pm) = msg.as_any().downcast_ref::<PasteMsg>() {
            self.insert_runes_from_user_input(&pm.content.chars().collect::<Vec<char>>());
        }

        if let Some(m) = msg.as_any().downcast_ref::<KeyPressMsg>() {
            let k = &m.0;
            if key::matches(k, std::slice::from_ref(&self.key_map.delete_after_cursor)) {
                self.col = clamp(self.col, 0, self.value[self.row].len());
                if self.col >= self.value[self.row].len() {
                    self.merge_line_below(self.row);
                } else {
                    self.delete_after_cursor();
                }
            } else if key::matches(k, std::slice::from_ref(&self.key_map.delete_before_cursor)) {
                self.col = clamp(self.col, 0, self.value[self.row].len());
                if self.col == 0 {
                    self.merge_line_above(self.row);
                } else {
                    self.delete_before_cursor();
                }
            } else if key::matches(
                k,
                std::slice::from_ref(&self.key_map.delete_character_backward),
            ) {
                self.col = clamp(self.col, 0, self.value[self.row].len());
                if self.col == 0 {
                    self.merge_line_above(self.row);
                } else if !self.value[self.row].is_empty() {
                    let mut v = self.value[self.row][..self.col.max(1) - 1].to_vec();
                    v.extend_from_slice(&self.value[self.row][self.col..]);
                    self.value[self.row] = v;
                    if self.col > 0 {
                        self.set_cursor_column(self.col - 1);
                    }
                }
            } else if key::matches(
                k,
                std::slice::from_ref(&self.key_map.delete_character_forward),
            ) {
                if !self.value[self.row].is_empty() && self.col < self.value[self.row].len() {
                    self.value[self.row].remove(self.col);
                }
                if self.col >= self.value[self.row].len() {
                    self.merge_line_below(self.row);
                }
            } else if key::matches(k, std::slice::from_ref(&self.key_map.delete_word_backward)) {
                if self.col == 0 {
                    self.merge_line_above(self.row);
                } else {
                    self.delete_word_left();
                }
            } else if key::matches(k, std::slice::from_ref(&self.key_map.delete_word_forward)) {
                self.col = clamp(self.col, 0, self.value[self.row].len());
                if self.col >= self.value[self.row].len() {
                    self.merge_line_below(self.row);
                } else {
                    self.delete_word_right();
                }
            } else if key::matches(k, std::slice::from_ref(&self.key_map.insert_newline)) {
                if self.at_content_limit() {
                    return None;
                }
                self.col = clamp(self.col, 0, self.value[self.row].len());
                self.split_line(self.row, self.col);
            } else if key::matches(k, std::slice::from_ref(&self.key_map.line_end)) {
                self.cursor_end();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.line_start)) {
                self.cursor_start();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.character_forward)) {
                self.character_right();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.line_next)) {
                self.cursor_down();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.word_forward)) {
                self.word_right();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.paste)) {
                return self.paste_cmd();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.character_backward)) {
                self.character_left(false /* insideLine */);
            } else if key::matches(k, std::slice::from_ref(&self.key_map.line_previous)) {
                self.cursor_up();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.word_backward)) {
                self.word_left();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.input_begin)) {
                self.move_to_begin();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.input_end)) {
                self.move_to_end();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.page_up)) {
                self.page_up();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.page_down)) {
                self.page_down();
            } else if key::matches(
                k,
                std::slice::from_ref(&self.key_map.lowercase_word_forward),
            ) {
                self.lowercase_right();
            } else if key::matches(
                k,
                std::slice::from_ref(&self.key_map.uppercase_word_forward),
            ) {
                self.uppercase_right();
            } else if key::matches(
                k,
                std::slice::from_ref(&self.key_map.capitalize_word_forward),
            ) {
                self.capitalize_right();
            } else if key::matches(
                k,
                std::slice::from_ref(&self.key_map.transpose_character_backward),
            ) {
                self.transpose_left();
            } else {
                self.insert_runes_from_user_input(&k.text.chars().collect::<Vec<char>>());
            }
        }

        if let Some(pm) = msg.as_any().downcast_ref::<PasteMsgInternal>() {
            self.insert_runes_from_user_input(&pm.0.chars().collect::<Vec<char>>());
        }

        if let Some(pm) = msg.as_any().downcast_ref::<PasteErrMsg>() {
            self.err = Some(pm.0.clone());
        }

        self.recalculate_height();

        // Make sure we set the content of the viewport before updating it.
        let view = self.view_inner();
        self.viewport.set_content(&view);
        let vp_cmd = self.viewport.update(msg);
        cmds.push(vp_cmd);

        if self.use_virtual_cursor {
            let cmd = self.virtual_cursor.update(msg);
            let mut cmd = cmd;

            // If the cursor has moved, reset the blink state. This is a
            // small UX nuance that makes cursor movement obvious and feel
            // snappy.
            let (new_row, new_col) = (self.cursor_line_number(), self.col);
            if (new_row != old_row || new_col != old_col)
                && self.virtual_cursor.mode() == cursor::Mode::Blink
            {
                self.virtual_cursor.is_blinked = false;
                cmd = self.virtual_cursor.blink();
            }
            cmds.push(cmd);
        }

        self.reposition_view();

        rusty_bubbletea::commands::batch(cmds)
    }

    fn view_inner(&self) -> String {
        if self.value().is_empty() && self.row == 0 && self.col == 0 && !self.placeholder.is_empty()
        {
            return self.placeholder_view();
        }
        self.view_content()
    }

    fn view_content(&self) -> String {
        let mut s = String::new();
        let styles = self.active_style();
        // Mirror the upstream `m.virtualCursor.TextStyle =
        // m.activeStyle().computedCursorLine()` at the top of `view()`: the
        // blink state of the virtual cursor renders with the cursor-line
        // style.
        let mut vc = self.virtual_cursor.clone();
        vc.text_style = styles.computed_cursor_line();
        let mut new_lines = 0usize;
        let mut widest_line_number = 0usize;
        let line_info = self.line_info();
        let mut display_line = 0usize;
        for (l, line) in self.value.iter().enumerate() {
            let wrapped_lines = self.memoized_wrap(line, self.width);

            let style = if self.row == l {
                styles.computed_cursor_line()
            } else {
                styles.computed_text()
            };

            for (wl, wrapped_line) in wrapped_lines.iter().enumerate() {
                let mut prompt = self.prompt_view(display_line);
                prompt = styles.computed_prompt().render(&prompt);
                s += &style.render(&prompt);
                display_line += 1;

                let ln = String::new();
                if self.show_line_numbers {
                    if wl == 0 {
                        // normal line
                        let is_cursor_line = self.row == l;
                        s += &self.line_number_view((l + 1) as isize, is_cursor_line);
                    } else {
                        // soft wrapped line
                        let is_cursor_line = self.row == l;
                        s += &self.line_number_view(-1, is_cursor_line);
                    }
                }

                // Note the widest line number for padding purposes later.
                // Upstream declares `var ln string` but never assigns it, so
                // the widest line number stays 0; mirror that.
                let lnw = string_width(&ln);
                if lnw > widest_line_number {
                    widest_line_number = lnw;
                }

                let mut wrapped_line = wrapped_line.clone();
                let strwidth = string_width(&String::from_iter(wrapped_line.iter()));
                let mut padding = self.width - strwidth;
                // If the trailing space causes the line to be wider than the
                // width, we should not draw it to the screen.
                if strwidth > self.width {
                    // The character causing the line to be wider than the
                    // width is guaranteed to be a space.
                    while wrapped_line.last() == Some(&' ') {
                        wrapped_line.pop();
                    }
                    padding = padding.saturating_sub(self.width - strwidth);
                }
                if self.row == l && line_info.row_offset == wl {
                    s += &style.render(&String::from_iter(
                        wrapped_line[..line_info.column_offset.min(wrapped_line.len())].iter(),
                    ));
                    if self.col >= line.len() && line_info.char_offset >= self.width {
                        vc.set_char(" ");
                        s += &vc.view();
                    } else {
                        let col = line_info.column_offset.min(wrapped_line.len());
                        let ch = if col < wrapped_line.len() {
                            String::from_iter(wrapped_line[col..col + 1].iter())
                        } else {
                            String::new()
                        };
                        vc.set_char(&ch);
                        s += &style.render(&vc.view());
                        s += &style.render(&String::from_iter(wrapped_line[col + 1..].iter()));
                    }
                } else {
                    s += &style.render(&String::from_iter(wrapped_line.iter()));
                }
                s += &style.render(&" ".repeat(padding));
                s += "\n";
                new_lines += 1;
            }
        }

        // Always show at least `m.height` lines at all times.
        // To do this we can simply pad out a few extra new lines in the
        // view.
        for _ in 0..self.height {
            let prompt = self.prompt_view(display_line);
            s += &prompt;
            display_line += 1;

            // Write end of buffer content
            let left_gutter = self.end_of_buffer_character.to_string();
            let right_gap_width =
                self.width().saturating_sub(string_width(&left_gutter)) + widest_line_number;
            let right_gap = " ".repeat(right_gap_width);
            s += &styles
                .computed_end_of_buffer()
                .render(&(left_gutter + &right_gap));
            s += "\n";
        }

        let _ = new_lines;
        s
    }

    /// View renders the text area in its current state.
    pub fn view(&self) -> String {
        // XXX: This is a workaround for the case where the viewport hasn't
        // been initialized yet like during the initial render.
        let mut viewport = self.viewport.clone();
        viewport.set_content(&self.view_inner());
        let view = viewport.view();
        let styles = self.active_style();
        styles.base.clone().render(&view)
    }

    /// promptView renders a single line of the prompt.
    pub fn prompt_view(&self, display_line: usize) -> String {
        let mut prompt = self.prompt.clone();
        if let Some(f) = &self.prompt_func {
            prompt = f(PromptInfo {
                line_number: display_line,
                focused: self.focus,
            });
            let width = rusty_lipgloss::size::width(&prompt);
            if width < self.prompt_width {
                prompt = format!("{}{}", " ".repeat(self.prompt_width - width), prompt);
            }
        }

        prompt
    }

    /// lineNumberView renders the line number.
    fn line_number_view(&self, n: isize, is_cursor_line: bool) -> String {
        if !self.show_line_numbers {
            return String::new();
        }

        let mut str_: String;
        if n <= 0 {
            str_ = " ".to_string();
        } else {
            str_ = n.to_string();
        }

        // XXX: is textStyle really necessary here?
        let mut text_style = self.active_style().computed_text();
        let mut line_number_style = self.active_style().computed_line_number();
        if is_cursor_line {
            text_style = self.active_style().computed_cursor_line();
            line_number_style = self.active_style().computed_cursor_line_number();
        }

        // Format line number dynamically based on the maximum number of
        // lines.
        let digits = num_digits(self.max_height);
        str_ = format!(" {:>width$} ", str_, width = digits);

        text_style.render(&line_number_style.render(&str_))
    }

    /// placeholderView returns the prompt and placeholder, if any.
    fn placeholder_view(&self) -> String {
        let mut s = String::new();
        let p = self.placeholder.clone();
        let styles = self.active_style();
        // word wrap lines
        let pwordwrap = wordwrap(&p, self.width, "");
        // hard wrap lines (handles lines that could not be word wrapped)
        let pwrap = hardwrap(&pwordwrap, self.width, true);
        // split string by new lines
        let plines: Vec<String> = pwrap.trim().split('\n').map(|x| x.to_string()).collect();

        for i in 0..self.height {
            let is_line_number = plines.len() > i;

            let mut line_style = styles.computed_placeholder();
            if plines.len() > i {
                line_style = styles.computed_cursor_line();
            }

            // render prompt
            let prompt = self.prompt_view(i);
            let prompt = styles.computed_prompt().render(&prompt);
            s += &line_style.render(&prompt);

            // when show line numbers enabled: render line number for only
            // the cursor line; indent other placeholder lines.
            if self.show_line_numbers {
                let mut ln = 0isize;

                match i {
                    0 => {
                        ln = (i + 1) as isize;
                        if plines.len() > i {
                            s += &self.line_number_view(ln, is_line_number);
                        }
                    }
                    _ => {
                        if plines.len() > i {
                            s += &self.line_number_view(ln, is_line_number);
                        }
                    }
                }
            }

            match i {
                // first line
                0 => {
                    // first character of first line as cursor with character
                    let mut vc = self.virtual_cursor.clone();
                    vc.text_style = styles.computed_placeholder();

                    let ch = plines[0].chars().next().unwrap_or(' ');
                    let rest: String = plines[0].chars().skip(1).collect();
                    vc.set_char(&ch.to_string());
                    s += &line_style.render(&vc.view());

                    // the rest of the first line
                    s += &line_style.render(&styles.computed_placeholder().render(&rest));

                    // extend the first line with spaces to fill the width
                    let gap = " ".repeat(
                        self.width
                            .saturating_sub(rusty_lipgloss::size::width(&plines[0])),
                    );
                    s += &line_style.render(&gap);
                }
                // remaining lines
                _ => {
                    if plines.len() > i {
                        // current line placeholder text
                        let placeholder_line = &plines[i];
                        let gap = " ".repeat(self.width.saturating_sub(string_width(&plines[i])));
                        s += &line_style.render(&(placeholder_line.clone() + &gap));
                    } else {
                        // end of line buffer character
                        let eob = styles
                            .computed_end_of_buffer()
                            .render(&self.end_of_buffer_character.to_string());
                        s += &eob;
                    }
                }
            }

            // terminate with new line
            s += "\n";
        }

        let mut viewport = self.viewport.clone();
        viewport.set_content(&s);
        let v = viewport.view();
        styles.base.clone().render(&v)
    }

    fn memoized_wrap(&self, runes: &[char], width: usize) -> Vec<Vec<char>> {
        // The cache is keyed by content hash; only used when the model is
        // mutable. For &self access we compute directly.
        let _ = runes;
        let _ = width;
        // Note: upstream memoizes via a mutable cache; this port computes
        // the wrap on demand to keep LineInfo usable through &self.
        let _ = &self.cache;
        wrap(runes, width)
    }

    /// cursorLineNumber returns the line number that the cursor is on.
    /// This accounts for soft wrapped lines.
    pub fn cursor_line_number(&self) -> usize {
        let mut line = 0;
        for i in 0..self.row {
            // Calculate the number of lines that the current line will be
            // split into.
            line += self.memoized_wrap(&self.value[i], self.width).len();
        }
        line + self.line_info().row_offset
    }

    /// TotalVisualLines returns the total number of display lines across
    /// all logical lines, accounting for soft wraps.
    pub fn total_visual_lines(&self) -> usize {
        let mut n = 0;
        for line in &self.value {
            n += self.memoized_wrap(line, self.width).len();
        }
        n
    }

    /// recalculateHeight recomputes and applies the textarea height based
    /// on content when DynamicHeight is enabled. It is a no-op otherwise.
    fn recalculate_height(&mut self) {
        if !self.dynamic_height {
            return;
        }
        let min_h = self.min_height.max(MIN_HEIGHT);
        let total = self.total_visual_lines();
        let mut h = total.max(min_h);
        if self.max_height > 0 {
            h = h.min(self.max_height);
        }
        let max_offset = total.saturating_sub(h);
        if self.viewport.y_offset() > max_offset {
            self.viewport.set_y_offset(max_offset);
        }
        self.set_height(h);
    }

    /// atContentLimit reports whether the textarea has reached its content
    /// limit.
    fn at_content_limit(&self) -> bool {
        if self.max_content_height > 0 {
            return self.total_visual_lines() >= self.max_content_height;
        }
        self.max_height > 0 && self.value.len() >= self.max_height
    }

    /// visualLinesForInsert estimates how many additional visual lines
    /// would result from inserting the given lines at the current cursor
    /// position.
    fn visual_lines_for_insert(&self, lines: &[Vec<char>]) -> usize {
        if lines.is_empty() {
            return 0;
        }

        // The current row's visual line count before insertion.
        let current_row_visual = self.memoized_wrap(&self.value[self.row], self.width).len();

        // Simulate merging the first paste line into the current row.
        let mut merged: Vec<char> = self.value[self.row][..self.col].to_vec();
        merged.extend_from_slice(&lines[0]);
        if lines.len() == 1 {
            merged.extend_from_slice(&self.value[self.row][self.col..]);
        }
        let delta = self.memoized_wrap(&merged, self.width).len() - current_row_visual;

        // Each additional line is a new logical line.
        let mut delta = delta;
        for (i, content) in lines.iter().enumerate() {
            let mut content = content.clone();
            if i == lines.len() - 1 {
                content.extend_from_slice(&self.value[self.row][self.col..]);
            }
            delta += self.memoized_wrap(&content, self.width).len();
        }

        delta
    }

    /// mergeLineBelow merges the current line the cursor is on with the
    /// line below.
    fn merge_line_below(&mut self, row: usize) {
        if row >= self.value.len() - 1 {
            return;
        }

        // To perform a merge, we will need to combine the two lines.
        let mut merged = self.value[row].clone();
        merged.extend_from_slice(&self.value[row + 1]);
        self.value[row] = merged;

        // Shift all lines up by one.
        for i in row + 1..self.value.len() - 1 {
            self.value[i] = self.value[i + 1].clone();
        }

        // And, remove the last line.
        if !self.value.is_empty() {
            self.value.pop();
        }
    }

    /// mergeLineAbove merges the current line the cursor is on with the
    /// line above.
    fn merge_line_above(&mut self, row: usize) {
        if row == 0 {
            return;
        }

        self.col = self.value[row - 1].len();
        self.row -= 1;

        // To perform a merge, we will need to combine the two lines.
        let mut merged = self.value[row - 1].clone();
        merged.extend_from_slice(&self.value[row]);
        self.value[row - 1] = merged;

        // Shift all lines up by one.
        for i in row..self.value.len() - 1 {
            self.value[i] = self.value[i + 1].clone();
        }

        // And, remove the last line.
        if !self.value.is_empty() {
            self.value.pop();
        }
    }

    fn split_line(&mut self, row: usize, col: usize) {
        // To perform a split, take the current line and keep the content
        // before the cursor, take the content after the cursor and make it
        // the content of the line underneath, and shift the remaining lines
        // down by one.
        let head: Vec<char> = self.value[row][..col].to_vec();
        let tail: Vec<char> = self.value[row][col..].to_vec();

        self.value.insert(row + 1, tail);

        self.value[row] = head;

        self.col = 0;
        self.row += 1;
    }

    /// Paste is a command for pasting from the clipboard into the text
    /// input.
    fn paste_cmd(&self) -> Cmd {
        Some(Box::new(|| match clipboard::read_all() {
            Ok(str) => Some(Box::new(PasteMsgInternal(str))),
            Err(err) => Some(Box::new(PasteErrMsg(err))),
        }))
    }
}

/// Blink returns the blink command for the virtual cursor.
pub fn blink() -> Box<dyn Msg> {
    crate::cursor::blink()
}

fn wrap(runes: &[char], width: usize) -> Vec<Vec<char>> {
    let mut lines: Vec<Vec<char>> = vec![vec![]];
    let mut word: Vec<char> = vec![];
    let mut row = 0usize;
    let mut spaces = 0usize;

    // Word wrap the runes
    for r in runes {
        if r.is_whitespace() {
            spaces += 1;
        } else {
            word.push(*r);
        }

        if spaces > 0 {
            if string_width(&String::from_iter(lines[row].iter()))
                + string_width(&String::from_iter(word.iter()))
                + spaces
                > width
            {
                row += 1;
                lines.push(vec![]);
                lines[row].extend_from_slice(&word);
                lines[row].extend_from_slice(&repeat_spaces(spaces));
                spaces = 0;
                word.clear();
            } else {
                lines[row].extend_from_slice(&word);
                lines[row].extend_from_slice(&repeat_spaces(spaces));
                spaces = 0;
                word.clear();
            }
        } else if !word.is_empty() {
            // If the last character is a double-width rune, then we may not
            // be able to add it to this line as it might cause us to go past
            // the width.
            let last_char_len = char_width(*word.last().unwrap());
            if string_width(&String::from_iter(word.iter())) + last_char_len > width {
                // If the current line has any content, let's move to the
                // next line because the current word fills up the entire
                // line.
                if !lines[row].is_empty() {
                    row += 1;
                    lines.push(vec![]);
                }
                lines[row].extend_from_slice(&word);
                word.clear();
            }
        }
    }

    if string_width(&String::from_iter(lines[row].iter()))
        + string_width(&String::from_iter(word.iter()))
        + spaces
        >= width
    {
        lines.push(vec![]);
        lines[row + 1].extend_from_slice(&word);
        // We add an extra space at the end of the line to account for the
        // trailing space at the end of the previous soft-wrapped lines so
        // that behaviour when navigating is consistent.
        spaces += 1;
        lines[row + 1].extend_from_slice(&repeat_spaces(spaces));
    } else {
        lines[row].extend_from_slice(&word);
        spaces += 1;
        lines[row].extend_from_slice(&repeat_spaces(spaces));
    }

    lines
}

fn repeat_spaces(n: usize) -> Vec<char> {
    vec![' '; n]
}

/// numDigits returns the number of digits in an integer.
fn num_digits(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let mut count = 0;
    let mut num = n;
    while num > 0 {
        count += 1;
        num /= 10;
    }
    count
}

fn clamp(v: usize, low: usize, high: usize) -> usize {
    if high < low {
        return low;
    }
    v.max(low).min(high)
}

fn char_width(c: char) -> usize {
    UnicodeWidthChar::width(c).unwrap_or(0)
}

fn string_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

/// wordwrap wraps a string to a given line length without breaking
/// word boundaries (char-based port of `charmbracelet/x/ansi`'s `Wordwrap`).
fn wordwrap(s: &str, limit: usize, breakpoints: &str) -> String {
    if limit < 1 {
        return s.to_string();
    }

    let mut buf = String::new();
    let mut word = String::new();
    let mut space = String::new();
    let mut cur_width = 0usize;
    let mut word_len = 0usize;

    // addSpace mirrors the upstream helper: the pending space run is
    // written into the buffer.
    let add_space = |buf: &mut String, space: &mut String, cur_width: &mut usize| {
        *cur_width += space.len();
        buf.push_str(space);
        space.clear();
    };
    // addWord mirrors the upstream helper: flush the pending space run,
    // then the current word.
    let add_word = |buf: &mut String,
                    space: &mut String,
                    word: &mut String,
                    cur_width: &mut usize,
                    word_len: &mut usize| {
        if word.is_empty() {
            return;
        }
        add_space(buf, space, cur_width);
        *cur_width += *word_len;
        buf.push_str(word);
        word.clear();
        *word_len = 0;
    };
    let add_newline = |buf: &mut String, space: &mut String, cur_width: &mut usize| {
        buf.push('\n');
        *cur_width = 0;
        space.clear();
    };

    for c in s.chars() {
        if c == '\n' {
            if word_len == 0 {
                if cur_width + space.len() > limit {
                    cur_width = 0;
                } else {
                    buf.push_str(&space);
                }
                space.clear();
            }
            add_word(
                &mut buf,
                &mut space,
                &mut word,
                &mut cur_width,
                &mut word_len,
            );
            add_newline(&mut buf, &mut space, &mut cur_width);
        } else if c.is_whitespace() && c != '\u{00A0}' {
            add_word(
                &mut buf,
                &mut space,
                &mut word,
                &mut cur_width,
                &mut word_len,
            );
            space.push(c);
        } else if c == '-' || breakpoints.contains(c) {
            add_space(&mut buf, &mut space, &mut cur_width);
            add_word(
                &mut buf,
                &mut space,
                &mut word,
                &mut cur_width,
                &mut word_len,
            );
            buf.push(c);
            cur_width += 1;
        } else {
            word.push(c);
            word_len += char_width(c);
            if cur_width + space.len() + word_len > limit && word_len < limit {
                add_newline(&mut buf, &mut space, &mut cur_width);
            }
        }
    }

    add_word(
        &mut buf,
        &mut space,
        &mut word,
        &mut cur_width,
        &mut word_len,
    );
    buf
}

/// hardwrap wraps a string to a given line length, breaking word boundaries
/// (char-based port of `charmbracelet/x/ansi`'s `Hardwrap`).
fn hardwrap(s: &str, limit: usize, preserve_space: bool) -> String {
    if limit < 1 {
        return s.to_string();
    }

    let mut buf = String::new();
    let mut cur_width = 0usize;
    let mut force_newline = false;

    for c in s.chars() {
        if c == '\n' {
            buf.push('\n');
            cur_width = 0;
            force_newline = false;
            continue;
        }

        let w = char_width(c);
        if cur_width + w > limit {
            buf.push('\n');
            cur_width = 0;
            force_newline = true;
        }

        // Skip spaces at the beginning of a line.
        if cur_width == 0 {
            if !preserve_space && force_newline && c.is_whitespace() {
                continue;
            }
            force_newline = false;
        }

        buf.push(c);
        cur_width += w;
    }

    buf
}
