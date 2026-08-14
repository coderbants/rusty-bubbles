//! Cleanroom Rust port of upstream Go source file: `viewport/viewport.go`
//! Cleanroom Rust port of upstream Go source file: `viewport/keymap.go`
//! Cleanroom Rust port of upstream Go source file: `viewport/highlight.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! <public-docs>
//! # Viewport
//!
//! A component for rendering a viewport in a Bubble Tea application.
//! </public-docs>

use crate::key::{self, Binding};
use rusty_bubbletea::key::KeyPressMsg;
use rusty_bubbletea::model::{Cmd, Msg};
use rusty_bubbletea::mouse::{MouseButton, MouseWheelMsg};
use rusty_lipgloss::{self, ranges::Range, Style};
use rusty_x_ansi;
use std::collections::HashMap;

/// defaultHorizontalStep is the number of columns the viewport moves
/// horizontally by default.
const DEFAULT_HORIZONTAL_STEP: usize = 6;

/// Option is a configuration option that works in conjunction with [`new`].
/// For example:
///
/// ```rust
/// # use rusty_bubbles::viewport;
/// let viewport = viewport::new(vec![viewport::with_width(10), viewport::with_height(5)]);
/// ```
pub type Option = Box<dyn FnOnce(&mut Model)>; // (std::option::Option is used for optionals)

/// WithWidth is an initialization option that sets the width of the
/// viewport. Pass as an argument to [`new`].
pub fn with_width(w: usize) -> Option {
    Box::new(move |m: &mut Model| {
        m.width = w;
    })
}

/// WithHeight is an initialization option that sets the height of the
/// viewport. Pass as an argument to [`new`].
pub fn with_height(h: usize) -> Option {
    Box::new(move |m: &mut Model| {
        m.height = h;
    })
}

/// New returns a new model with the given width and height as well as
/// default key mappings.
impl Default for KeyMap {
    fn default() -> Self {
        default_key_map()
    }
}

/// WithKeyMap sets the keymap used by the viewport.
pub fn with_key_map(km: KeyMap) -> Option {
    Box::new(move |m: &mut Model| {
        m.key_map = km.clone();
    })
}

pub fn new(opts: Vec<Option>) -> Model {
    let mut m = Model {
        width: 0,
        height: 0,
        key_map: default_key_map(),
        soft_wrap: false,
        fill_height: false,
        mouse_wheel_enabled: true,
        mouse_wheel_delta: 3,
        y_offset: 0,
        x_offset: 0,
        horizontal_step: DEFAULT_HORIZONTAL_STEP,
        y_position: 0,
        style: Style::new(),
        left_gutter_func: None,
        initialized: false,
        lines: vec![],
        longest_line_width: 0,
        highlight_style: Style::new(),
        selected_highlight_style: Style::new(),
        style_line_func: None,
        highlights: vec![],
        hi_idx: -1,
        clone_hack: std::marker::PhantomData,
    };

    for opt in opts {
        opt(&mut m);
    }
    m.set_initial_values();
    m
}

/// GutterContext provides context to a [`GutterFunc`].
#[derive(Debug, Clone, Copy)]
pub struct GutterContext {
    /// Index is the line index of the line which the gutter is being
    /// rendered for.
    pub index: usize,

    /// TotalLines is the total number of lines in the viewport.
    pub total_lines: usize,

    /// Soft is whether or not the line is soft wrapped.
    pub soft: bool,
}

/// GutterFunc can be implemented and set into [`Model::left_gutter_func`].
///
/// Example implementation showing line numbers:
///
/// ```rust
/// # use rusty_bubbles::viewport::{self, GutterContext};
/// fn line_numbers(info: GutterContext) -> String {
///     if info.soft {
///         return "     │ ".to_string();
///     }
///     if info.index >= info.total_lines {
///         return "   ~ │ ".to_string();
///     }
///     format!("{:4} │ ", info.index + 1)
/// }
/// ```
pub type GutterFunc = Box<dyn Fn(GutterContext) -> String + Send + Sync>;

/// Model is the Bubble Tea model for this viewport element.
pub struct Model {
    width: usize,
    height: usize,
    /// The key mappings for the viewport.
    pub key_map: KeyMap,

    /// Whether or not to wrap text. If false, it'll allow horizontal
    /// scrolling instead.
    pub soft_wrap: bool,

    /// Whether or not to fill to the height of the viewport with empty
    /// lines.
    pub fill_height: bool,

    /// Whether or not to respond to the mouse. The mouse must be enabled in
    /// Bubble Tea for this to work.
    pub mouse_wheel_enabled: bool,

    /// The number of lines the mouse wheel will scroll. By default, this is
    /// 3.
    pub mouse_wheel_delta: usize,

    /// y_offset is the vertical scroll position.
    y_offset: usize,

    /// x_offset is the horizontal scroll position.
    x_offset: usize,

    /// horizontal_step is the number of columns we move left or right
    /// during a default horizontal scroll.
    horizontal_step: usize,

    /// YPosition is the position of the viewport in relation to the terminal
    /// window. It's used in high performance rendering only.
    pub y_position: usize,

    /// Style applies a lipgloss style to the viewport. Realistically, it's
    /// most useful for setting borders, margins and padding.
    pub style: Style,

    /// LeftGutterFunc allows to define a [`GutterFunc`] that adds a column
    /// into the left of the viewport, which is kept when horizontal
    /// scrolling.
    pub left_gutter_func: std::option::Option<GutterFunc>,

    #[doc(hidden)]
    #[allow(dead_code)]
    clone_hack: std::marker::PhantomData<()>,

    initialized: bool,
    lines: Vec<String>,
    longest_line_width: usize,

    /// HighlightStyle highlights the ranges set with [`set_highlights`](Self::set_highlights).
    pub highlight_style: Style,

    /// SelectedHighlightStyle highlights the highlight range focused during
    /// navigation.
    pub selected_highlight_style: Style,

    /// StyleLineFunc allows to return a [`Style`] for each line. The
    /// argument is the line index.
    pub style_line_func: std::option::Option<Box<dyn Fn(usize) -> Style + Send + Sync>>,

    highlights: Vec<HighlightInfo>,
    hi_idx: isize,
}

impl Clone for Model {
    fn clone(&self) -> Self {
        Model {
            width: self.width,
            height: self.height,
            key_map: self.key_map.clone(),
            soft_wrap: self.soft_wrap,
            fill_height: self.fill_height,
            mouse_wheel_enabled: self.mouse_wheel_enabled,
            mouse_wheel_delta: self.mouse_wheel_delta,
            y_offset: self.y_offset,
            x_offset: self.x_offset,
            horizontal_step: self.horizontal_step,
            y_position: self.y_position,
            style: self.style.clone(),
            left_gutter_func: None,
            initialized: self.initialized,
            lines: self.lines.clone(),
            longest_line_width: self.longest_line_width,
            highlight_style: self.highlight_style.clone(),
            selected_highlight_style: self.selected_highlight_style.clone(),
            style_line_func: None,
            highlights: self.highlights.clone(),
            hi_idx: self.hi_idx,
            clone_hack: std::marker::PhantomData,
        }
    }
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("viewport::Model")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("y_offset", &self.y_offset)
            .field("lines", &self.lines.len())
            .finish()
    }
}

impl Model {
    fn set_initial_values(&mut self) {
        self.mouse_wheel_enabled = true;
        self.mouse_wheel_delta = 3;
        self.horizontal_step = DEFAULT_HORIZONTAL_STEP;
        self.initialized = true;
    }

    /// Height returns the height of the viewport.
    pub fn height(&self) -> usize {
        self.height
    }

    /// SetHeight sets the height of the viewport.
    pub fn set_height(&mut self, h: usize) {
        self.height = h;
    }

    /// Width returns the width of the viewport.
    pub fn width(&self) -> usize {
        self.width
    }

    /// SetWidth sets the width of the viewport.
    pub fn set_width(&mut self, w: usize) {
        self.width = w;
    }

    /// AtTop returns whether or not the viewport is at the very top
    /// position.
    pub fn at_top(&self) -> bool {
        self.y_offset() == 0
    }

    /// AtBottom returns whether or not the viewport is at or past the very
    /// bottom position.
    pub fn at_bottom(&self) -> bool {
        self.y_offset() >= self.max_y_offset()
    }

    /// PastBottom returns whether or not the viewport is scrolled beyond the
    /// last line. This can happen when adjusting the viewport height.
    pub fn past_bottom(&self) -> bool {
        self.y_offset() > self.max_y_offset()
    }

    /// ScrollPercent returns the amount scrolled as a float between 0 and 1.
    pub fn scroll_percent(&self) -> f64 {
        let (total, _, _) = self.calculate_line(0);
        if self.height() >= total {
            return 1.0;
        }
        let y = self.y_offset() as f64;
        let h = self.height() as f64;
        let t = total as f64;
        let v = y / (t - h);
        clamp(v, 0.0, 1.0)
    }

    /// HorizontalScrollPercent returns the amount horizontally scrolled as a
    /// float between 0 and 1.
    pub fn horizontal_scroll_percent(&self) -> f64 {
        if self.x_offset >= self.longest_line_width.saturating_sub(self.width()) {
            return 1.0;
        }
        let y = self.x_offset as f64;
        let h = self.width() as f64;
        let t = self.longest_line_width as f64;
        let v = y / (t - h);
        clamp(v, 0.0, 1.0)
    }

    /// SetContent set the pager's text content. Line endings will be
    /// normalized to '\n'.
    pub fn set_content(&mut self, s: &str) {
        self.set_content_lines(&s.split('\n').map(|x| x.to_string()).collect::<Vec<_>>());
    }

    /// SetContentLines allows to set the lines to be shown instead of the
    /// content. If a given line has a \n in it, it will still be split into
    /// multiple lines similar to that of [`set_content`](Self::set_content).
    pub fn set_content_lines(&mut self, lines: &[String]) {
        // if there's no content, set content to actual nil instead of one
        // empty line.
        self.lines = lines.to_vec();
        if self.lines.len() == 1 && rusty_x_ansi::string_width(&self.lines[0]) == 0 {
            self.lines.clear();
        } else {
            // iterate in reverse, so we can safely modify the slice.
            let mut sub_lines: Vec<String>;
            let mut i = self.lines.len();
            while i > 0 {
                i -= 1;
                if !self.lines[i].contains('\r') && !self.lines[i].contains('\n') {
                    continue;
                }

                self.lines[i] = self.lines[i].replace("\r\n", "\n"); // normalize line endings
                sub_lines = self.lines[i].split('\n').map(|x| x.to_string()).collect();
                if sub_lines.len() > 1 {
                    self.lines
                        .splice(i + 1..i + 1, sub_lines[1..].iter().cloned());
                    self.lines[i] = sub_lines[0].clone();
                }
            }
        }

        self.longest_line_width = max_line_width(&self.lines);
        self.clear_highlights();

        if self.y_offset() > self.max_y_offset() {
            self.goto_bottom();
        }
    }

    /// GetContent returns the entire content as a single string.
    /// Line endings are normalized to '\n'.
    pub fn get_content(&self) -> String {
        self.lines.join("\n")
    }

    /// calculateLine taking soft wrapping into account, returns the total
    /// viewable lines and the real-line index for the given yoffset, as well
    /// as the virtual line offset.
    fn calculate_line(&self, yoffset: usize) -> (usize, usize, usize) {
        if !self.soft_wrap {
            let total = self.lines.len();
            let ridx = yoffset.min(self.lines.len());
            return (total, ridx, 0);
        }

        let max_width = self.max_width() as f64;
        let mut total = 0usize;
        let mut ridx = self.lines.len();
        let mut voffset = 0usize;

        for (i, line) in self.lines.iter().enumerate() {
            let line_height =
                1usize.max((rusty_x_ansi::string_width(line) as f64 / max_width).ceil() as usize);

            if yoffset >= total && yoffset < total + line_height {
                ridx = i;
                voffset = yoffset - total;
            }
            total += line_height;
        }

        if yoffset >= total {
            ridx = self.lines.len();
            voffset = 0;
        }

        (total, ridx, voffset)
    }

    /// maxYOffset returns the maximum possible value of the y-offset based
    /// on the viewport's content and set height.
    fn max_y_offset(&self) -> usize {
        let (total, _, _) = self.calculate_line(0);
        total
            .saturating_sub(self.height())
            .saturating_add(self.style.get_vertical_frame_size())
    }

    /// maxXOffset returns the maximum possible value of the x-offset based
    /// on the viewport's content and set width.
    fn max_x_offset(&self) -> usize {
        self.longest_line_width.saturating_sub(self.width())
    }

    /// maxWidth returns the maximum width of the viewport. It accounts for
    /// the frame size, in addition to the gutter size.
    fn max_width(&self) -> usize {
        let mut gutter_size = 0;
        if let Some(g) = &self.left_gutter_func {
            gutter_size = rusty_x_ansi::string_width(&g(GutterContext {
                index: 0,
                total_lines: 0,
                soft: false,
            }));
        }
        self.width()
            .saturating_sub(self.style.get_horizontal_frame_size())
            .saturating_sub(gutter_size)
    }

    /// maxHeight returns the maximum height of the viewport. It accounts for
    /// the frame size.
    fn max_height(&self) -> usize {
        self.height()
            .saturating_sub(self.style.get_vertical_frame_size())
    }

    /// visibleLines returns the lines that should currently be visible in
    /// the viewport.
    fn visible_lines(&self) -> Vec<String> {
        let max_height = self.max_height();
        let max_width = self.max_width();

        if max_height == 0 || max_width == 0 {
            return vec![];
        }

        let (total, ridx, voffset) = self.calculate_line(self.y_offset());
        let mut lines: Vec<String> = vec![];
        if total > 0 {
            let bottom = clamp(ridx + max_height, ridx, self.lines.len());
            lines = self.style_lines(self.lines[ridx..bottom].to_vec(), ridx);
            lines = self.highlight_lines(lines, ridx);
        }

        while self.fill_height && lines.len() < max_height {
            lines.push(String::new());
        }

        // if longest line fit within width, no need to do anything else.
        if (self.x_offset == 0 && self.longest_line_width <= max_width) || max_width == 0 {
            let out = self.setup_gutter(lines, total, ridx);
            return out;
        }

        if self.soft_wrap {
            return self.soft_wrap_lines(lines, max_width, max_height, total, ridx, voffset);
        }

        // Cut the lines to the viewport width.
        for line in lines.iter_mut() {
            *line = rusty_x_ansi::cut(line, self.x_offset, self.x_offset + max_width);
        }
        self.setup_gutter(lines, total, ridx)
    }

    /// styleLines styles the lines using [`Model::style_line_func`].
    fn style_lines(&self, lines: Vec<String>, offset: usize) -> Vec<String> {
        match &self.style_line_func {
            Some(f) => lines
                .iter()
                .enumerate()
                .map(|(i, l)| f(i + offset).render(l))
                .collect(),
            None => lines,
        }
    }

    /// highlightLines highlights the lines with [`Model::highlight_style`]
    /// and [`Model::selected_highlight_style`].
    fn highlight_lines(&self, lines: Vec<String>, offset: usize) -> Vec<String> {
        if self.highlights.is_empty() {
            return lines;
        }
        lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let ranges =
                    make_highlight_ranges(&self.highlights, i + offset, &self.highlight_style);
                if self.hi_idx >= 0 {
                    let sel = &self.highlights[self.hi_idx as usize];
                    if let Some(hi) = sel.lines.get(&(i + offset)) {
                        // Upstream re-styles the line with ONLY the selected
                        // range, replacing the normal highlight ranges.
                        return rusty_lipgloss::ranges::style_ranges(
                            line,
                            &[rusty_lipgloss::ranges::new_range(
                                hi.0,
                                hi.1,
                                self.selected_highlight_style.clone(),
                            )],
                        );
                    }
                }
                rusty_lipgloss::ranges::style_ranges(line, &ranges)
            })
            .collect()
    }

    fn soft_wrap_lines(
        &self,
        lines: Vec<String>,
        max_width: usize,
        max_height: usize,
        total: usize,
        ridx: usize,
        voffset: usize,
    ) -> Vec<String> {
        let mut wrapped_lines: Vec<String> = Vec::with_capacity(max_height);

        let mut idx: usize;
        let mut line_width: usize;
        let mut truncated_line: String;

        for (i, line) in lines.iter().enumerate() {
            // If the line is less than or equal to the max width, it can be
            // added as is.
            line_width = rusty_x_ansi::string_width(line);

            if line_width <= max_width {
                if let Some(g) = &self.left_gutter_func {
                    let gutter = g(GutterContext {
                        index: i + ridx,
                        total_lines: total,
                        soft: false,
                    });
                    wrapped_lines.push(gutter + line);
                } else {
                    wrapped_lines.push(line.clone());
                }
                continue;
            }

            idx = 0;
            while line_width > idx {
                truncated_line = rusty_x_ansi::cut(line, idx, max_width + idx);
                if let Some(g) = &self.left_gutter_func {
                    let gutter = g(GutterContext {
                        index: i + ridx,
                        total_lines: total,
                        soft: idx > 0,
                    });
                    wrapped_lines.push(gutter + &truncated_line);
                } else {
                    wrapped_lines.push(truncated_line);
                }
                idx += max_width;
            }
        }

        wrapped_lines[voffset..(voffset + max_height).min(wrapped_lines.len())].to_vec()
    }

    /// setupGutter sets up the left gutter using [`Model::left_gutter_func`].
    fn setup_gutter(&self, lines: Vec<String>, total: usize, ridx: usize) -> Vec<String> {
        match &self.left_gutter_func {
            None => lines,
            Some(g) => lines
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let gutter = g(GutterContext {
                        index: i + ridx,
                        total_lines: total,
                        soft: false,
                    });
                    gutter + l
                })
                .collect(),
        }
    }

    /// SetYOffset sets the Y offset.
    pub fn set_y_offset(&mut self, n: usize) {
        self.y_offset = clamp(n, 0, self.max_y_offset());
    }

    /// YOffset returns the current Y offset - the vertical scroll position.
    pub fn y_offset(&self) -> usize {
        self.y_offset
    }

    /// EnsureVisible ensures that the given line and column are in the
    /// viewport.
    pub fn ensure_visible(&mut self, line: usize, colstart: usize, colend: usize) {
        let max_width = self.max_width();
        if colend <= max_width {
            self.set_x_offset(0);
        } else {
            self.set_x_offset(colstart.saturating_sub(self.horizontal_step)); // put one step to the left, feels more natural
        }

        if line < self.y_offset() || line >= self.y_offset() + self.max_height() {
            self.set_y_offset(line);
        }
    }

    /// PageDown moves the view down by the number of lines in the viewport.
    pub fn page_down(&mut self) {
        if self.at_bottom() {
            return;
        }
        self.scroll_down(self.height());
    }

    /// PageUp moves the view up by one height of the viewport.
    pub fn page_up(&mut self) {
        if self.at_top() {
            return;
        }
        self.scroll_up(self.height());
    }

    /// HalfPageDown moves the view down by half the height of the viewport.
    pub fn half_page_down(&mut self) {
        if self.at_bottom() {
            return;
        }
        self.scroll_down(self.height() / 2);
    }

    /// HalfPageUp moves the view up by half the height of the viewport.
    pub fn half_page_up(&mut self) {
        if self.at_top() {
            return;
        }
        self.scroll_up(self.height() / 2);
    }

    /// ScrollDown moves the view down by the given number of lines.
    pub fn scroll_down(&mut self, n: usize) {
        if self.at_bottom() || n == 0 || self.lines.is_empty() {
            return;
        }
        // Make sure the number of lines by which we're going to scroll isn't
        // greater than the number of lines we actually have left before we
        // reach the bottom.
        self.set_y_offset(self.y_offset() + n);
        self.hi_idx = self.find_nearest_match();
    }

    /// ScrollUp moves the view up by the given number of lines.
    pub fn scroll_up(&mut self, n: usize) {
        if self.at_top() || n == 0 || self.lines.is_empty() {
            return;
        }
        // Make sure the number of lines by which we're going to scroll isn't
        // greater than the number of lines we are from the top.
        self.set_y_offset(self.y_offset() - n);
        self.hi_idx = self.find_nearest_match();
    }

    /// SetHorizontalStep sets the amount of cells that the viewport moves in
    /// the default viewport keymapping. If set to 0 or less, horizontal
    /// scrolling is disabled.
    pub fn set_horizontal_step(&mut self, n: usize) {
        self.horizontal_step = n;
    }

    /// XOffset returns the current X offset - the horizontal scroll
    /// position.
    pub fn x_offset(&self) -> usize {
        self.x_offset
    }

    /// SetXOffset sets the X offset.
    /// No-op when soft wrap is enabled.
    pub fn set_x_offset(&mut self, n: usize) {
        if self.soft_wrap {
            return;
        }
        self.x_offset = clamp(n, 0, self.max_x_offset());
    }

    /// ScrollLeft moves the viewport to the left by the given number of
    /// columns.
    pub fn scroll_left(&mut self, n: usize) {
        // Upstream uses signed ints and clamps the resulting offset to 0;
        // saturating subtraction mirrors that without overflowing.
        self.set_x_offset(self.x_offset.saturating_sub(n));
    }

    /// ScrollRight moves viewport to the right by the given number of
    /// columns.
    pub fn scroll_right(&mut self, n: usize) {
        self.set_x_offset(self.x_offset + n);
    }

    /// TotalLineCount returns the total number of lines (both hidden and
    /// visible) within the viewport.
    pub fn total_line_count(&self) -> usize {
        let (total, _, _) = self.calculate_line(0);
        total
    }

    /// VisibleLineCount returns the number of the visible lines within the
    /// viewport.
    pub fn visible_line_count(&self) -> usize {
        self.visible_lines().len()
    }

    /// GotoTop sets the viewport to the top position.
    pub fn goto_top(&mut self) -> Vec<String> {
        if self.at_top() {
            return vec![];
        }
        self.set_y_offset(0);
        self.hi_idx = self.find_nearest_match();
        self.visible_lines()
    }

    /// GotoBottom sets the viewport to the bottom position.
    pub fn goto_bottom(&mut self) -> Vec<String> {
        self.set_y_offset(self.max_y_offset());
        self.hi_idx = self.find_nearest_match();
        self.visible_lines()
    }

    /// SetHighlights sets ranges of characters to highlight.
    /// For instance, `[[2, 10], [20, 30]]` will highlight characters 2 to 10
    /// and 20 to 30.
    /// Note that highlights are not expected to transpose each other, and
    /// are also expected to be in order.
    pub fn set_highlights(&mut self, matches: &[Vec<usize>]) {
        if matches.is_empty() || self.lines.is_empty() {
            return;
        }
        self.highlights = parse_matches(&self.get_content(), matches);
        self.hi_idx = self.find_nearest_match();
        self.show_highlight();
    }

    /// Highlights returns the currently set highlight ranges.
    ///
    /// This is exposed so integration tests can assert on highlight ranges
    /// the same way the upstream in-package tests do.
    pub fn highlights(&self) -> &[HighlightInfo] {
        &self.highlights
    }

    /// ClearHighlights clears previously set highlights.
    pub fn clear_highlights(&mut self) {
        self.highlights.clear();
        self.hi_idx = -1;
    }

    fn show_highlight(&mut self) {
        if self.hi_idx == -1 {
            return;
        }
        let (line, colstart, colend) = self.highlights[self.hi_idx as usize].coords();
        self.ensure_visible(line, colstart, colend);
    }

    /// HighlightNext highlights the next match.
    pub fn highlight_next(&mut self) {
        if self.highlights.is_empty() {
            return;
        }
        self.hi_idx = (self.hi_idx + 1) % self.highlights.len() as isize;
        self.show_highlight();
    }

    /// HighlightPrevious highlights the previous match.
    pub fn highlight_previous(&mut self) {
        if self.highlights.is_empty() {
            return;
        }
        self.hi_idx =
            (self.hi_idx - 1 + self.highlights.len() as isize) % self.highlights.len() as isize;
        self.show_highlight();
    }

    fn find_nearest_match(&self) -> isize {
        for (i, m) in self.highlights.iter().enumerate() {
            if m.line_start >= self.y_offset() {
                return i as isize;
            }
        }
        -1
    }

    /// Update handles standard message-based viewport updates.
    pub fn update(&mut self, msg: &dyn Msg) -> Cmd {
        self.update_as_model(msg);
        None
    }

    fn update_as_model(&mut self, msg: &dyn Msg) {
        if !self.initialized {
            self.set_initial_values();
        }

        if let Some(m) = msg.as_any().downcast_ref::<KeyPressMsg>() {
            let k = &m.0;
            if key::matches(k, std::slice::from_ref(&self.key_map.page_down)) {
                self.page_down();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.page_up)) {
                self.page_up();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.half_page_down)) {
                self.half_page_down();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.half_page_up)) {
                self.half_page_up();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.down)) {
                self.scroll_down(1);
            } else if key::matches(k, std::slice::from_ref(&self.key_map.up)) {
                self.scroll_up(1);
            } else if key::matches(k, std::slice::from_ref(&self.key_map.left)) {
                self.scroll_left(self.horizontal_step);
            } else if key::matches(k, std::slice::from_ref(&self.key_map.right)) {
                self.scroll_right(self.horizontal_step);
            }
            return;
        }

        if let Some(m) = msg.as_any().downcast_ref::<MouseWheelMsg>() {
            if !self.mouse_wheel_enabled {
                return;
            }
            let mouse = &m.0;
            match mouse.button {
                MouseButton::MouseWheelDown => {
                    // NOTE: some terminal emulators don't send the shift
                    // event for mouse actions.
                    if mouse.mod_keys.contains(rusty_bubbletea::key::KeyMod::SHIFT) {
                        self.scroll_right(self.horizontal_step);
                        return;
                    }
                    self.scroll_down(self.mouse_wheel_delta);
                }
                MouseButton::MouseWheelUp => {
                    // NOTE: some terminal emulators don't send the shift
                    // event for mouse actions.
                    if mouse.mod_keys.contains(rusty_bubbletea::key::KeyMod::SHIFT) {
                        self.scroll_left(self.horizontal_step);
                        return;
                    }
                    self.scroll_up(self.mouse_wheel_delta);
                }
                MouseButton::MouseWheelLeft => {
                    self.scroll_left(self.horizontal_step);
                }
                MouseButton::MouseWheelRight => {
                    self.scroll_right(self.horizontal_step);
                }
                _ => {}
            }
        }
    }

    /// View renders the viewport into a string.
    pub fn view(&self) -> String {
        let mut w = self.width();
        let mut h = self.height();
        let sw = self.style.get_width();
        if sw != 0 {
            w = w.min(sw);
        }
        let sh = self.style.get_height();
        if sh != 0 {
            h = h.min(sh);
        }

        if w == 0 || h == 0 {
            return String::new();
        }

        let content_width = w - self.style.get_horizontal_frame_size();
        let content_height = h - self.style.get_vertical_frame_size();
        let vl = self.visible_lines();
        let contents = rusty_lipgloss::new_style()
            .width(content_width) // pad to width.
            .height(content_height) // pad to height.
            .render(&vl.join("\n"));
        self.style
            .clone()
            .unset_width()
            .unset_height() // Style size already applied in contents.
            .render(&contents)
    }
}

/// KeyMap defines the keybindings for the viewport.
#[derive(Debug, Clone)]
pub struct KeyMap {
    /// Page down binding.
    pub page_down: Binding,
    /// Page up binding.
    pub page_up: Binding,
    /// Half page up binding.
    pub half_page_up: Binding,
    /// Half page down binding.
    pub half_page_down: Binding,
    /// Down binding.
    pub down: Binding,
    /// Up binding.
    pub up: Binding,
    /// Left binding.
    pub left: Binding,
    /// Right binding.
    pub right: Binding,
}

/// DefaultKeyMap returns a set of pager-like default keybindings.
pub fn default_key_map() -> KeyMap {
    KeyMap {
        page_down: key::new_binding(vec![
            key::with_keys(&["pgdown", "space", "f"]),
            key::with_help("f/pgdn", "page down"),
        ]),
        page_up: key::new_binding(vec![
            key::with_keys(&["pgup", "b"]),
            key::with_help("b/pgup", "page up"),
        ]),
        half_page_up: key::new_binding(vec![
            key::with_keys(&["u", "ctrl+u"]),
            key::with_help("u", "½ page up"),
        ]),
        half_page_down: key::new_binding(vec![
            key::with_keys(&["d", "ctrl+d"]),
            key::with_help("d", "½ page down"),
        ]),
        up: key::new_binding(vec![
            key::with_keys(&["up", "k"]),
            key::with_help("↑/k", "up"),
        ]),
        down: key::new_binding(vec![
            key::with_keys(&["down", "j"]),
            key::with_help("↓/j", "down"),
        ]),
        left: key::new_binding(vec![
            key::with_keys(&["left", "h"]),
            key::with_help("←/h", "move left"),
        ]),
        right: key::new_binding(vec![
            key::with_keys(&["right", "l"]),
            key::with_help("→/l", "move right"),
        ]),
    }
}

/// HighlightInfo holds the highlight ranges for a set of matches.
///
/// This is exposed so integration tests can assert on highlight ranges the
/// same way the upstream in-package tests do.
#[derive(Debug, Clone, PartialEq)]
pub struct HighlightInfo {
    /// in which line this highlight starts and ends
    pub line_start: usize,
    /// in which line this highlight ends
    pub line_end: usize,
    /// the grapheme highlight ranges for each of these lines
    pub lines: HashMap<usize, (usize, usize)>,
}

impl HighlightInfo {
    /// coords returns the line x column of this highlight.
    fn coords(&self) -> (usize, usize, usize) {
        for i in self.line_start..=self.line_end {
            if let Some(hl) = self.lines.get(&i) {
                return (i, hl.0, hl.1);
            }
        }
        (self.line_start, 0, 0)
    }
}

/// parseMatches converts the given matches into highlight ranges.
///
/// Assumptions:
/// - matches are measured in bytes, e.g. what a regex match would return
/// - matches were made against the given content
/// - matches are in order
/// - matches do not overlap
/// - content is line terminated with \n only
fn parse_matches(content: &str, matches: &[Vec<usize>]) -> Vec<HighlightInfo> {
    if matches.is_empty() {
        return vec![];
    }

    // NOTE: matches are byte ranges into the raw (unstyled) content, so the
    // walk below must index by *byte* position, decoding each UTF-8 char as
    // it goes (the upstream Go code indexes a []byte directly).
    let stripped: Vec<u8> = rusty_x_ansi::strip(content).as_bytes().to_vec();

    let mut highlights: Vec<HighlightInfo> = Vec::with_capacity(matches.len());

    for m in matches {
        let (byte_start, byte_end) = (m[0], m[1]);

        // highlight for this match:
        let mut hi = HighlightInfo {
            line_start: 0,
            line_end: 0,
            lines: HashMap::new(),
        };

        let mut line = 0usize;
        let mut grapheme_pos = 0usize;
        let mut previous_lines_offset = 0usize;
        let mut byte_pos = 0usize;

        // find the beginning of this byte range, setup current line and
        // grapheme position.
        while byte_start > byte_pos && byte_pos < stripped.len() {
            let c = char_at(&stripped, byte_pos);
            if c == '\n' {
                previous_lines_offset = grapheme_pos + 1;
                line += 1;
            }
            grapheme_pos += 1usize.max(char_width(c));
            byte_pos += char_len(c);
        }

        hi.line_start = line;
        hi.line_end = line;

        let grapheme_start = grapheme_pos;

        // loop until we find the end
        while byte_end > byte_pos && byte_pos < stripped.len() {
            let c = char_at(&stripped, byte_pos);
            // if it ends with a new line, add the range, increase line, and
            // continue
            if c == '\n' {
                let colstart = grapheme_start.saturating_sub(previous_lines_offset);
                let colend = (grapheme_pos.saturating_sub(previous_lines_offset) + 1).max(colstart); // +1 its \n itself

                if colend > colstart {
                    hi.lines.insert(line, (colstart, colend));
                    hi.line_end = line;
                }

                previous_lines_offset = grapheme_pos + 1;
                line += 1;
            }

            grapheme_pos += 1usize.max(char_width(c));
            byte_pos += char_len(c);
        }

        // we found it!, add highlight and continue
        if byte_pos == byte_end {
            let colstart = grapheme_start.saturating_sub(previous_lines_offset);
            let colend = (grapheme_pos.saturating_sub(previous_lines_offset)).max(colstart);

            if colend > colstart {
                hi.lines.insert(line, (colstart, colend));
                hi.line_end = line;
            }
        }

        highlights.push(hi);
    }

    highlights
}

/// CharAt decodes the UTF-8 character at the given byte offset. The offset
/// must lie on a character boundary (all offsets used here do).
fn char_at(s: &[u8], byte_pos: usize) -> char {
    std::str::from_utf8(&s[byte_pos..])
        .ok()
        .and_then(|r| r.chars().next())
        .unwrap_or('\u{FFFD}')
}

fn make_highlight_ranges(highlights: &[HighlightInfo], line: usize, style: &Style) -> Vec<Range> {
    let mut result: Vec<Range> = vec![];
    for hi in highlights {
        if let Some(lihi) = hi.lines.get(&line) {
            if *lihi == (0, 0) {
                continue;
            }
            result.push(rusty_lipgloss::ranges::new_range(
                lihi.0,
                lihi.1,
                style.clone(),
            ));
        }
    }
    result
}

fn char_width(c: char) -> usize {
    unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
}

fn char_len(c: char) -> usize {
    c.len_utf8()
}

fn clamp<T: PartialOrd + Copy>(v: T, low: T, high: T) -> T {
    if high < low {
        return low;
    }
    if v < low {
        low
    } else if v > high {
        high
    } else {
        v
    }
}

fn max_line_width(lines: &[String]) -> usize {
    let mut result = 0;
    for line in lines {
        result = result.max(rusty_x_ansi::string_width(line));
    }
    result
}
