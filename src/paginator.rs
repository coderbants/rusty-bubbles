//! Cleanroom Rust port of upstream Go source file: `paginator/paginator.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! <public-docs>
//! # Paginator
//!
//! A Bubble Tea package for calculating pagination and rendering pagination
//! info. Note that this package does not render actual pages: it's purely for
//! handling keystrokes related to pagination, and rendering pagination status.
//! </public-docs>

use crate::key::{self, Binding};
use rusty_bubbletea::key::{Key, KeyPressMsg};
use rusty_bubbletea::model::{Cmd, Msg};

/// Type specifies the way we render pagination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Type {
    /// Arabic numerals, e.g. "3/7".
    #[default]
    Arabic,
    /// Dot indicators.
    Dots,
}

/// KeyMap is the key bindings for different actions within the paginator.
#[derive(Debug, Clone)]
pub struct KeyMap {
    /// Binding to go to the previous page.
    pub prev_page: Binding,
    /// Binding to go to the next page.
    pub next_page: Binding,
}

/// DefaultKeyMap is the default set of key bindings for navigating and acting
/// upon the paginator.
pub fn default_key_map() -> KeyMap {
    KeyMap {
        prev_page: key::new_binding(vec![key::with_keys(&["pgup", "left", "h"])]),
        next_page: key::new_binding(vec![key::with_keys(&["pgdown", "right", "l"])]),
    }
}

/// Model is the Bubble Tea model for this user interface.
#[derive(Debug, Clone)]
pub struct Model {
    /// Type configures how the pagination is rendered (Arabic, Dots).
    pub type_: Type,
    /// Page is the current page number.
    pub page: usize,
    /// PerPage is the number of items per page.
    pub per_page: usize,
    /// TotalPages is the total number of pages.
    pub total_pages: usize,
    /// ActiveDot is used to mark the current page under the Dots display type.
    pub active_dot: String,
    /// InactiveDot is used to mark inactive pages under the Dots display type.
    pub inactive_dot: String,
    /// ArabicFormat is the printf-style format to use for the Arabic display type.
    pub arabic_format: String,

    /// KeyMap encodes the keybindings recognized by the widget.
    pub key_map: KeyMap,
}

impl Model {
    /// SetTotalPages is a helper function for calculating the total number of
    /// pages from a given number of items. Its use is optional since this
    /// pager can be used for other things beyond navigating sets. Note that
    /// it both returns the number of total pages and alters the model.
    pub fn set_total_pages(&mut self, items: usize) -> usize {
        if items < 1 {
            return self.total_pages;
        }
        let mut n = items / self.per_page;
        if !items.is_multiple_of(self.per_page) {
            n += 1;
        }
        self.total_pages = n;
        n
    }

    /// ItemsOnPage is a helper function for returning the number of items on
    /// the current page given the total number of items passed as an
    /// argument.
    pub fn items_on_page(&self, total_items: usize) -> usize {
        if total_items < 1 {
            return 0;
        }
        let (start, end) = self.get_slice_bounds(total_items);
        end - start
    }

    /// GetSliceBounds is a helper function for paginating slices. Pass the
    /// length of the slice you're rendering and you'll receive the start and
    /// end bounds corresponding to the pagination. For example:
    ///
    /// ```rust
    /// # use rusty_bubbles::paginator;
    /// # let mut model = paginator::new(vec![]);
    /// # model.per_page = 2;
    /// let bunch_of_stuff = vec![1, 2, 3, 4, 5];
    /// let (start, end) = model.get_slice_bounds(bunch_of_stuff.len());
    /// let slice_to_render = &bunch_of_stuff[start..end];
    /// ```
    pub fn get_slice_bounds(&self, length: usize) -> (usize, usize) {
        let start = self.page * self.per_page;
        let end = (self.page * self.per_page + self.per_page).min(length);
        (start, end)
    }

    /// PrevPage is a helper function for navigating one page backward. It
    /// will not page beyond the first page (i.e. page 0).
    pub fn prev_page(&mut self) {
        if self.page > 0 {
            self.page -= 1;
        }
    }

    /// NextPage is a helper function for navigating one page forward. It
    /// will not page beyond the last page (i.e. totalPages - 1).
    pub fn next_page(&mut self) {
        if !self.on_last_page() {
            self.page += 1;
        }
    }

    /// OnLastPage returns whether or not we're on the last page.
    pub fn on_last_page(&self) -> bool {
        self.page == self.total_pages - 1
    }

    /// OnFirstPage returns whether or not we're on the first page.
    pub fn on_first_page(&self) -> bool {
        self.page == 0
    }

    /// Update is the Tea update function which binds keystrokes to
    /// pagination.
    pub fn update(&mut self, msg: &dyn Msg) -> Cmd {
        if let Some(m) = msg.as_any().downcast_ref::<KeyPressMsg>() {
            let k: &Key = &m.0;
            if key::matches(k, std::slice::from_ref(&self.key_map.next_page)) {
                self.next_page();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.prev_page)) {
                self.prev_page();
            }
        }
        None
    }

    /// View renders the pagination to a string.
    pub fn view(&self) -> String {
        match self.type_ {
            Type::Dots => self.dots_view(),
            Type::Arabic => self.arabic_view(),
        }
    }

    fn dots_view(&self) -> String {
        let mut s = String::new();
        for i in 0..self.total_pages {
            if i == self.page {
                s += &self.active_dot;
                continue;
            }
            s += &self.inactive_dot;
        }
        s
    }

    fn arabic_view(&self) -> String {
        // %d/%d with Go's fmt.Sprintf semantics; only the two integer
        // placeholders used by default are supported.
        let format = self.arabic_format.clone();
        if format == "%d/%d" {
            format!("{}/{}", self.page + 1, self.total_pages)
        } else {
            let s = format.replacen("%d", &(self.page + 1).to_string(), 1);
            s.replacen("%d", &self.total_pages.to_string(), 1)
        }
    }
}

/// Option is used to set options in [`new`].
pub type Option = Box<dyn FnOnce(&mut Model)>;

/// New creates a new model with defaults.
pub fn new(opts: Vec<Option>) -> Model {
    let mut m = Model {
        type_: Type::Arabic,
        page: 0,
        per_page: 1,
        total_pages: 1,
        key_map: default_key_map(),
        active_dot: "•".to_string(),
        inactive_dot: "○".to_string(),
        arabic_format: "%d/%d".to_string(),
    };

    for opt in opts {
        opt(&mut m);
    }

    m
}

/// WithTotalPages sets the total pages.
pub fn with_total_pages(total_pages: usize) -> Option {
    Box::new(move |m: &mut Model| {
        m.total_pages = total_pages;
    })
}

/// WithPerPage sets the total pages.
pub fn with_per_page(per_page: usize) -> Option {
    Box::new(move |m: &mut Model| {
        m.per_page = per_page;
    })
}
