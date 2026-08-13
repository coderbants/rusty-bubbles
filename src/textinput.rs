//! Cleanroom Rust port of upstream Go source file: `textinput/textinput.go`
//! Cleanroom Rust port of upstream Go source file: `textinput/styles.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! <public-docs>
//! # TextInput
//!
//! A text input component for Bubble Tea applications.
//! </public-docs>

use crate::cursor;
use crate::internal::clipboard;
use crate::internal::runeutil::{self, Sanitizer};
use crate::key::{self, Binding};
use rusty_bubbletea::commands;
use rusty_bubbletea::cursor::CursorShape;
use rusty_bubbletea::key::{Key, KeyPressMsg};
use rusty_bubbletea::model::{Cmd, Msg};
use rusty_bubbletea::paste::PasteMsg;
use rusty_lipgloss::{self, Color, Style};
use std::time::Duration;
use unicode_width::UnicodeWidthChar;

/// Internal messages for clipboard operations.
#[derive(Debug)]
pub struct PasteMsgInternal(pub String);

#[derive(Debug)]
pub struct PasteErrMsg(pub String);

/// EchoMode sets the input behavior of the text input field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EchoMode {
    /// EchoNormal displays text as is. This is the default behavior.
    #[default]
    EchoNormal,

    /// EchoPassword displays the EchoCharacter mask instead of actual
    /// characters. This is commonly used for password fields.
    EchoPassword,

    /// EchoNone displays nothing as characters are entered. This is commonly
    /// seen for password fields on the command line.
    EchoNone,
}

/// ValidateFunc is a function that returns an error if the input is invalid.
pub type ValidateFunc = Box<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

/// KeyMap is the key bindings for different actions within the textinput.
#[derive(Debug, Clone)]
pub struct KeyMap {
    /// Move the cursor forward one character.
    pub character_forward: Binding,
    /// Move the cursor backward one character.
    pub character_backward: Binding,
    /// Move the cursor forward one word.
    pub word_forward: Binding,
    /// Move the cursor backward one word.
    pub word_backward: Binding,
    /// Delete the word backward.
    pub delete_word_backward: Binding,
    /// Delete the word forward.
    pub delete_word_forward: Binding,
    /// Delete after the cursor.
    pub delete_after_cursor: Binding,
    /// Delete before the cursor.
    pub delete_before_cursor: Binding,
    /// Delete the character backward.
    pub delete_character_backward: Binding,
    /// Delete the character forward.
    pub delete_character_forward: Binding,
    /// Go to the line start.
    pub line_start: Binding,
    /// Go to the line end.
    pub line_end: Binding,
    /// Paste from the clipboard.
    pub paste: Binding,
    /// Accept the current suggestion.
    pub accept_suggestion: Binding,
    /// Go to the next suggestion.
    pub next_suggestion: Binding,
    /// Go to the previous suggestion.
    pub prev_suggestion: Binding,
}

/// DefaultKeyMap is the default set of key bindings for navigating and
/// acting upon the textinput.
pub fn default_key_map() -> KeyMap {
    KeyMap {
        character_forward: key::new_binding(vec![key::with_keys(&["right", "ctrl+f"])]),
        character_backward: key::new_binding(vec![key::with_keys(&["left", "ctrl+b"])]),
        word_forward: key::new_binding(vec![key::with_keys(&["alt+right", "ctrl+right", "alt+f"])]),
        word_backward: key::new_binding(vec![key::with_keys(&["alt+left", "ctrl+left", "alt+b"])]),
        delete_word_backward: key::new_binding(vec![key::with_keys(&["alt+backspace", "ctrl+w"])]),
        delete_word_forward: key::new_binding(vec![key::with_keys(&["alt+delete", "alt+d"])]),
        delete_after_cursor: key::new_binding(vec![key::with_keys(&["ctrl+k"])]),
        delete_before_cursor: key::new_binding(vec![key::with_keys(&["ctrl+u"])]),
        delete_character_backward: key::new_binding(vec![key::with_keys(&["backspace", "ctrl+h"])]),
        delete_character_forward: key::new_binding(vec![key::with_keys(&["delete", "ctrl+d"])]),
        line_start: key::new_binding(vec![key::with_keys(&["home", "ctrl+a"])]),
        line_end: key::new_binding(vec![key::with_keys(&["end", "ctrl+e"])]),
        paste: key::new_binding(vec![key::with_keys(&["ctrl+v"])]),
        accept_suggestion: key::new_binding(vec![key::with_keys(&["tab"])]),
        next_suggestion: key::new_binding(vec![key::with_keys(&["down", "ctrl+n"])]),
        prev_suggestion: key::new_binding(vec![key::with_keys(&["up", "ctrl+p"])]),
    }
}

/// Model is the Bubble Tea model for this text input element.
pub struct Model {
    /// The validation error, if any.
    pub err: Option<String>,

    /// General settings.
    /// The prompt shown before the input.
    pub prompt: String,
    /// The placeholder shown when the input is empty.
    pub placeholder: String,
    /// The echo mode of the input.
    pub echo_mode: EchoMode,
    /// The character used for masking in [`EchoMode::EchoPassword`].
    pub echo_character: char,

    /// use_virtual_cursor determines whether or not to use the virtual
    /// cursor.
    pub use_virtual_cursor: bool,

    /// Virtual cursor manager.
    pub virtual_cursor: cursor::Model,

    /// CharLimit is the maximum amount of characters this input element will
    /// accept. If 0 or less, there's no limit.
    pub char_limit: usize,

    /// Styling. FocusedStyle and BlurredStyle are used to style the textarea
    /// in focused and blurred states.
    pub styles: Styles,

    /// Width is the maximum number of characters that can be displayed at
    /// once. It essentially treats the text field like a horizontally
    /// scrolling viewport. If 0 or less this setting is ignored.
    pub width: usize,

    /// KeyMap encodes the keybindings recognized by the widget.
    pub key_map: KeyMap,

    /// Underlying text value.
    value: Vec<char>,

    /// focus indicates whether user input focus should be on this input
    /// component. When false, ignore keyboard input and hide the cursor.
    pub focus: bool,

    /// Cursor position.
    pos: usize,

    /// Used to emulate a viewport when width is set and the content is
    /// overflowing.
    offset: usize,
    offset_right: usize,

    /// Validate is a function that checks whether or not the text within the
    /// input is valid. If it is not valid, the `Err` field will be set to the
    /// error returned by the function.
    pub validate: Option<ValidateFunc>,

    /// rune sanitizer for input.
    rsan: Option<runeutil::Sanitizer_>,

    /// Should the input suggest to complete.
    pub show_suggestions: bool,

    /// suggestions is a list of suggestions that may be used to complete the
    /// input.
    suggestions: Vec<Vec<char>>,
    matched_suggestions: Vec<Vec<char>>,
    current_suggestion_index: usize,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("textinput::Model")
            .field("value", &String::from_iter(self.value.iter()))
            .field("focus", &self.focus)
            .field("pos", &self.pos)
            .finish()
    }
}

/// New creates a new model with default settings.
pub fn new() -> Model {
    let mut m = Model {
        prompt: "> ".to_string(),
        echo_character: '*',
        char_limit: 0,
        styles: default_dark_styles(),
        show_suggestions: false,
        use_virtual_cursor: true,
        virtual_cursor: cursor::new(),
        key_map: default_key_map(),
        suggestions: vec![],
        value: vec![],
        focus: false,
        pos: 0,
        placeholder: String::new(),
        echo_mode: EchoMode::EchoNormal,
        err: None,
        width: 0,
        validate: None,
        rsan: None,
        offset: 0,
        offset_right: 0,
        matched_suggestions: vec![],
        current_suggestion_index: 0,
    };
    m.update_virtual_cursor_style();
    m
}

impl Model {
    /// VirtualCursor returns whether the model is using a virtual cursor.
    pub fn virtual_cursor(&self) -> bool {
        self.use_virtual_cursor
    }

    /// SetVirtualCursor sets whether the model should use a virtual cursor.
    pub fn set_virtual_cursor(&mut self, v: bool) {
        self.use_virtual_cursor = v;
        self.update_virtual_cursor_style();
    }

    /// Styles returns the current set of styles.
    pub fn styles(&self) -> &Styles {
        &self.styles
    }

    /// SetStyles sets the styles for the text input.
    pub fn set_styles(&mut self, s: Styles) {
        self.styles = s;
        self.update_virtual_cursor_style();
    }

    /// Cursor returns a real cursor for rendering in a Bubble Tea program.
    /// This requires that [`use_virtual_cursor`](Self::use_virtual_cursor) is
    /// false.
    pub fn cursor(&self) -> Option<rusty_bubbletea::cursor::Cursor> {
        if self.use_virtual_cursor || !self.focus {
            return None;
        }

        let prompt_width = rusty_lipgloss::size::width(&self.prompt_view());
        let mut x_offset = self.pos + prompt_width;
        if self.width > 0 {
            x_offset = x_offset.min(self.width + prompt_width);
        }

        let style = &self.styles.cursor;
        let mut c = rusty_bubbletea::cursor::Cursor::new(x_offset, 0);
        c.blink = style.blink;
        // The cursor color: upstream stores a color.Color; the bubbletea
        // Cursor expects an RGBColor — convert from the style color.
        let (r, g, b, _) = style.color.rgba_bytes();
        c.color = Some(rusty_x_ansi::color::RGBColor { r, g, b });
        c.shape = style.shape;
        Some(c)
    }

    /// Width returns the width of the text input.
    pub fn width(&self) -> usize {
        self.width
    }

    /// SetWidth sets the width of the text input.
    pub fn set_width(&mut self, w: usize) {
        self.width = w;
    }

    /// SetValue sets the value of the text input.
    pub fn set_value(&mut self, s: &str) {
        // Clean up any special characters in the input provided by the
        // caller. This avoids bugs due to e.g. tab characters and whatnot.
        let runes = self.san().sanitize(&s.chars().collect::<Vec<char>>());
        let err = self.validate(&runes);
        self.set_value_internal(runes, err);
    }

    fn set_value_internal(&mut self, runes: Vec<char>, err: Option<String>) {
        self.err = err;

        let empty = self.value.is_empty();

        if self.char_limit > 0 && runes.len() > self.char_limit {
            self.value = runes[..self.char_limit].to_vec();
        } else {
            self.value = runes;
        }
        if (self.pos == 0 && empty) || self.pos > self.value.len() {
            self.set_cursor(self.value.len());
        }
        self.handle_overflow();
    }

    /// Value returns the value of the text input.
    pub fn value(&self) -> String {
        String::from_iter(self.value.iter())
    }

    /// Position returns the cursor position.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// SetCursor moves the cursor to the given position. If the position is
    /// out of bounds the cursor will be moved to the start or end
    /// accordingly.
    pub fn set_cursor(&mut self, pos: usize) {
        self.pos = clamp(pos, 0, self.value.len());
        self.handle_overflow();
    }

    /// CursorStart moves the cursor to the start of the input field.
    pub fn cursor_start(&mut self) {
        self.set_cursor(0);
    }

    /// CursorEnd moves the cursor to the end of the input field.
    pub fn cursor_end(&mut self) {
        self.set_cursor(self.value.len());
    }

    /// Focused returns the focus state on the model.
    pub fn focused(&self) -> bool {
        self.focus
    }

    /// Focus sets the focus state on the model. When the model is in focus
    /// it can receive keyboard input and the cursor will be shown.
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
        self.value = vec![];
        self.set_cursor(0);
    }

    /// SetSuggestions sets the suggestions for the input.
    pub fn set_suggestions(&mut self, suggestions: &[String]) {
        self.suggestions = suggestions.iter().map(|s| s.chars().collect()).collect();

        self.update_suggestions();
    }

    /// rsan initializes or retrieves the rune sanitizer.
    fn san(&mut self) -> &runeutil::Sanitizer_ {
        if self.rsan.is_none() {
            // Textinput has all its input on a single line so collapse
            // newlines/tabs to single spaces.
            self.rsan = Some(runeutil::new_sanitizer(vec![
                runeutil::replace_tabs(" "),
                runeutil::replace_newlines(" "),
            ]));
        }
        self.rsan.as_ref().unwrap()
    }

    fn insert_ranes_from_user_input(&mut self, v: &[char]) {
        // Clean up any special characters in the input provided by the
        // clipboard. This avoids bugs due to e.g. tab characters and whatnot.
        let mut paste = self.san().sanitize(v);

        let mut avail_space: usize = 0;
        if self.char_limit > 0 {
            avail_space = self.char_limit - self.value.len();

            // If the char limit's been reached, cancel.
            if avail_space == 0 {
                return;
            }

            // If there's not enough space to paste the whole thing cut the
            // pasted runes down so they'll fit.
            if avail_space < paste.len() {
                paste.truncate(avail_space);
            }
        }

        // Stuff before and after the cursor
        let mut head: Vec<char> = self.value[..self.pos].to_vec();
        let tail: Vec<char> = self.value[self.pos..].to_vec();

        // Insert pasted runes
        for r in paste {
            head.push(r);
            self.pos += 1;
            if self.char_limit > 0 {
                avail_space -= 1;
                if avail_space == 0 {
                    break;
                }
            }
        }

        // Put it all back together
        let mut value = head;
        value.extend_from_slice(&tail);
        let input_err = self.validate(&value);
        self.set_value_internal(value, input_err);
    }

    /// If a max width is defined, perform some logic to treat the visible
    /// area as a horizontally scrolling viewport.
    fn handle_overflow(&mut self) {
        if self.width() == 0 || string_width(&String::from_iter(self.value.iter())) <= self.width()
        {
            self.offset = 0;
            self.offset_right = self.value.len();
            return;
        }

        // Correct right offset if we've deleted characters
        self.offset_right = self.offset_right.min(self.value.len());

        if self.pos < self.offset {
            self.offset = self.pos;

            let mut w = 0;
            let mut i = 0;
            let runes = &self.value[self.offset..];

            while i < runes.len() && w <= self.width() {
                w += rune_width(runes[i]);
                if w <= self.width() + 1 {
                    i += 1;
                }
            }

            self.offset_right = self.offset + i;
        } else if self.pos >= self.offset_right {
            self.offset_right = self.pos;

            let mut w = 0;
            let runes = &self.value[..self.offset_right];
            let mut i = runes.len() - 1;

            while i > 0 && w < self.width() {
                w += rune_width(runes[i]);
                if w <= self.width() {
                    i -= 1;
                }
            }

            self.offset = self.offset_right - (runes.len() - 1 - i);
        }
    }

    /// deleteBeforeCursor deletes all text before the cursor.
    fn delete_before_cursor(&mut self) {
        self.value = self.value[self.pos..].to_vec();
        self.err = self.validate(&self.value);
        self.offset = 0;
        self.set_cursor(0);
    }

    /// deleteAfterCursor deletes all text after the cursor. If input is
    /// masked delete everything after the cursor so as not to reveal word
    /// breaks in the masked input.
    fn delete_after_cursor(&mut self) {
        self.value = self.value[..self.pos].to_vec();
        self.err = self.validate(&self.value);
        self.set_cursor(self.value.len());
    }

    /// deleteWordBackward deletes the word left to the cursor.
    fn delete_word_backward(&mut self) {
        if self.pos == 0 || self.value.is_empty() {
            return;
        }

        if self.echo_mode != EchoMode::EchoNormal {
            self.delete_before_cursor();
            return;
        }

        // Linter note: it's critical that we acquire the initial cursor
        // position here prior to altering it via SetCursor() below.
        let old_pos = self.pos;

        self.set_cursor(self.pos - 1);
        loop {
            if self.pos == 0 {
                break;
            }
            if !self.value[self.pos].is_whitespace() {
                break;
            }
            // ignore series of whitespace before cursor
            self.set_cursor(self.pos - 1);
        }

        while self.pos > 0 {
            if !self.value[self.pos].is_whitespace() {
                self.set_cursor(self.pos - 1);
            } else {
                if self.pos > 0 {
                    // keep the previous space
                    self.set_cursor(self.pos + 1);
                }
                break;
            }
        }

        if old_pos > self.value.len() {
            self.value = self.value[..self.pos].to_vec();
        } else {
            let mut v = self.value[..self.pos].to_vec();
            v.extend_from_slice(&self.value[old_pos..]);
            self.value = v;
        }
        self.err = self.validate(&self.value);
    }

    /// deleteWordForward deletes the word right to the cursor. If input is
    /// masked delete everything after the cursor so as not to reveal word
    /// breaks in the masked input.
    fn delete_word_forward(&mut self) {
        if self.pos >= self.value.len() || self.value.is_empty() {
            return;
        }

        if self.echo_mode != EchoMode::EchoNormal {
            self.delete_after_cursor();
            return;
        }

        let old_pos = self.pos;
        self.set_cursor(self.pos + 1);
        loop {
            // ignore series of whitespace after cursor
            self.set_cursor(self.pos + 1);
            if self.pos >= self.value.len() {
                break;
            }
            if !self.value[self.pos].is_whitespace() {
                break;
            }
        }

        while self.pos < self.value.len() {
            if !self.value[self.pos].is_whitespace() {
                self.set_cursor(self.pos + 1);
            } else {
                break;
            }
        }

        if self.pos > self.value.len() {
            self.value = self.value[..old_pos].to_vec();
        } else {
            let mut v = self.value[..old_pos].to_vec();
            v.extend_from_slice(&self.value[self.pos..]);
            self.value = v;
        }
        self.err = self.validate(&self.value);

        self.set_cursor(old_pos);
    }

    /// wordBackward moves the cursor one word to the left. If input is
    /// masked, move input to the start so as not to reveal word breaks in
    /// the masked input.
    fn word_backward(&mut self) {
        if self.pos == 0 || self.value.is_empty() {
            return;
        }

        if self.echo_mode != EchoMode::EchoNormal {
            self.cursor_start();
            return;
        }

        let mut i = self.pos as isize - 1;
        while i >= 0 {
            if self.value[i as usize].is_whitespace() {
                self.set_cursor(self.pos - 1);
                i -= 1;
            } else {
                break;
            }
        }

        while i >= 0 {
            if !self.value[i as usize].is_whitespace() {
                self.set_cursor(self.pos - 1);
                i -= 1;
            } else {
                break;
            }
        }
    }

    /// wordForward moves the cursor one word to the right. If the input is
    /// masked, move input to the end so as not to reveal word breaks in the
    /// masked input.
    fn word_forward(&mut self) {
        if self.pos >= self.value.len() || self.value.is_empty() {
            return;
        }

        if self.echo_mode != EchoMode::EchoNormal {
            self.cursor_end();
            return;
        }

        let mut i = self.pos;
        while i < self.value.len() {
            if self.value[i].is_whitespace() {
                self.set_cursor(self.pos + 1);
                i += 1;
            } else {
                break;
            }
        }

        while i < self.value.len() {
            if !self.value[i].is_whitespace() {
                self.set_cursor(self.pos + 1);
                i += 1;
            } else {
                break;
            }
        }
    }

    fn echo_transform(&self, v: &str) -> String {
        match self.echo_mode {
            EchoMode::EchoPassword => self.echo_character.to_string().repeat(string_width(v)),
            EchoMode::EchoNone => String::new(),
            EchoMode::EchoNormal => v.to_string(),
        }
    }

    /// Update is the Bubble Tea update loop.
    pub fn update(&mut self, msg: &dyn Msg) -> Cmd {
        if !self.focus {
            return None;
        }

        // Need to check for completion before, because key is configurable
        // and might be double assigned.
        let key_press = msg.as_any().downcast_ref::<KeyPressMsg>();
        if let Some(kp) = key_press {
            if key::matches(&kp.0, std::slice::from_ref(&self.key_map.accept_suggestion))
                && self.can_accept_suggestion()
            {
                let suggestion = &self.matched_suggestions[self.current_suggestion_index];
                let rest: Vec<char> = suggestion[self.value.len()..].to_vec();
                self.value.extend_from_slice(&rest);
                self.cursor_end();
            }
        }

        // Let's remember where the position of the cursor currently is so
        // that if the cursor position changes, we can reset the blink.
        let old_pos = self.pos;

        if let Some(kp) = key_press {
            let k: &Key = &kp.0;
            if key::matches(k, std::slice::from_ref(&self.key_map.delete_word_backward)) {
                self.delete_word_backward();
            } else if key::matches(
                k,
                std::slice::from_ref(&self.key_map.delete_character_backward),
            ) {
                self.err = None;
                if !self.value.is_empty() {
                    let mut v = self.value[..self.pos.max(1) - 1].to_vec();
                    v.extend_from_slice(&self.value[self.pos..]);
                    self.value = v;
                    self.err = self.validate(&self.value);
                    if self.pos > 0 {
                        self.set_cursor(self.pos - 1);
                    }
                }
            } else if key::matches(k, std::slice::from_ref(&self.key_map.word_backward)) {
                self.word_backward();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.character_backward)) {
                if self.pos > 0 {
                    self.set_cursor(self.pos - 1);
                }
            } else if key::matches(k, std::slice::from_ref(&self.key_map.word_forward)) {
                self.word_forward();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.character_forward)) {
                if self.pos < self.value.len() {
                    self.set_cursor(self.pos + 1);
                }
            } else if key::matches(k, std::slice::from_ref(&self.key_map.line_start)) {
                self.cursor_start();
            } else if key::matches(
                k,
                std::slice::from_ref(&self.key_map.delete_character_forward),
            ) {
                if !self.value.is_empty() && self.pos < self.value.len() {
                    self.value.remove(self.pos);
                    self.err = self.validate(&self.value);
                }
            } else if key::matches(k, std::slice::from_ref(&self.key_map.line_end)) {
                self.cursor_end();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.delete_after_cursor)) {
                self.delete_after_cursor();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.delete_before_cursor)) {
                self.delete_before_cursor();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.paste)) {
                return self.paste();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.delete_word_forward)) {
                self.delete_word_forward();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.next_suggestion)) {
                self.next_suggestion();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.prev_suggestion)) {
                self.previous_suggestion();
            } else {
                // Input one or more regular characters.
                let text: Vec<char> = kp.0.text.chars().collect();
                self.insert_ranes_from_user_input(&text);
            }

            // Check again if can be completed because value might be
            // something that does not match the completion prefix.
            self.update_suggestions();
        } else if let Some(pm) = msg.as_any().downcast_ref::<PasteMsg>() {
            let content: Vec<char> = pm.content.chars().collect();
            self.insert_ranes_from_user_input(&content);
        } else if let Some(pm) = msg.as_any().downcast_ref::<PasteMsgInternal>() {
            let content: Vec<char> = pm.0.chars().collect();
            self.insert_ranes_from_user_input(&content);
        } else if let Some(pm) = msg.as_any().downcast_ref::<PasteErrMsg>() {
            self.err = Some(pm.0.clone());
        }

        let mut cmds: Vec<Cmd> = Vec::new();

        if self.use_virtual_cursor {
            let cmd = self.virtual_cursor.update(msg);
            cmds.push(cmd);

            // If the cursor position changed, reset the blink state. This is
            // a small UX nuance that makes cursor movement obvious and feel
            // snappy.
            if old_pos != self.pos && self.virtual_cursor.mode() == cursor::Mode::Blink {
                self.virtual_cursor.is_blinked = false;
                cmds.push(self.virtual_cursor.blink());
            }
        }

        self.handle_overflow();
        commands::batch(cmds)
    }

    /// View renders the textinput in its current state.
    pub fn view(&self) -> String {
        // Placeholder text
        if self.value.is_empty() && !self.placeholder.is_empty() {
            return self.placeholder_view();
        }

        let styles = self.active_style();

        let style_text = styles.text.clone().inline(true);

        let value = &self.value[self.offset..self.offset_right];
        let pos = self.pos - self.offset;
        let mut v =
            style_text.render(&self.echo_transform(&String::from_iter(value[..pos].iter())));

        // The upstream View() operates on a copy of the model, so cursor
        // mutations are applied to a local clone here.
        let mut vc = self.virtual_cursor.clone();

        if pos < value.len() {
            let char = self.echo_transform(&String::from_iter(value[pos..pos + 1].iter()));
            vc.set_char(&char);
            v += &vc.view(); // cursor and text under it
            v += &style_text
                .render(&self.echo_transform(&String::from_iter(value[pos + 1..].iter()))); // text after cursor
            v += &self.completion_view(0); // suggested completion
        } else if self.focus && self.can_accept_suggestion() {
            let suggestion = &self.matched_suggestions[self.current_suggestion_index];
            if value.len() < suggestion.len() {
                vc.text_style = styles.suggestion.clone();
                vc.set_char(
                    &self.echo_transform(&String::from_iter(suggestion[pos..pos + 1].iter())),
                );
                v += &vc.view();
                v += &self.completion_view(1);
            } else {
                vc.set_char(" ");
                v += &vc.view();
            }
        } else {
            vc.set_char(" ");
            v += &vc.view();
        }

        // If a max width and background color were set fill the empty spaces
        // with the background color.
        let val_width = string_width(&String::from_iter(value.iter()));
        if self.width() > 0 && val_width <= self.width() {
            let mut padding = self.width() - val_width;
            if val_width + padding <= self.width() && pos < value.len() {
                padding += 1;
            }
            v += &style_text.render(&" ".repeat(padding));
        }

        self.prompt_view() + &v
    }

    fn prompt_view(&self) -> String {
        self.active_style().prompt.clone().render(&self.prompt)
    }

    /// placeholderView returns the prompt and placeholder view, if any.
    fn placeholder_view(&self) -> String {
        let styles = self.active_style();
        let render = styles.placeholder.clone();

        let mut p: Vec<char> = self.placeholder.chars().collect();
        p.resize(self.width() + 1, '\0');

        let mut vc = self.virtual_cursor.clone();
        vc.text_style = styles.placeholder.clone();
        vc.set_char(&p[..1].iter().collect::<String>());
        let mut v = vc.view();

        // If the entire placeholder is already set and no padding is needed,
        // finish.
        if self.width() < 1 && p.len() <= 1 {
            return styles.prompt.clone().render(&self.prompt) + &v;
        }

        // If Width is set then size placeholder accordingly.
        if self.width() > 0 {
            // available width is width - len + cursor offset of 1
            let mut min_width = rusty_lipgloss::size::width(&self.placeholder);
            let avail = (self.width() as i64) - (min_width as i64) + 1;
            let avail_width: usize;

            // if width < len, 'subtract'(add) number to len and dont add
            // padding
            if avail < 0 {
                min_width = (min_width as i64 + avail).max(0) as usize;
                avail_width = 0;
            } else {
                avail_width = avail as usize;
            }
            // append placeholder[len] - cursor, append padding
            v += &render.render(&String::from_iter(p[1..min_width].iter()));
            v += &render.render(&" ".repeat(avail_width));
        } else {
            // if there is no width, the placeholder can be any length
            v += &render.render(&String::from_iter(p[1..].iter()));
        }

        styles.prompt.clone().render(&self.prompt) + &v
    }

    fn completion_view(&self, offset: usize) -> String {
        if !self.can_accept_suggestion() {
            return String::new();
        }
        let value = &self.value;
        let suggestion = &self.matched_suggestions[self.current_suggestion_index];
        if value.len() < suggestion.len() {
            return self
                .active_style()
                .suggestion
                .clone()
                .inline(true)
                .render(&String::from_iter(
                    suggestion[value.len() + offset..].iter(),
                ));
        }
        String::new()
    }

    /// AvailableSuggestions returns the list of available suggestions.
    pub fn available_suggestions(&self) -> Vec<String> {
        self.suggestions
            .iter()
            .map(|s| String::from_iter(s.iter()))
            .collect()
    }

    /// MatchedSuggestions returns the list of matched suggestions.
    pub fn matched_suggestions(&self) -> Vec<String> {
        self.matched_suggestions
            .iter()
            .map(|s| String::from_iter(s.iter()))
            .collect()
    }

    /// CurrentSuggestionIndex returns the currently selected suggestion
    /// index.
    pub fn current_suggestion_index(&self) -> usize {
        self.current_suggestion_index
    }

    /// CurrentSuggestion returns the currently selected suggestion.
    pub fn current_suggestion(&self) -> String {
        if self.current_suggestion_index >= self.matched_suggestions.len() {
            return String::new();
        }

        String::from_iter(self.matched_suggestions[self.current_suggestion_index].iter())
    }

    /// canAcceptSuggestion returns whether there is an acceptable suggestion
    /// to autocomplete the current value.
    pub fn can_accept_suggestion(&self) -> bool {
        !self.matched_suggestions.is_empty()
    }

    /// updateSuggestions refreshes the list of matching suggestions.
    fn update_suggestions(&mut self) {
        if !self.show_suggestions {
            return;
        }

        if self.value.is_empty() || self.suggestions.is_empty() {
            self.matched_suggestions = vec![];
            return;
        }

        let mut matches: Vec<Vec<char>> = Vec::new();
        for s in &self.suggestions {
            let suggestion = String::from_iter(s.iter());

            let lower_suggestion = suggestion.to_lowercase();
            let lower_value = String::from_iter(self.value.iter()).to_lowercase();
            if lower_suggestion.starts_with(&lower_value) {
                matches.push(s.clone());
            }
        }
        if matches != self.matched_suggestions {
            self.current_suggestion_index = 0;
        }

        self.matched_suggestions = matches;
    }

    /// nextSuggestion selects the next suggestion.
    fn next_suggestion(&mut self) {
        self.current_suggestion_index += 1;
        if self.current_suggestion_index >= self.matched_suggestions.len() {
            self.current_suggestion_index = 0;
        }
    }

    /// previousSuggestion selects the previous suggestion.
    fn previous_suggestion(&mut self) {
        if self.current_suggestion_index == 0 {
            self.current_suggestion_index = self.matched_suggestions.len() - 1;
        } else {
            self.current_suggestion_index -= 1;
        }
    }

    fn validate(&self, v: &[char]) -> Option<String> {
        match &self.validate {
            Some(f) => {
                let s = String::from_iter(v.iter());
                f(&s).err()
            }
            None => None,
        }
    }

    fn update_virtual_cursor_style(&mut self) {
        if !self.use_virtual_cursor {
            // Hide the virtual cursor if we're using a real cursor.
            self.virtual_cursor.set_mode(cursor::Mode::Hide);
            return;
        }

        self.virtual_cursor.style = Style::new().foreground_color(self.styles.cursor.color.clone());

        // By default, the blink speed of the cursor is set to a default
        // internally.
        if self.styles.cursor.blink {
            if !self.styles.cursor.blink_speed.is_zero() {
                self.virtual_cursor.blink_speed = self.styles.cursor.blink_speed;
            }
            self.virtual_cursor.set_mode(cursor::Mode::Blink);
            return;
        }
        self.virtual_cursor.set_mode(cursor::Mode::Static);
    }

    fn active_style(&self) -> StyleState {
        if self.focus {
            self.styles.focused.clone()
        } else {
            self.styles.blurred.clone()
        }
    }

    /// Paste is a command for pasting from the clipboard into the text input.
    fn paste(&self) -> Cmd {
        Some(Box::new(|| match clipboard::read_all() {
            Ok(str) => Some(Box::new(PasteMsgInternal(str))),
            Err(err) => Some(Box::new(PasteErrMsg(err))),
        }))
    }
}

/// Blink is a command used to initialize cursor blinking.
pub fn blink() -> Box<dyn Msg> {
    crate::cursor::blink()
}

fn clamp(v: usize, low: usize, high: usize) -> usize {
    if high < low {
        return low;
    }
    high.min(low.max(v))
}

fn rune_width(c: char) -> usize {
    UnicodeWidthChar::width(c).unwrap_or(0)
}

fn string_width(s: &str) -> usize {
    s.chars().map(rune_width).sum()
}

/// DefaultStyles returns the default styles for focused and blurred states
/// for the textarea.
pub fn default_styles(is_dark: bool) -> Styles {
    let light_dark = rusty_lipgloss::color::light_dark(is_dark);

    Styles {
        focused: StyleState {
            placeholder: Style::new().foreground("240"),
            suggestion: Style::new().foreground("240"),
            prompt: Style::new().foreground("7"),
            text: Style::new(),
        },
        blurred: StyleState {
            placeholder: Style::new().foreground("240"),
            suggestion: Style::new().foreground("240"),
            prompt: Style::new().foreground("7"),
            text: Style::new().foreground_color(light_dark(Color::parse("245"), Color::parse("7"))),
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
    /// Style for the text.
    pub text: Style,
    /// Style for the placeholder.
    pub placeholder: Style,
    /// Style for the suggestion.
    pub suggestion: Style,
    /// Style for the prompt.
    pub prompt: Style,
}

/// CursorStyle is the style for real and virtual cursors.
#[derive(Debug, Clone)]
pub struct CursorStyle {
    /// Style styles the cursor block.
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
