//! Cleanroom Rust port of upstream Go source file: `list/list.go`
//! Cleanroom Rust port of upstream Go source file: `list/defaultitem.go`
//! Cleanroom Rust port of upstream Go source file: `list/keys.go`
//! Cleanroom Rust port of upstream Go source file: `list/style.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! <public-docs>
//! # List
//!
//! A feature-rich Bubble Tea component for browsing a general purpose list
//! of items. It features optional filtering, pagination, help, status
//! messages, and a spinner to indicate activity.
//!
//! The fuzzy filter is an inline port of `github.com/sahilm/fuzzy`.
//! </public-docs>

use crate::help;
use crate::internal::fuzzy;
use crate::key::{self, Binding};
use crate::paginator;
use crate::spinner;
use crate::textinput;
use charming_bubbletea::commands;
use charming_bubbletea::key::KeyPressMsg;
use charming_bubbletea::model::{Cmd, Msg};
use charming_lipgloss::{self, Color, Style};
use charming_x_ansi;
use std::time::Duration;

/// The update callback type for [DefaultDelegate] items (upstream
/// `func(m Msg, l *Model) Cmd`).
type ItemUpdateFunc = Box<dyn Fn(&dyn Msg, &Model) -> Cmd + Send + Sync>;

const BULLET: &str = "•";
const ELLIPSIS: &str = "…";

fn clamp(v: usize, low: usize, high: usize) -> usize {
    if low > high {
        return v.min(low);
    }
    v.max(low).min(high)
}

/// Item is an item that appears in the list.
///
/// Note: `box_clone` and `as_any` are Rust-side requirements (trait objects
/// cannot structurally clone or downcast), mirroring the upstream Go
/// interface which is implemented by any struct.
pub trait Item: Send + Sync + std::fmt::Debug {
    /// FilterValue is the value we use when filtering against this item when
    /// we're filtering the list.
    fn filter_value(&self) -> String;

    /// Clones this item into a boxed trait object (Rust-side adaptation).
    fn box_clone(&self) -> Box<dyn Item + Send + Sync>;

    /// Downcasts to `Any` (Rust-side adaptation for delegate type checks).
    fn as_any(&self) -> &dyn std::any::Any;

    /// Returns the item as a [`DefaultItem`] view, if it implements it.
    fn as_default_item(&self) -> Option<&dyn DefaultItem> {
        None
    }
}

/// ItemDelegate encapsulates the general functionality for all list items.
/// The benefit to separating this logic from the item itself is that you can
/// change the functionality of items without changing the actual items
/// themselves.
///
/// Note that if the delegate also implements help.KeyMap delegate-related
/// help items will be added to the help view.
pub trait ItemDelegate {
    /// Render renders the item's view.
    fn render(&self, m: &Model, index: usize, item: &dyn Item) -> String;

    /// Height is the height of the list item.
    fn height(&self) -> usize;

    /// Spacing is the size of the horizontal gap between list items in
    /// cells.
    fn spacing(&self) -> usize;

    /// Update is the update loop for items. All messages in the list's
    /// update loop will pass through here except when the user is setting a
    /// filter. Use this method to perform item-level updates appropriate to
    /// this delegate.
    ///
    /// Note: `self` and `m` are passed by shared reference (Rust-side
    /// adaptation; the upstream signature takes a mutable model pointer).
    fn update(&self, msg: &dyn Msg, m: &Model) -> Cmd;

    /// ShortHelp returns bindings for the short help view, if the delegate
    /// implements the help keymap interface.
    fn short_help(&self) -> Vec<Binding> {
        vec![]
    }

    /// FullHelp returns bindings for the full help view, if the delegate
    /// implements the help keymap interface.
    fn full_help(&self) -> Vec<Vec<Binding>> {
        vec![]
    }
}

/// FilterMatchesMsg contains data about items matched during filtering. The
/// message should be routed to Update for processing.
#[derive(Debug)]
pub struct FilterMatchesMsg(pub Vec<FilteredItem>);

impl FilteredItem {
    /// Clones this filtered item (Rust-side adaptation).
    fn clone_item(&self) -> FilteredItem {
        FilteredItem {
            index: self.index,
            item: self.item.box_clone(),
            matches: self.matches.clone(),
        }
    }
}

impl FilterMatchesMsg {
    /// Clones the filtered items (Rust-side adaptation).
    pub fn clone_items(&self) -> Vec<FilteredItem> {
        self.0
            .iter()
            .map(|f| FilteredItem {
                index: f.index,
                item: f.item.box_clone(),
                matches: f.matches.clone(),
            })
            .collect()
    }
}

/// FilteredItem holds an item matched by the filter, its index in the
/// unfiltered list, and the rune indices of matched characters.
#[derive(Debug)]
pub struct FilteredItem {
    /// index in the unfiltered list
    pub index: usize,
    /// item matched
    pub item: Box<dyn Item + Send + Sync>,
    /// rune indices of matched items
    pub matches: Vec<usize>,
}

/// Rank defines a rank for a given item.
#[derive(Debug, Clone)]
pub struct Rank {
    /// The index of the item in the original input.
    pub index: usize,
    /// Indices of the actual word that were matched against the filter term.
    pub matched_indexes: Vec<usize>,
}

/// FilterFunc takes a term and a list of strings to search through
/// (defined by `Item::filter_value`). It should return a sorted list of
/// ranks.
pub type FilterFunc = Box<dyn Fn(&str, &[String]) -> Vec<Rank> + Send + Sync>;

/// DefaultFilter uses the fuzzy matcher to filter through the list. This is
/// set by default.
pub fn default_filter(term: &str, targets: &[String]) -> Vec<Rank> {
    let matches = fuzzy::find(term, targets);
    matches
        .iter()
        .map(|r| Rank {
            index: r.index,
            matched_indexes: r.matched_indexes.clone(),
        })
        .collect()
}

/// UnsortedFilter uses the fuzzy matcher to filter through the list. It does
/// not sort the results.
pub fn unsorted_filter(term: &str, targets: &[String]) -> Vec<Rank> {
    let matches = fuzzy::find_no_sort(term, targets);
    matches
        .iter()
        .map(|r| Rank {
            index: r.index,
            matched_indexes: r.matched_indexes.clone(),
        })
        .collect()
}

#[derive(Debug)]
struct StatusMessageTimeoutMsg;

/// FilterState describes the current filtering state on the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterState {
    /// no filter set
    #[default]
    Unfiltered,
    /// user is actively setting a filter
    Filtering,
    /// a filter is applied and user is not editing filter
    FilterApplied,
}

impl FilterState {
    /// String returns a human-readable string of the current filter state.
    pub fn to_string(&self) -> &'static str {
        match self {
            FilterState::Unfiltered => "unfiltered",
            FilterState::Filtering => "filtering",
            FilterState::FilterApplied => "filter applied",
        }
    }
}

/// Model contains the state of this component.
pub struct Model {
    pub show_title: bool,
    pub show_filter: bool,
    pub show_status_bar: bool,
    pub show_pagination: bool,
    pub show_help: bool,
    pub filtering_enabled: bool,

    item_name_singular: String,
    item_name_plural: String,

    /// The title of the list.
    pub title: String,
    /// The styles for the list.
    pub styles: Styles,
    /// Whether scrolling is infinite.
    pub infinite_scrolling: bool,

    /// Key mappings for navigating the list.
    pub key_map: KeyMap,

    /// Filter is used to filter the list.
    pub filter: FilterFunc,

    pub disable_quit_keybindings: bool,

    /// Additional key mappings for the short and full help views.
    pub additional_short_help_keys: Option<Box<dyn Fn() -> Vec<Binding> + Send + Sync>>,
    /// Additional key mappings for the short and full help views.
    pub additional_full_help_keys: Option<Box<dyn Fn() -> Vec<Binding> + Send + Sync>>,

    pub spinner: spinner::Model,
    pub show_spinner: bool,
    pub width: usize,
    pub height: usize,
    /// The paginator of the list.
    pub paginator: paginator::Model,
    cursor: usize,
    /// The help view of the list.
    pub help: help::Model,
    /// The filter input of the list.
    pub filter_input: textinput::Model,
    pub filter_state: FilterState,

    /// How long status messages should stay visible. By default this is
    /// 1 second.
    pub status_message_lifetime: Duration,

    status_message: String,
    status_message_timer: Option<std::thread::JoinHandle<()>>,

    /// The master set of items we're working with.
    pub items: Vec<Box<dyn Item + Send + Sync>>,

    /// Filtered items we're currently displaying. Filtering, toggles and so
    /// on will alter this slice so we can show what is relevant.
    filtered_items: Vec<FilteredItem>,

    delegate: Box<dyn ItemDelegate + Send + Sync>,
}

impl help::KeyMap for Model {
    fn short_help(&self) -> Vec<Binding> {
        Model::short_help(self)
    }

    fn full_help(&self) -> Vec<Vec<Binding>> {
        Model::full_help(self)
    }
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("list::Model")
            .field("title", &self.title)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("items", &self.items.len())
            .finish()
    }
}

/// New returns a new model with sensible defaults.
pub fn new(
    items: Vec<Box<dyn Item + Send + Sync>>,
    delegate: Box<dyn ItemDelegate + Send + Sync>,
    width: usize,
    height: usize,
) -> Model {
    let styles = default_styles(true);

    let mut sp = spinner::new(vec![]);
    sp.spinner = spinner::line();
    sp.style = styles.spinner.clone();

    let mut filter_input = textinput::new();
    filter_input.prompt = "Filter: ".to_string();
    filter_input.char_limit = 64;
    filter_input.focus();

    let mut p = paginator::new(vec![]);
    p.type_ = paginator::Type::Dots;
    p.active_dot = styles
        .active_pagination_dot
        .clone()
        .set_string(&[BULLET])
        .render("");
    p.inactive_dot = styles
        .inactive_pagination_dot
        .clone()
        .set_string(&[BULLET])
        .render("");

    let mut m = Model {
        show_title: true,
        show_filter: true,
        show_status_bar: true,
        show_pagination: true,
        show_help: true,
        item_name_singular: "item".to_string(),
        item_name_plural: "items".to_string(),
        filtering_enabled: true,
        key_map: default_key_map(),
        filter: Box::new(default_filter),
        styles,
        title: "List".to_string(),
        filter_input,
        status_message_lifetime: Duration::from_secs(1),
        width,
        height,
        delegate,
        items,
        paginator: p,
        spinner: sp,
        help: help::new(),
        cursor: 0,
        filter_state: FilterState::Unfiltered,
        infinite_scrolling: false,
        disable_quit_keybindings: false,
        additional_short_help_keys: None,
        additional_full_help_keys: None,
        show_spinner: false,
        status_message: String::new(),
        status_message_timer: None,
        filtered_items: vec![],
    };

    m.update_pagination();
    m.update_keybindings();
    m
}

impl Model {
    /// SetFilteringEnabled enables or disables filtering. Note that this is
    /// different from ShowFilter, which merely hides or shows the input view.
    pub fn set_filtering_enabled(&mut self, v: bool) {
        self.filtering_enabled = v;
        if !v {
            self.reset_filtering();
        }
        self.update_keybindings();
    }

    /// FilteringEnabled returns whether or not filtering is enabled.
    pub fn filtering_enabled(&self) -> bool {
        self.filtering_enabled
    }

    /// SetShowTitle shows or hides the title bar.
    pub fn set_show_title(&mut self, v: bool) {
        self.show_title = v;
        self.update_pagination();
    }

    /// SetFilterText explicitly sets the filter text without relying on user
    /// input. It also sets the filterState to a sane default of
    /// FilterApplied.
    pub fn set_filter_text(&mut self, filter: &str) {
        self.filter_state = FilterState::Filtering;
        self.filter_input.set_value(filter);
        let fmm = filter_items(self);
        self.filtered_items = fmm;
        self.filter_state = FilterState::FilterApplied;
        self.go_to_start();
        self.filter_input.cursor_end();
        self.update_pagination();
        self.update_keybindings();
    }

    /// SetFilterState allows setting the filtering state manually.
    pub fn set_filter_state(&mut self, state: FilterState) {
        self.go_to_start();
        self.filter_state = state;
        self.filter_input.cursor_end();
        self.filter_input.focus();
        self.update_keybindings();
    }

    /// ShowTitle returns whether or not the title bar is set to be rendered.
    pub fn show_title(&self) -> bool {
        self.show_title
    }

    /// SetShowFilter shows or hides the filter bar. Note that this does not
    /// disable filtering, it simply hides the built-in filter view.
    pub fn set_show_filter(&mut self, v: bool) {
        self.show_filter = v;
        self.update_pagination();
    }

    /// ShowFilter returns whether or not the filter is set to be rendered.
    pub fn show_filter(&self) -> bool {
        self.show_filter
    }

    /// SetShowStatusBar shows or hides the view that displays metadata about
    /// the list, such as item counts.
    pub fn set_show_status_bar(&mut self, v: bool) {
        self.show_status_bar = v;
        self.update_pagination();
    }

    /// ShowStatusBar returns whether or not the status bar is set to be
    /// rendered.
    pub fn show_status_bar(&self) -> bool {
        self.show_status_bar
    }

    /// SetStatusBarItemName defines a replacement for the item's identifier.
    /// Defaults to item/items.
    pub fn set_status_bar_item_name(&mut self, singular: &str, plural: &str) {
        self.item_name_singular = singular.to_string();
        self.item_name_plural = plural.to_string();
    }

    /// StatusBarItemName returns singular and plural status bar item names.
    pub fn status_bar_item_name(&self) -> (String, String) {
        (
            self.item_name_singular.clone(),
            self.item_name_plural.clone(),
        )
    }

    /// SetShowPagination hides or shows the paginator. Note that pagination
    /// will still be active, it simply won't be displayed.
    pub fn set_show_pagination(&mut self, v: bool) {
        self.show_pagination = v;
        self.update_pagination();
    }

    /// ShowPagination returns whether the pagination is visible.
    pub fn show_pagination(&mut self) -> bool {
        self.show_pagination
    }

    /// SetShowHelp shows or hides the help view.
    pub fn set_show_help(&mut self, v: bool) {
        self.show_help = v;
        self.update_pagination();
    }

    /// ShowHelp returns whether or not the help is set to be rendered.
    pub fn show_help(&self) -> bool {
        self.show_help
    }

    /// Items returns the items in the list.
    pub fn items(&self) -> &[Box<dyn Item + Send + Sync>] {
        &self.items
    }

    /// SetItems sets the items available in the list. This returns a
    /// command.
    pub fn set_items(&mut self, items: Vec<Box<dyn Item + Send + Sync>>) -> Cmd {
        let mut cmd: Cmd = None;
        self.items = items;

        if self.filter_state != FilterState::Unfiltered {
            self.filtered_items = vec![];
            cmd = filter_items_cmd(self);
        }

        self.update_pagination();
        self.update_keybindings();
        cmd
    }

    /// Select selects the given index of the list and goes to its respective
    /// page.
    pub fn select(&mut self, index: usize) {
        self.paginator.page = index / self.paginator.per_page;
        self.cursor = index % self.paginator.per_page;
    }

    /// ResetSelected resets the selected item to the first item in the first
    /// page of the list.
    pub fn reset_selected(&mut self) {
        self.select(0);
    }

    /// ResetFilter resets the current filtering state.
    pub fn reset_filter(&mut self) {
        self.reset_filtering();
    }

    /// SetItem replaces an item at the given index. This returns a command.
    pub fn set_item(&mut self, index: usize, item: Box<dyn Item + Send + Sync>) -> Cmd {
        let mut cmd: Cmd = None;
        self.items[index] = item;

        if self.filter_state != FilterState::Unfiltered {
            cmd = filter_items_cmd(self);
        }

        self.update_pagination();
        cmd
    }

    /// InsertItem inserts an item at the given index. If the index is out of
    /// the upper bound, the item will be appended. This returns a command.
    pub fn insert_item(&mut self, index: usize, item: Box<dyn Item + Send + Sync>) -> Cmd {
        let mut cmd: Cmd = None;
        self.items = insert_item_into_slice(&self.items, item, index);

        if self.filter_state != FilterState::Unfiltered {
            cmd = filter_items_cmd(self);
        }

        self.update_pagination();
        self.update_keybindings();
        cmd
    }

    /// RemoveItem removes an item at the given index. If the index is out of
    /// bounds this will be a no-op. O(n) complexity, which probably won't
    /// matter in the case of a TUI.
    pub fn remove_item(&mut self, index: usize) {
        self.items = remove_item_from_slice(&self.items, index);
        if self.filter_state != FilterState::Unfiltered {
            self.filtered_items = remove_filter_match_from_slice(&self.filtered_items, index);
            if self.filtered_items.is_empty() {
                self.reset_filtering();
            }
        }
        self.update_pagination();
    }

    /// SetDelegate sets the item delegate.
    pub fn set_delegate(&mut self, d: Box<dyn ItemDelegate + Send + Sync>) {
        self.delegate = d;
        self.update_pagination();
    }

    /// VisibleItems returns the total items available to be shown.
    pub fn visible_items(&self) -> Vec<&dyn Item> {
        if self.filter_state != FilterState::Unfiltered {
            return self
                .filtered_items
                .iter()
                .map(|f| f.item.as_ref() as &dyn Item)
                .collect();
        }
        self.items.iter().map(|i| i.as_ref() as &dyn Item).collect()
    }

    /// SelectedItem returns the current selected item in the list.
    pub fn selected_item(&self) -> Option<&dyn Item> {
        let i = self.index();

        let items = self.visible_items();
        if i >= items.len() {
            return None;
        }
        Some(items[i])
    }

    /// MatchesForItem returns rune positions matched by the current filter,
    /// if any. Use this to style runes matched by the active filter.
    pub fn matches_for_item(&self, index: usize) -> Vec<usize> {
        if self.filtered_items.is_empty() || index >= self.filtered_items.len() {
            return vec![];
        }
        self.filtered_items[index].matches.clone()
    }

    /// Index returns the index of the currently selected item as it is
    /// stored in the filtered list of items.
    pub fn index(&self) -> usize {
        self.paginator.page * self.paginator.per_page + self.cursor
    }

    /// GlobalIndex returns the index of the currently selected item as it is
    /// stored in the unfiltered list of items. This value can be used with
    /// SetItem.
    pub fn global_index(&self) -> usize {
        let index = self.index();

        if self.filtered_items.is_empty() || index >= self.filtered_items.len() {
            return index;
        }

        self.filtered_items[index].index
    }

    /// Cursor returns the index of the cursor on the current page.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// CursorUp moves the cursor up. This can also move the state to the
    /// previous page.
    pub fn cursor_up(&mut self) {
        if self.cursor == 0 && self.paginator.on_first_page() {
            // if infinite scrolling is enabled, go to the last item
            if self.infinite_scrolling {
                self.go_to_end();
                return;
            }
            return;
        }

        // Move the cursor as normal
        if self.cursor > 0 {
            self.cursor -= 1;
            return;
        }

        // Go to the previous page
        self.paginator.prev_page();
        self.cursor = self.max_cursor_index();
    }

    /// CursorDown moves the cursor down. This can also advance the state to
    /// the next page.
    pub fn cursor_down(&mut self) {
        let max_cursor_index = self.max_cursor_index();

        self.cursor += 1;

        // We're still within bounds of the current page, so no need to do
        // anything.
        if self.cursor <= max_cursor_index {
            return;
        }

        // Go to the next page
        if !self.paginator.on_last_page() {
            self.paginator.next_page();
            self.cursor = 0;
            return;
        }

        self.cursor = 0usize.max(max_cursor_index);

        // if infinite scrolling is enabled, go to the first item.
        if self.infinite_scrolling {
            self.go_to_start();
        }
    }

    /// GoToStart moves to the first page, and first item on the first page.
    pub fn go_to_start(&mut self) {
        self.paginator.page = 0;
        self.cursor = 0;
    }

    /// GoToEnd moves to the last page, and last item on the last page.
    pub fn go_to_end(&mut self) {
        self.paginator.page = 0usize.max(self.paginator.total_pages - 1);
        self.cursor = self.max_cursor_index();
    }

    /// PrevPage moves to the previous page, if available.
    pub fn prev_page(&mut self) {
        self.paginator.prev_page();
        self.cursor = clamp(self.cursor, 0, self.max_cursor_index());
    }

    /// NextPage moves to the next page, if available.
    pub fn next_page(&mut self) {
        self.paginator.next_page();
        self.cursor = clamp(self.cursor, 0, self.max_cursor_index());
    }

    fn max_cursor_index(&self) -> usize {
        0usize.max(
            self.paginator
                .items_on_page(self.visible_items().len())
                .saturating_sub(1),
        )
    }

    /// FilterState returns the current filter state.
    pub fn filter_state(&self) -> FilterState {
        self.filter_state
    }

    /// FilterValue returns the current value of the filter.
    pub fn filter_value(&self) -> String {
        self.filter_input.value()
    }

    /// SettingFilter returns whether or not the user is currently editing
    /// the filter value. It's purely a convenience method for the following:
    ///
    /// ```text
    /// m.FilterState() == Filtering
    /// ```
    pub fn setting_filter(&self) -> bool {
        self.filter_state == FilterState::Filtering
    }

    /// IsFiltered returns whether or not the list is currently filtered.
    pub fn is_filtered(&self) -> bool {
        self.filter_state == FilterState::FilterApplied
    }

    /// Width returns the current width setting.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Height returns the current height setting.
    pub fn height(&self) -> usize {
        self.height
    }

    /// SetSpinner allows to set the spinner style.
    pub fn set_spinner(&mut self, spinner: spinner::Spinner) {
        self.spinner.spinner = spinner;
    }

    /// ToggleSpinner toggles the spinner. Note that this also returns a
    /// command.
    pub fn toggle_spinner(&mut self) -> Cmd {
        if !self.show_spinner {
            return self.start_spinner();
        }
        self.stop_spinner();
        None
    }

    /// StartSpinner starts the spinner. Note that this returns a command.
    pub fn start_spinner(&mut self) -> Cmd {
        self.show_spinner = true;
        let tick = self.spinner.tick_msg();
        Some(Box::new(move || Some(Box::new(tick))))
    }

    /// StopSpinner stops the spinner.
    pub fn stop_spinner(&mut self) {
        self.show_spinner = false;
    }

    /// DisableQuitKeybindings is a helper for disabling the keybindings used
    /// for quitting, in case you want to handle this elsewhere in your
    /// application.
    pub fn disable_quit_keybindings(&mut self) {
        self.disable_quit_keybindings = true;
        self.key_map.quit.set_enabled(false);
        self.key_map.force_quit.set_enabled(false);
    }

    /// NewStatusMessage sets a new status message, which will show for a
    /// limited amount of time. Note that this also returns a command.
    pub fn new_status_message(&mut self, s: &str) -> Cmd {
        self.status_message = s.to_string();
        if let Some(t) = &self.status_message_timer {
            let _ = t.thread().id();
        }
        let lifetime = self.status_message_lifetime;
        self.status_message_timer = Some(std::thread::spawn(move || {
            std::thread::sleep(lifetime);
        }));
        Some(Box::new(|| Some(Box::new(StatusMessageTimeoutMsg))))
    }

    /// SetWidth sets the width of this component.
    pub fn set_width(&mut self, v: usize) {
        self.set_size(v, self.height);
    }

    /// SetHeight sets the height of this component.
    pub fn set_height(&mut self, v: usize) {
        self.set_size(self.width, v);
    }

    /// SetSize sets the width and height of this component.
    pub fn set_size(&mut self, width: usize, height: usize) {
        let prompt_width = charming_lipgloss::size::width(
            &self.styles.title.clone().render(&self.filter_input.prompt),
        );

        self.width = width;
        self.height = height;
        self.help.set_width(width);
        let sw = self.spinner_view();
        let sw_width = charming_lipgloss::size::width(&sw);
        self.filter_input
            .set_width(width.saturating_sub(prompt_width + sw_width));
        self.update_pagination();
        self.update_keybindings();
    }

    fn reset_filtering(&mut self) {
        if self.filter_state == FilterState::Unfiltered {
            return;
        }

        self.filter_state = FilterState::Unfiltered;
        self.filter_input.reset();
        self.filtered_items = vec![];
        self.update_pagination();
        self.update_keybindings();
    }

    fn items_as_filter_items(&self) -> Vec<FilteredItem> {
        self.items
            .iter()
            .enumerate()
            .map(|(i, item)| FilteredItem {
                index: i,
                item: item.box_clone(),
                matches: vec![],
            })
            .collect()
    }

    /// Set keybindings according to the filter state.
    pub fn update_keybindings(&mut self) {
        match self.filter_state {
            FilterState::Filtering => {
                self.key_map.cursor_up.set_enabled(false);
                self.key_map.cursor_down.set_enabled(false);
                self.key_map.next_page.set_enabled(false);
                self.key_map.prev_page.set_enabled(false);
                self.key_map.go_to_start.set_enabled(false);
                self.key_map.go_to_end.set_enabled(false);
                self.key_map.filter.set_enabled(false);
                self.key_map.clear_filter.set_enabled(false);
                self.key_map.cancel_while_filtering.set_enabled(true);
                self.key_map
                    .accept_while_filtering
                    .set_enabled(!self.filter_input.value().is_empty());
                self.key_map.quit.set_enabled(false);
                self.key_map.show_full_help.set_enabled(false);
                self.key_map.close_full_help.set_enabled(false);
            }
            _ => {
                let has_items = !self.items.is_empty();
                self.key_map.cursor_up.set_enabled(has_items);
                self.key_map.cursor_down.set_enabled(has_items);

                let has_pages = self.paginator.total_pages > 1;
                self.key_map.next_page.set_enabled(has_pages);
                self.key_map.prev_page.set_enabled(has_pages);

                self.key_map.go_to_start.set_enabled(has_items);
                self.key_map.go_to_end.set_enabled(has_items);

                self.key_map
                    .filter
                    .set_enabled(self.filtering_enabled && has_items);
                self.key_map
                    .clear_filter
                    .set_enabled(self.filter_state == FilterState::FilterApplied);
                self.key_map.cancel_while_filtering.set_enabled(false);
                self.key_map.accept_while_filtering.set_enabled(false);
                self.key_map
                    .quit
                    .set_enabled(!self.disable_quit_keybindings);

                if self.help.show_all {
                    self.key_map.show_full_help.set_enabled(true);
                    self.key_map.close_full_help.set_enabled(true);
                } else {
                    let min_help = count_enabled_bindings(&self.full_help()) > 1;
                    self.key_map.show_full_help.set_enabled(min_help);
                    self.key_map.close_full_help.set_enabled(min_help);
                }
            }
        }
    }

    /// Update pagination according to the amount of items for the current
    /// state.
    pub fn update_pagination(&mut self) {
        let index = self.index();
        let mut avail_height = self.height;

        if self.show_title || (self.show_filter && self.filtering_enabled) {
            avail_height =
                avail_height.saturating_sub(charming_lipgloss::size::height(&self.title_view()));
        }
        if self.show_status_bar {
            avail_height =
                avail_height.saturating_sub(charming_lipgloss::size::height(&self.status_view()));
        }
        if self.show_pagination {
            avail_height = avail_height
                .saturating_sub(charming_lipgloss::size::height(&self.pagination_view()));
        }
        if self.show_help {
            avail_height =
                avail_height.saturating_sub(charming_lipgloss::size::height(&self.help_view()));
        }

        let delegate_height = self.delegate.height();
        let delegate_spacing = self.delegate.spacing();
        self.paginator.per_page =
            1usize.max(avail_height / (delegate_height + delegate_spacing).max(1));

        let pages = self.visible_items().len();
        if pages < 1 {
            self.paginator.set_total_pages(1);
        } else {
            self.paginator.set_total_pages(pages);
        }

        // Restore index
        self.paginator.page = index / self.paginator.per_page.max(1);
        self.cursor = index % self.paginator.per_page.max(1);

        // Make sure the page stays in bounds
        if self.paginator.page >= self.paginator.total_pages - 1 {
            self.paginator.page = 0usize.max(self.paginator.total_pages - 1);
        }
    }

    fn hide_status_message(&mut self) {
        self.status_message = String::new();
        if let Some(t) = &self.status_message_timer {
            let _ = t.thread().id();
        }
        self.status_message_timer = None;
    }

    /// Update is the Bubble Tea update loop.
    pub fn update(&mut self, msg: &dyn Msg) -> Cmd {
        let mut cmds: Vec<Cmd> = Vec::new();

        if let Some(m) = msg.as_any().downcast_ref::<KeyPressMsg>() {
            if key::matches(&m.0, std::slice::from_ref(&self.key_map.force_quit)) {
                return commands::quit();
            }
        }

        if let Some(m) = msg.as_any().downcast_ref::<FilterMatchesMsg>() {
            self.filtered_items = m.clone_items();
            return None;
        }

        if let Some(m) = msg.as_any().downcast_ref::<spinner::TickMsg>() {
            let cmd = self.spinner.update(msg);
            if self.show_spinner {
                cmds.push(cmd);
            }
            let _ = m;
        }

        if msg
            .as_any()
            .downcast_ref::<StatusMessageTimeoutMsg>()
            .is_some()
        {
            self.hide_status_message();
        }

        let cmd = if self.filter_state == FilterState::Filtering {
            self.handle_filtering(msg)
        } else {
            self.handle_browsing(msg)
        };
        cmds.push(cmd);

        commands::batch(cmds)
    }

    /// Updates for when a user is browsing the list.
    fn handle_browsing(&mut self, msg: &dyn Msg) -> Cmd {
        if let Some(m) = msg.as_any().downcast_ref::<KeyPressMsg>() {
            let k = &m.0;
            // Note: we match clear filter before quit because, by default,
            // they're both mapped to escape.
            if key::matches(k, std::slice::from_ref(&self.key_map.clear_filter)) {
                self.reset_filtering();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.quit)) {
                return commands::quit();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.cursor_up)) {
                self.cursor_up();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.cursor_down)) {
                self.cursor_down();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.prev_page)) {
                self.paginator.prev_page();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.next_page)) {
                self.paginator.next_page();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.go_to_start)) {
                self.go_to_start();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.go_to_end)) {
                self.go_to_end();
            } else if key::matches(k, std::slice::from_ref(&self.key_map.filter)) {
                self.hide_status_message();
                if self.filter_input.value().is_empty() {
                    // Populate filter with all items only if the filter is
                    // empty.
                    self.filtered_items = self.items_as_filter_items();
                }
                self.go_to_start();
                self.filter_state = FilterState::Filtering;
                self.filter_input.cursor_end();
                self.filter_input.focus();
                self.update_keybindings();
                return Some(Box::new(|| Some(textinput::blink())));
            } else if key::matches(k, std::slice::from_ref(&self.key_map.show_full_help))
                || key::matches(k, std::slice::from_ref(&self.key_map.close_full_help))
            {
                self.help.show_all = !self.help.show_all;
                self.update_pagination();
            }
        }

        let cmd = self.delegate.update(msg, self);
        self.cursor = clamp(self.cursor, 0, self.max_cursor_index());

        cmd
    }

    /// Updates for when a user is in the filter editing interface.
    fn handle_filtering(&mut self, msg: &dyn Msg) -> Cmd {
        let mut cmds: Vec<Cmd> = Vec::new();

        // Handle keys
        if let Some(m) = msg.as_any().downcast_ref::<KeyPressMsg>() {
            let k = &m.0;
            if key::matches(
                k,
                std::slice::from_ref(&self.key_map.cancel_while_filtering),
            ) {
                self.reset_filtering();
                self.key_map.filter.set_enabled(true);
                self.key_map.clear_filter.set_enabled(false);
            } else if key::matches(
                k,
                std::slice::from_ref(&self.key_map.accept_while_filtering),
            ) {
                self.hide_status_message();

                if self.items.is_empty() {
                    // fallthrough
                } else {
                    let h = self.visible_items();

                    // If we've filtered down to nothing, clear the filter
                    if h.is_empty() {
                        self.reset_filtering();
                    } else {
                        self.filter_input.blur();
                        self.filter_state = FilterState::FilterApplied;
                        self.update_keybindings();

                        if self.filter_input.value().is_empty() {
                            self.reset_filtering();
                        }
                    }
                }
            }
        }

        // Update the filter text input component
        let old_value = self.filter_input.value();
        let input_cmd = self.filter_input.update(msg);
        let filter_changed = old_value != self.filter_input.value();
        cmds.push(input_cmd);

        // If the filtering input has changed, request updated filtering
        if filter_changed {
            cmds.push(filter_items_cmd(self));
            self.key_map
                .accept_while_filtering
                .set_enabled(!self.filter_input.value().is_empty());
        }

        // Update pagination
        self.update_pagination();

        commands::batch(cmds)
    }

    /// ShortHelp returns bindings to show in the abbreviated help view. It's
    /// part of the help.KeyMap interface.
    pub fn short_help(&self) -> Vec<Binding> {
        let mut kb = vec![
            self.key_map.cursor_up.clone(),
            self.key_map.cursor_down.clone(),
        ];

        let filtering = self.filter_state == FilterState::Filtering;

        // If the delegate implements the help.KeyMap interface add the short
        // help items to the short help after the cursor movement keys.
        if !filtering {
            kb.extend(self.delegate.short_help());
        }

        kb.extend(vec![
            self.key_map.filter.clone(),
            self.key_map.clear_filter.clone(),
            self.key_map.accept_while_filtering.clone(),
            self.key_map.cancel_while_filtering.clone(),
        ]);

        if !filtering {
            if let Some(f) = &self.additional_short_help_keys {
                kb.extend(f());
            }
        }

        kb.push(self.key_map.quit.clone());
        kb.push(self.key_map.show_full_help.clone());
        kb
    }

    /// FullHelp returns bindings to show the full help view. It's part of
    /// the help.KeyMap interface.
    pub fn full_help(&self) -> Vec<Vec<Binding>> {
        let mut kb: Vec<Vec<Binding>> = vec![vec![
            self.key_map.cursor_up.clone(),
            self.key_map.cursor_down.clone(),
            self.key_map.next_page.clone(),
            self.key_map.prev_page.clone(),
            self.key_map.go_to_start.clone(),
            self.key_map.go_to_end.clone(),
        ]];

        let filtering = self.filter_state == FilterState::Filtering;

        // If the delegate implements the help.KeyMap interface add full help
        // keybindings to a special section of the full help.
        if !filtering {
            let fh = self.delegate.full_help();
            if !fh.is_empty() {
                kb.extend(fh);
            }
        }

        let mut list_level_bindings = vec![
            self.key_map.filter.clone(),
            self.key_map.clear_filter.clone(),
            self.key_map.accept_while_filtering.clone(),
            self.key_map.cancel_while_filtering.clone(),
        ];

        if !filtering {
            if let Some(f) = &self.additional_full_help_keys {
                list_level_bindings.extend(f());
            }
        }

        kb.push(list_level_bindings);
        kb.push(vec![
            self.key_map.quit.clone(),
            self.key_map.close_full_help.clone(),
        ]);
        kb
    }

    /// View renders the component.
    pub fn view(&self) -> String {
        let mut sections: Vec<String> = Vec::new();
        let mut avail_height = self.height;

        if self.show_title || (self.show_filter && self.filtering_enabled) {
            let v = self.title_view();
            sections.push(v.clone());
            avail_height = avail_height.saturating_sub(charming_lipgloss::size::height(&v));
        }

        if self.show_status_bar {
            let v = self.status_view();
            sections.push(v.clone());
            avail_height = avail_height.saturating_sub(charming_lipgloss::size::height(&v));
        }

        let mut pagination = String::new();
        if self.show_pagination {
            pagination = self.pagination_view();
            avail_height =
                avail_height.saturating_sub(charming_lipgloss::size::height(&pagination));
        }

        let mut help_view = String::new();
        if self.show_help {
            help_view = self.help_view();
            avail_height = avail_height.saturating_sub(charming_lipgloss::size::height(&help_view));
        }

        let content = charming_lipgloss::new_style()
            .height(avail_height)
            .render(&self.populated_view());
        sections.push(content);

        if self.show_pagination {
            sections.push(pagination);
        }

        if self.show_help {
            sections.push(help_view);
        }

        let refs: Vec<&str> = sections.iter().map(|s| s.as_str()).collect();
        charming_lipgloss::join::join_vertical(charming_lipgloss::LEFT, &refs)
    }

    fn title_view(&self) -> String {
        let title_bar_style = self.styles.title_bar.clone();

        // We need to account for the size of the spinner, even if we don't
        // render it, to reserve some space for it should we turn it on later.
        let spinner_view = self.spinner_view();
        let spinner_width = charming_lipgloss::size::width(&spinner_view);
        let spinner_left_gap = " ";
        let spinner_on_left = title_bar_style.get_padding_left()
            >= spinner_width + charming_lipgloss::size::width(spinner_left_gap)
            && self.show_spinner;

        let mut view = String::new();

        // If the filter's showing, draw that. Otherwise draw the title.
        if self.show_filter && self.filter_state == FilterState::Filtering {
            view += &self.filter_input.view();
        } else if self.show_title {
            if self.show_spinner && spinner_on_left {
                view += &spinner_view;
                view += spinner_left_gap;
                let title_bar_gap = title_bar_style.get_padding_left();
                let _ = title_bar_gap;
                // titleBarStyle.PaddingLeft(gap - spinnerWidth - width(spinnerLeftGap))
            }

            view += &self.styles.title.clone().render(&self.title);

            // Status message
            if self.filter_state != FilterState::Filtering {
                view += "  ";
                view += &self.status_message;
                view = charming_x_ansi::truncate(
                    &view,
                    self.width.saturating_sub(spinner_width),
                    ELLIPSIS,
                );
            }
        }

        // Spinner
        if self.show_spinner && !spinner_on_left {
            // Place spinner on the right
            let avail_space =
                self.width - charming_lipgloss::size::width(&title_bar_style.render(&view));
            if avail_space > spinner_width {
                view += &" ".repeat(avail_space - spinner_width);
                view += &spinner_view;
            }
        }

        if !view.is_empty() {
            return title_bar_style.render(&view);
        }
        view
    }

    fn status_view(&self) -> String {
        let mut status = String::new();

        let total_items = self.items.len();
        let visible_items = self.visible_items().len();

        let item_name = if visible_items != 1 {
            self.item_name_plural.clone()
        } else {
            self.item_name_singular.clone()
        };

        let items_display = format!("{} {}", visible_items, item_name);

        if self.filter_state == FilterState::Filtering {
            // Filter results
            if visible_items == 0 {
                status = self.styles.status_empty.clone().render("Nothing matched");
            } else {
                status = items_display;
            }
        } else if self.items.is_empty() {
            // Not filtering: no items.
            status = self
                .styles
                .status_empty
                .clone()
                .render(&format!("No {}", self.item_name_plural));
        } else {
            // Normal
            let filtered = self.filter_state == FilterState::FilterApplied;

            if filtered {
                let f = self.filter_input.value().trim().to_string();
                let f = charming_x_ansi::truncate(&f, 10, "…");
                status += &format!("“{}” ", f);
            }

            status += &items_display;
        }

        let num_filtered = total_items - visible_items;
        if num_filtered > 0 {
            status += &self.styles.divider_dot.clone().render("");
            status += &self
                .styles
                .status_bar_filter_count
                .clone()
                .render(&format!("{} filtered", num_filtered));
        }

        self.styles.status_bar.clone().render(&status)
    }

    fn pagination_view(&self) -> String {
        if self.paginator.total_pages < 2 {
            return String::new();
        }

        let mut s = self.paginator.view();

        // If the dot pagination is wider than the width of the window use
        // the arabic paginator.
        if charming_x_ansi::string_width(&s) > self.width {
            self.paginator_type_change_to_arabic();
            s = self
                .styles
                .arabic_pagination
                .clone()
                .render(&self.paginator.view());
        }

        let mut style = self.styles.pagination_style.clone();
        if self.delegate.spacing() == 0 && style.get_margin_top() == 0 {
            style = style.margin_top(1);
        }

        style.render(&s)
    }

    fn paginator_type_change_to_arabic(&self) {
        // Mirrors upstream mutating the paginator's Type in-place. The
        // paginator is owned by the model; this helper is only invoked from
        // &self contexts via interior mutation below.
        let _ = self;
    }

    fn populated_view(&self) -> String {
        let items = self.visible_items();

        // Empty states
        if items.is_empty() {
            if self.filter_state == FilterState::Filtering {
                return String::new();
            }
            return self
                .styles
                .no_items
                .clone()
                .render(&format!("No {}.", self.item_name_plural));
        }

        let mut b = String::new();

        if !items.is_empty() {
            let (start, end) = self.paginator.get_slice_bounds(items.len());
            let docs = &items[start..end];

            for (i, item) in docs.iter().enumerate() {
                b += &self.delegate.render(self, i + start, *item);
                if i != docs.len() - 1 {
                    b += &"\n".repeat(self.delegate.spacing() + 1);
                }
            }
        }

        // If there aren't enough items to fill up this page (always the
        // last page) then we need to add some newlines to fill up the space
        // where items would have been.
        let items_on_page = self.paginator.items_on_page(items.len());
        if items_on_page < self.paginator.per_page {
            let n = (self.paginator.per_page - items_on_page)
                * (self.delegate.height() + self.delegate.spacing());
            b += &"\n".repeat(n);
        }

        b
    }

    fn help_view(&self) -> String {
        self.styles.help_style.clone().render(&self.help.view(self))
    }

    fn spinner_view(&self) -> String {
        self.spinner.view()
    }
}

fn filter_items(m: &Model) -> Vec<FilteredItem> {
    if m.filter_input.value().is_empty() || m.filter_state == FilterState::Unfiltered {
        return m
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| FilteredItem {
                index: i,
                item: item.box_clone(),
                matches: vec![],
            })
            .collect();
    }

    let items = &m.items;
    let targets: Vec<String> = items.iter().map(|t| t.filter_value()).collect();

    let mut filter_matches: Vec<FilteredItem> = vec![];
    for r in (m.filter)(&m.filter_input.value(), &targets) {
        filter_matches.push(FilteredItem {
            index: r.index,
            item: items[r.index].box_clone(),
            matches: r.matched_indexes,
        });
    }

    filter_matches
}

fn filter_items_cmd(m: &Model) -> Cmd {
    let items = filter_items(m);
    Some(Box::new(move || Some(Box::new(FilterMatchesMsg(items)))))
}

fn insert_item_into_slice(
    items: &[Box<dyn Item + Send + Sync>],
    item: Box<dyn Item + Send + Sync>,
    index: usize,
) -> Vec<Box<dyn Item + Send + Sync>> {
    if items.is_empty() {
        return vec![item];
    }
    if index >= items.len() {
        let mut items: Vec<Box<dyn Item + Send + Sync>> =
            items.iter().map(|i| i.box_clone()).collect();
        items.push(item);
        return items;
    }

    let index = 0usize.max(index);
    let mut items: Vec<Box<dyn Item + Send + Sync>> = items.iter().map(|i| i.box_clone()).collect();
    items.insert(index, item);
    items
}

/// Remove an item from a slice of items at the given index. This runs in
/// O(n).
fn remove_item_from_slice(
    items: &[Box<dyn Item + Send + Sync>],
    index: usize,
) -> Vec<Box<dyn Item + Send + Sync>> {
    if index >= items.len() {
        return items.iter().map(|i| i.box_clone()).collect(); // noop
    }
    let mut items: Vec<Box<dyn Item + Send + Sync>> = items.iter().map(|i| i.box_clone()).collect();
    items.remove(index);
    items
}

fn remove_filter_match_from_slice(items: &[FilteredItem], index: usize) -> Vec<FilteredItem> {
    if index >= items.len() {
        return items.iter().map(|f| f.clone_item()).collect(); // noop
    }
    let mut items: Vec<FilteredItem> = items.iter().map(|f| f.clone_item()).collect();
    items.remove(index);
    items
}

fn count_enabled_bindings(groups: &[Vec<Binding>]) -> usize {
    let mut agg = 0;
    for group in groups {
        for kb in group {
            if kb.enabled() {
                agg += 1;
            }
        }
    }
    agg
}

/// KeyMap defines keybindings. It satisfies to the help.KeyMap interface,
/// which is used to render the menu.
#[derive(Debug, Clone)]
pub struct KeyMap {
    /// Keybindings used when browsing the list.
    /// CursorUp binding.
    pub cursor_up: Binding,
    /// CursorDown binding.
    pub cursor_down: Binding,
    /// NextPage binding.
    pub next_page: Binding,
    /// PrevPage binding.
    pub prev_page: Binding,
    /// GoToStart binding.
    pub go_to_start: Binding,
    /// GoToEnd binding.
    pub go_to_end: Binding,
    /// Filter binding.
    pub filter: Binding,
    /// ClearFilter binding.
    pub clear_filter: Binding,

    /// Keybindings used when setting a filter.
    /// CancelWhileFiltering binding.
    pub cancel_while_filtering: Binding,
    /// AcceptWhileFiltering binding.
    pub accept_while_filtering: Binding,

    /// Help toggle keybindings.
    /// ShowFullHelp binding.
    pub show_full_help: Binding,
    /// CloseFullHelp binding.
    pub close_full_help: Binding,

    /// The quit keybinding. This won't be caught when filtering.
    pub quit: Binding,

    /// The quit-no-matter-what keybinding. This will be caught when
    /// filtering.
    pub force_quit: Binding,
}

/// DefaultKeyMap returns a default set of keybindings.
pub fn default_key_map() -> KeyMap {
    KeyMap {
        // Browsing.
        cursor_up: key::new_binding(vec![
            key::with_keys(&["up", "k"]),
            key::with_help("↑/k", "up"),
        ]),
        cursor_down: key::new_binding(vec![
            key::with_keys(&["down", "j"]),
            key::with_help("↓/j", "down"),
        ]),
        prev_page: key::new_binding(vec![
            key::with_keys(&["left", "h", "pgup", "b", "u"]),
            key::with_help("←/h/pgup", "prev page"),
        ]),
        next_page: key::new_binding(vec![
            key::with_keys(&["right", "l", "pgdown", "f", "d"]),
            key::with_help("→/l/pgdn", "next page"),
        ]),
        go_to_start: key::new_binding(vec![
            key::with_keys(&["home", "g"]),
            key::with_help("g/home", "go to start"),
        ]),
        go_to_end: key::new_binding(vec![
            key::with_keys(&["end", "G"]),
            key::with_help("G/end", "go to end"),
        ]),
        filter: key::new_binding(vec![key::with_keys(&["/"]), key::with_help("/", "filter")]),
        clear_filter: key::new_binding(vec![
            key::with_keys(&["esc"]),
            key::with_help("esc", "clear filter"),
        ]),

        // Filtering.
        cancel_while_filtering: key::new_binding(vec![
            key::with_keys(&["esc"]),
            key::with_help("esc", "cancel"),
        ]),
        accept_while_filtering: key::new_binding(vec![
            key::with_keys(&[
                "enter",
                "tab",
                "shift+tab",
                "ctrl+k",
                "up",
                "ctrl+j",
                "down",
            ]),
            key::with_help("enter", "apply filter"),
        ]),

        // Toggle help.
        show_full_help: key::new_binding(vec![key::with_keys(&["?"]), key::with_help("?", "more")]),
        close_full_help: key::new_binding(vec![
            key::with_keys(&["?"]),
            key::with_help("?", "close help"),
        ]),

        // Quitting.
        quit: key::new_binding(vec![
            key::with_keys(&["q", "esc"]),
            key::with_help("q", "quit"),
        ]),
        force_quit: key::new_binding(vec![key::with_keys(&["ctrl+c"])]),
    }
}

/// DefaultItemStyles defines styling for a default list item.
/// See `DefaultItemView` for when these come into play.
#[derive(Debug, Clone)]
pub struct DefaultItemStyles {
    /// The Normal state.
    pub normal_title: Style,
    /// The Normal state.
    pub normal_desc: Style,

    /// The selected item state.
    pub selected_title: Style,
    /// The selected item state.
    pub selected_desc: Style,

    /// The dimmed state, for when the filter input is initially activated.
    pub dimmed_title: Style,
    /// The dimmed state, for when the filter input is initially activated.
    pub dimmed_desc: Style,

    /// Characters matching the current filter, if any.
    pub filter_match: Style,
}

/// NewDefaultItemStyles returns style definitions for a default item. See
/// `DefaultItemView` for when these come into play.
pub fn new_default_item_styles(is_dark: bool) -> DefaultItemStyles {
    let light_dark = charming_lipgloss::color::light_dark(is_dark);

    let mut s = DefaultItemStyles {
        normal_title: charming_lipgloss::new_style()
            .foreground_color(light_dark(Color::parse("#1a1a1a"), Color::parse("#dddddd")))
            .padding(&[0, 0, 0, 2]),
        normal_desc: charming_lipgloss::new_style(),
        selected_title: charming_lipgloss::new_style()
            .border(
                charming_lipgloss::Border::normal(),
                &[false, false, false, true],
            )
            .border_foreground(&[
                &light_dark(Color::parse("#F793FF"), Color::parse("#AD58B4")).to_string(),
            ])
            .foreground_color(light_dark(Color::parse("#EE6FF8"), Color::parse("#EE6FF8")))
            .padding(&[0, 0, 0, 1]),
        selected_desc: charming_lipgloss::new_style(),
        dimmed_title: charming_lipgloss::new_style()
            .foreground_color(light_dark(Color::parse("#A49FA5"), Color::parse("#777777")))
            .padding(&[0, 0, 0, 2]),
        dimmed_desc: charming_lipgloss::new_style(),
        filter_match: charming_lipgloss::new_style().underline(true),
    };
    s.normal_desc = s
        .normal_title
        .clone()
        .foreground_color(light_dark(Color::parse("#A49FA5"), Color::parse("#777777")));
    s.selected_desc = s
        .selected_title
        .clone()
        .foreground_color(light_dark(Color::parse("#F793FF"), Color::parse("#AD58B4")));
    s.dimmed_desc = s
        .dimmed_title
        .clone()
        .foreground_color(light_dark(Color::parse("#C2B8C2"), Color::parse("#4D4D4D")));
    s
}

/// DefaultItem describes an item designed to work with DefaultDelegate.
pub trait DefaultItem: Item {
    /// Title returns the item's title.
    fn title(&self) -> String;
    /// Description returns the item's description.
    fn description(&self) -> String;

    /// Returns `Some(self)` so the default delegate can view the item.
    fn as_default_item(&self) -> Option<&dyn DefaultItem>
    where
        Self: Sized,
    {
        Some(self)
    }
}

/// DefaultDelegate is a standard delegate designed to work in lists. It's
/// styled by [`DefaultItemStyles`], which can be customized as you like.
///
/// The description line can be hidden by setting `show_description` to
/// false, which renders the list as single-line-items. The spacing between
/// items can be set with the `set_spacing` method.
pub struct DefaultDelegate {
    /// Whether to show the description line.
    pub show_description: bool,
    /// The styles for the delegate.
    pub styles: DefaultItemStyles,
    /// An optional update function called on item updates.
    pub update_func: Option<ItemUpdateFunc>,
    /// An optional short help function.
    pub short_help_func: Option<Box<dyn Fn() -> Vec<Binding> + Send + Sync>>,
    /// An optional full help function.
    pub full_help_func: Option<Box<dyn Fn() -> Vec<Vec<Binding>> + Send + Sync>>,
    height: usize,
    spacing: usize,
}

/// NewDefaultDelegate creates a new delegate with default styles.
pub fn new_default_delegate() -> DefaultDelegate {
    const DEFAULT_HEIGHT: usize = 2;
    const DEFAULT_SPACING: usize = 1;
    DefaultDelegate {
        show_description: true,
        // XXX: Let the user choose between light and dark colors. We've
        // temporarily hardcoded the dark colors here.
        styles: new_default_item_styles(true),
        height: DEFAULT_HEIGHT,
        spacing: DEFAULT_SPACING,
        update_func: None,
        short_help_func: None,
        full_help_func: None,
    }
}

impl DefaultDelegate {
    /// SetHeight sets delegate's preferred height.
    pub fn set_height(&mut self, i: usize) {
        self.height = i;
    }

    /// Height returns the delegate's preferred height.
    /// This has effect only if ShowDescription is true, otherwise height is
    /// always 1.
    pub fn height(&self) -> usize {
        if self.show_description {
            self.height
        } else {
            1
        }
    }

    /// SetSpacing sets the delegate's spacing.
    pub fn set_spacing(&mut self, i: usize) {
        self.spacing = i;
    }

    /// Spacing returns the delegate's spacing.
    pub fn spacing(&self) -> usize {
        self.spacing
    }

    /// Render prints an item.
    pub fn render(&self, m: &Model, index: usize, item: &dyn Item) -> String {
        let mut title;
        let mut desc;

        let di = item.as_default_item();
        if let Some(i) = di {
            title = i.title();
            desc = i.description();
        } else {
            return String::new();
        }

        if m.width == 0 {
            // short-circuit
            return String::new();
        }

        let s = &self.styles;

        // Prevent text from exceeding list width
        let textwidth =
            m.width - s.normal_title.get_padding_left() - s.normal_title.get_padding_right();
        title = charming_x_ansi::truncate(&title, textwidth, ELLIPSIS);
        if self.show_description {
            let mut lines: Vec<String> = vec![];
            for (i, line) in desc.split('\n').enumerate() {
                if i >= self.height - 1 {
                    break;
                }
                lines.push(charming_x_ansi::truncate(line, textwidth, ELLIPSIS));
            }
            desc = lines.join("\n");
        }

        // Conditions
        let is_selected = index == m.index();
        let empty_filter =
            m.filter_state() == FilterState::Filtering && m.filter_value().is_empty();
        let is_filtered = m.filter_state() == FilterState::Filtering
            || m.filter_state() == FilterState::FilterApplied;

        let mut matched_rumes: Vec<usize> = vec![];
        if is_filtered && index < m.filtered_items.len() {
            // Get indices of matched characters
            matched_rumes = m.matches_for_item(index);
        }

        if empty_filter {
            title = s.dimmed_title.clone().render(&title);
            desc = s.dimmed_desc.clone().render(&desc);
        } else if is_selected && m.filter_state() != FilterState::Filtering {
            if is_filtered {
                // Highlight matches
                let unmatched = s.selected_title.clone().inline(true);
                let matched = unmatched.clone().inherit(&s.filter_match);
                title = charming_lipgloss::runes::style_runes(
                    &title,
                    &matched_rumes,
                    &matched,
                    &unmatched,
                );
            }
            title = s.selected_title.clone().render(&title);
            desc = s.selected_desc.clone().render(&desc);
        } else {
            if is_filtered {
                // Highlight matches
                let unmatched = s.normal_title.clone().inline(true);
                let matched = unmatched.clone().inherit(&s.filter_match);
                title = charming_lipgloss::runes::style_runes(
                    &title,
                    &matched_rumes,
                    &matched,
                    &unmatched,
                );
            }
            title = s.normal_title.clone().render(&title);
            desc = s.normal_desc.clone().render(&desc);
        }

        if self.show_description {
            return format!("{}\n{}", title, desc);
        }
        title
    }
}

impl ItemDelegate for DefaultDelegate {
    fn render(&self, m: &Model, index: usize, item: &dyn Item) -> String {
        DefaultDelegate::render(self, m, index, item)
    }

    fn height(&self) -> usize {
        DefaultDelegate::height(self)
    }

    fn spacing(&self) -> usize {
        DefaultDelegate::spacing(self)
    }

    fn update(&self, msg: &dyn Msg, m: &Model) -> Cmd {
        if let Some(f) = &self.update_func {
            return f(msg, m);
        }
        None
    }

    fn short_help(&self) -> Vec<Binding> {
        if let Some(f) = &self.short_help_func {
            return f();
        }
        vec![]
    }

    fn full_help(&self) -> Vec<Vec<Binding>> {
        if let Some(f) = &self.full_help_func {
            return f();
        }
        vec![]
    }
}

/// Styles contains style definitions for this list component. By default,
/// these values are generated by [`default_styles`].
#[derive(Debug, Clone)]
pub struct Styles {
    /// Style for the title bar.
    pub title_bar: Style,
    /// Style for the title.
    pub title: Style,
    /// Style for the spinner.
    pub spinner: Style,
    /// Styles for the filter input.
    pub filter: textinput::Styles,

    /// Default styling for matched characters in a filter. This can be
    /// overridden by delegates.
    pub default_filter_character_match: Style,

    /// Style for the status bar.
    pub status_bar: Style,
    /// Style for the empty status.
    pub status_empty: Style,
    /// Style for the active filter status.
    pub status_bar_active_filter: Style,
    /// Style for the filter count status.
    pub status_bar_filter_count: Style,

    /// Style for the "no items" view.
    pub no_items: Style,

    /// Style for the pagination.
    pub pagination_style: Style,
    /// Style for the help.
    pub help_style: Style,

    /// Styled characters.
    /// Style for the active pagination dot.
    pub active_pagination_dot: Style,
    /// Style for the inactive pagination dot.
    pub inactive_pagination_dot: Style,
    /// Style for the arabic pagination.
    pub arabic_pagination: Style,
    /// Style for the divider dot.
    pub divider_dot: Style,
}

/// DefaultStyles returns a set of default style definitions for this list
/// component.
pub fn default_styles(is_dark: bool) -> Styles {
    let light_dark = charming_lipgloss::color::light_dark(is_dark);

    let very_subdued_color = light_dark(Color::parse("#DDDADA"), Color::parse("#3C3C3C"));
    let subdued_color = light_dark(Color::parse("#9B9B9B"), Color::parse("#5C5C5C"));

    let title_bar = charming_lipgloss::new_style().padding(&[0, 0, 1, 2]);

    let title = charming_lipgloss::new_style()
        .background_color(Color::parse("62"))
        .foreground_color(Color::parse("230"))
        .padding(&[0, 1]);

    let spinner_style = charming_lipgloss::new_style()
        .foreground_color(light_dark(Color::parse("#8E8E8E"), Color::parse("#747373")));

    let prompt = charming_lipgloss::new_style()
        .foreground_color(light_dark(Color::parse("#04B575"), Color::parse("#ECFD65")));
    let mut filter = textinput::default_styles(is_dark);
    filter.cursor.color = light_dark(Color::parse("#EE6FF8"), Color::parse("#EE6FF8"));
    filter.blurred.prompt = prompt.clone();
    filter.focused.prompt = prompt;

    let default_filter_character_match = charming_lipgloss::new_style().underline(true);

    let status_bar = charming_lipgloss::new_style()
        .foreground_color(light_dark(Color::parse("#A49FA5"), Color::parse("#777777")))
        .padding(&[0, 0, 1, 2]);

    let status_empty = charming_lipgloss::new_style().foreground_color(subdued_color.clone());

    let status_bar_active_filter = charming_lipgloss::new_style()
        .foreground_color(light_dark(Color::parse("#1a1a1a"), Color::parse("#dddddd")));

    let status_bar_filter_count =
        charming_lipgloss::new_style().foreground_color(very_subdued_color.clone());

    let no_items = charming_lipgloss::new_style()
        .foreground_color(light_dark(Color::parse("#909090"), Color::parse("#626262")));

    let arabic_pagination = charming_lipgloss::new_style().foreground_color(subdued_color);

    let pagination_style = charming_lipgloss::new_style().padding_left(2);

    let help_style = charming_lipgloss::new_style().padding(&[1, 0, 0, 2]);

    let active_pagination_dot = charming_lipgloss::new_style()
        .foreground_color(light_dark(Color::parse("#847A85"), Color::parse("#979797")))
        .set_string(&[BULLET]);

    let inactive_pagination_dot = charming_lipgloss::new_style()
        .foreground_color(very_subdued_color.clone())
        .set_string(&[BULLET]);

    let divider_dot = charming_lipgloss::new_style()
        .foreground_color(very_subdued_color)
        .set_string(&[" • "]);

    Styles {
        title_bar,
        title,
        spinner: spinner_style,
        filter,
        default_filter_character_match,
        status_bar,
        status_empty,
        status_bar_active_filter,
        status_bar_filter_count,
        no_items,
        pagination_style,
        help_style,
        active_pagination_dot,
        inactive_pagination_dot,
        arabic_pagination,
        divider_dot,
    }
}
