//! Cleanroom Rust port of upstream Go source file: `list/list_test.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! List status bar, filter state and filter text behavior tests. The
//! upstream tests assert on the private `statusView` helper; here the
//! assertions run against the full rendered view, which contains the
//! status bar.

use rusty_bubbles::list::{
    self, new, new_default_delegate, DefaultItem, FilterState, Item, ItemDelegate, Model,
};
use rusty_bubbletea::model::{Cmd, Msg};

#[derive(Debug)]
struct Item_(String);

impl Item for Item_ {
    fn filter_value(&self) -> String {
        self.0.clone()
    }
    fn box_clone(&self) -> Box<dyn Item + Send + Sync> {
        Box::new(Item_(self.0.clone()))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct ItemDelegate_;

impl ItemDelegate for ItemDelegate_ {
    fn height(&self) -> usize {
        1
    }
    fn spacing(&self) -> usize {
        0
    }
    fn update(&self, _msg: &dyn Msg, _m: &Model) -> Cmd {
        None
    }
    fn render(&self, m: &Model, index: usize, item: &dyn Item) -> String {
        let i = item.filter_value();
        let str = format!("{}. {}", index + 1, i);
        m.styles.title_bar.clone().render(&str)
    }
}

fn new_list(items: Vec<Item_>) -> Model {
    let items: Vec<Box<dyn Item + Send + Sync>> = items
        .into_iter()
        .map(|i| Box::new(i) as Box<dyn Item + Send + Sync>)
        .collect();
    list::new(items, Box::new(ItemDelegate_), 10, 10)
}

#[test]
fn test_status_bar_item_name() {
    let mut list = new_list(vec![Item_("foo".into()), Item_("bar".into())]);
    let expected = "2 items";
    assert!(
        list.view().contains(expected),
        "expected view to contain {expected}"
    );

    list.set_items(vec![Box::new(Item_("foo".into()))]);
    let expected = "1 item";
    assert!(
        list.view().contains(expected),
        "expected view to contain {expected}"
    );
}

#[test]
fn test_status_bar_without_items() {
    let list = new_list(vec![]);
    let expected = "No items";
    assert!(
        list.view().contains(expected),
        "expected view to contain {expected}"
    );
}

#[test]
fn test_custom_status_bar_item_name() {
    let mut list = new_list(vec![Item_("foo".into()), Item_("bar".into())]);
    list.set_status_bar_item_name("connection", "connections");

    let expected = "2 connections";
    assert!(
        list.view().contains(expected),
        "expected view to contain {expected}"
    );

    list.set_items(vec![Box::new(Item_("foo".into()))]);
    let expected = "1 connection";
    assert!(
        list.view().contains(expected),
        "expected view to contain {expected}"
    );

    list.set_items(vec![]);
    let expected = "No connections";
    assert!(
        list.view().contains(expected),
        "expected view to contain {expected}"
    );
}

#[test]
fn test_set_filter_text() {
    let tc: Vec<Item_> = vec![
        Item_("foo".into()),
        Item_("bar".into()),
        Item_("baz".into()),
    ];

    let mut list = new_list(tc);
    list.set_filter_text("ba");

    list.set_filter_state(FilterState::Unfiltered);
    let expected: Vec<String> = vec!["foo".into(), "bar".into(), "baz".into()];
    let vis: Vec<&dyn Item> = list.visible_items();
    let got: Vec<String> = vis.iter().map(|i| i.filter_value()).collect();
    assert_eq!(got, expected, "expected view to contain only {expected:?}");

    list.set_filter_state(FilterState::Filtering);
    let expected: Vec<String> = vec!["bar".into(), "baz".into()];
    let vis: Vec<&dyn Item> = list.visible_items();
    let got: Vec<String> = vis.iter().map(|i| i.filter_value()).collect();
    assert_eq!(got, expected, "expected view to contain only {expected:?}");

    list.set_filter_state(FilterState::FilterApplied);
    let vis: Vec<&dyn Item> = list.visible_items();
    let got: Vec<String> = vis.iter().map(|i| i.filter_value()).collect();
    assert_eq!(got, expected, "expected view to contain only {expected:?}");
}

#[test]
fn test_set_filter_state() {
    let tc: Vec<Item_> = vec![
        Item_("foo".into()),
        Item_("bar".into()),
        Item_("baz".into()),
    ];

    let mut list = new_list(tc);
    list.set_filter_text("ba");

    list.set_filter_state(FilterState::Unfiltered);
    let (expected, not_expected) = ("up", "clear filter");
    let view = list.view();
    let lines: Vec<&str> = view.split('\n').collect();
    let footer = lines[lines.len() - 1];
    assert!(
        footer.contains(expected) && !footer.contains(not_expected),
        "expected view to contain '{expected}' not '{not_expected}'"
    );

    list.set_filter_state(FilterState::Filtering);
    let (expected, not_expected) = ("filter", "more");
    let view = list.view();
    let lines: Vec<&str> = view.split('\n').collect();
    let footer = lines[lines.len() - 1];
    assert!(
        footer.contains(expected) && !footer.contains(not_expected),
        "expected view to contain '{expected}' not '{not_expected}'"
    );

    list.set_filter_state(FilterState::FilterApplied);
    let expected = "clear";
    let view = list.view();
    let lines: Vec<&str> = view.split('\n').collect();
    let footer = lines[lines.len() - 1];
    assert!(
        footer.contains(expected),
        "expected view to contain '{expected}'"
    );
}

#[test]
fn test_list_navigation_and_items() {
    use rusty_bubbletea::key::{Key, KeyMod, KeyPressMsg};

    let items = vec![
        Item_("Apple".into()),
        Item_("Banana".into()),
        Item_("Cherry".into()),
        Item_("Date".into()),
        Item_("Elderberry".into()),
    ];
    let mut list = new_list(items);
    list.title = "Fruits".to_string();
    assert_eq!(list.index(), 0);
    assert_eq!(list.selected_item().unwrap().filter_value(), "Apple");

    // Select down / up
    list.select(2);
    assert_eq!(list.index(), 2);
    assert_eq!(list.selected_item().unwrap().filter_value(), "Cherry");

    list.cursor_up();
    assert_eq!(list.index(), 1);

    list.cursor_down();
    assert_eq!(list.index(), 2);

    // Key update
    list.update(&KeyPressMsg(Key::new('k', "k", KeyMod::default())));
    assert_eq!(list.index(), 1);

    list.update(&KeyPressMsg(Key::new('j', "j", KeyMod::default())));
    assert_eq!(list.index(), 2);

    // Reset selected item & set items
    list.set_items(vec![
        Box::new(Item_("X".into())),
        Box::new(Item_("Y".into())),
    ]);
    assert_eq!(list.items().len(), 2);

    // Insert item & remove item
    list.insert_item(1, Box::new(Item_("Inserted".into())));
    assert_eq!(list.items().len(), 3);
    list.remove_item(1);
    assert_eq!(list.items().len(), 2);

    // Set item
    list.set_item(0, Box::new(Item_("Z".into())));
    assert_eq!(list.visible_items()[0].filter_value(), "Z");

    // Infinite scrolling
    list.infinite_scrolling = true;
    list.select(0);
    list.cursor_up();
    assert_eq!(list.index(), 1);
    list.cursor_down();
    assert_eq!(list.index(), 0);

    // Toggle spinner
    let cmd = list.toggle_spinner();
    assert!(cmd.is_some());
    assert!(list.show_spinner);
    let cmd2 = list.toggle_spinner();
    assert!(cmd2.is_none());
    assert!(!list.show_spinner);

    // Filter value & state methods
    assert_eq!(list.filter_state().to_string(), "unfiltered");
    assert!(!list.setting_filter());
    assert!(!list.is_filtered());

    #[derive(Debug)]
    struct TestItem {
        title: String,
        desc: String,
    }
    impl Item for TestItem {
        fn filter_value(&self) -> String {
            self.title.clone()
        }
        fn as_default_item(&self) -> Option<&dyn DefaultItem> {
            Some(self)
        }
        fn box_clone(&self) -> Box<dyn Item + Send + Sync> {
            Box::new(TestItem {
                title: self.title.clone(),
                desc: self.desc.clone(),
            })
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
    impl DefaultItem for TestItem {
        fn title(&self) -> String {
            self.title.clone()
        }
        fn description(&self) -> String {
            self.desc.clone()
        }
    }

    let delegate = new_default_delegate();
    let mut list2 = new(
        vec![
            Box::new(TestItem {
                title: "Foo".into(),
                desc: "Foo desc".into(),
            }),
            Box::new(TestItem {
                title: "Bar".into(),
                desc: "Bar desc".into(),
            }),
        ],
        Box::new(delegate),
        40,
        20,
    );
    let v = list2.view();
    assert!(v.contains("Foo"));
    assert!(v.contains("Foo desc"));

    // Enter filtering with '/'
    list2.update(&KeyPressMsg(Key::new('/', "/", KeyMod::default())));
    assert_eq!(list2.filter_state(), FilterState::Filtering);
    assert!(list2.setting_filter());

    // Type filter chars
    list2.update(&KeyPressMsg(Key::new('F', "F", KeyMod::default())));
    list2.update(&KeyPressMsg(Key::new('o', "o", KeyMod::default())));
    assert_eq!(list2.filter_value(), "Fo");

    // Accept filter with enter
    list2.update(&KeyPressMsg(Key::new(
        rusty_bubbletea::key::KEY_ENTER,
        "enter",
        KeyMod::default(),
    )));
    assert_eq!(list2.filter_state(), FilterState::FilterApplied);
    assert!(list2.is_filtered());

    // Clear filter with esc
    list2.update(&KeyPressMsg(Key::new('\x1b', "esc", KeyMod::default())));
    assert_eq!(list2.filter_state(), FilterState::Unfiltered);

    // Full help toggle with '?'
    list2.update(&KeyPressMsg(Key::new('?', "?", KeyMod::default())));
    assert!(list2.help.show_all);
    list2.update(&KeyPressMsg(Key::new('?', "?", KeyMod::default())));
    assert!(!list2.help.show_all);
}
