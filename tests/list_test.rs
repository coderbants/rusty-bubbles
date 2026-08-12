//! Cleanroom Rust port of upstream Go source file: `list/list_test.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! List status bar, filter state and filter text behavior tests. The
//! upstream tests assert on the private `statusView` helper; here the
//! assertions run against the full rendered view, which contains the
//! status bar.

use charming_bubbles::list::{self, FilterState, Item, ItemDelegate, Model};
use charming_bubbletea::model::{Cmd, Msg};

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
    let items: Vec<Box<dyn Item + Send + Sync>> =
        items.into_iter().map(|i| Box::new(i) as Box<dyn Item + Send + Sync>).collect();
    list::new(items, Box::new(ItemDelegate_), 10, 10)
}

#[test]
fn test_status_bar_item_name() {
    let mut list = new_list(vec![Item_("foo".into()), Item_("bar".into())]);
    let expected = "2 items";
    assert!(list.view().contains(expected), "expected view to contain {expected}");

    list.set_items(vec![Box::new(Item_("foo".into()))]);
    let expected = "1 item";
    assert!(list.view().contains(expected), "expected view to contain {expected}");
}

#[test]
fn test_status_bar_without_items() {
    let list = new_list(vec![]);
    let expected = "No items";
    assert!(list.view().contains(expected), "expected view to contain {expected}");
}

#[test]
fn test_custom_status_bar_item_name() {
    let mut list = new_list(vec![Item_("foo".into()), Item_("bar".into())]);
    list.set_status_bar_item_name("connection", "connections");

    let expected = "2 connections";
    assert!(list.view().contains(expected), "expected view to contain {expected}");

    list.set_items(vec![Box::new(Item_("foo".into()))]);
    let expected = "1 connection";
    assert!(list.view().contains(expected), "expected view to contain {expected}");

    list.set_items(vec![]);
    let expected = "No connections";
    assert!(list.view().contains(expected), "expected view to contain {expected}");
}

#[test]
fn test_set_filter_text() {
    let tc: Vec<Item_> = vec![Item_("foo".into()), Item_("bar".into()), Item_("baz".into())];

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
    let tc: Vec<Item_> = vec![Item_("foo".into()), Item_("bar".into()), Item_("baz".into())];

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
    assert!(footer.contains(expected), "expected view to contain '{expected}'");
}
