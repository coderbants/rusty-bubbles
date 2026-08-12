//! Cleanroom Rust port of upstream Go source file:
//! `internal/memoization/memoization_test.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! LRU memo-cache behavior tests: set/get semantics, nil values, eviction
//! and access-order tracking, plus a deterministic replay of the upstream
//! fuzz seeds.

use charming_bubbles::internal::memoization::{self, HInt, HString};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
enum ActionType {
    Set,
    Get,
}

struct CacheAction {
    action_type: ActionType,
    key: String,
    value: Option<String>,
    expected_value: Option<String>,
}

impl CacheAction {
    fn set(key: &str, value: &str) -> CacheAction {
        CacheAction {
            action_type: ActionType::Set,
            key: key.to_string(),
            value: Some(value.to_string()),
            expected_value: None,
        }
    }
    fn set_nil(key: &str) -> CacheAction {
        CacheAction {
            action_type: ActionType::Set,
            key: key.to_string(),
            value: None,
            expected_value: None,
        }
    }
    fn get(key: &str, expected: Option<&str>) -> CacheAction {
        CacheAction {
            action_type: ActionType::Get,
            key: key.to_string(),
            value: None,
            expected_value: expected.map(|s| s.to_string()),
        }
    }
}

struct TestCase {
    name: String,
    capacity: usize,
    actions: Vec<CacheAction>,
}

#[test]
fn test_cache() {
    let tests: Vec<TestCase> = vec![
        TestCase {
            name: "TestNewMemoCache".into(),
            capacity: 5,
            actions: vec![CacheAction::get("", None)],
        },
        TestCase {
            name: "TestSetAndGet".into(),
            capacity: 10,
            actions: vec![
                CacheAction::set("key1", "value1"),
                CacheAction::get("key1", Some("value1")),
                CacheAction::set("key1", "newValue1"),
                CacheAction::get("key1", Some("newValue1")),
                CacheAction::get("nonExistentKey", None),
                CacheAction::set("nilKey", ""),
                CacheAction::get("nilKey", Some("")),
                CacheAction::set("keyA", "valueA"),
                CacheAction::set("keyB", "valueB"),
                CacheAction::get("keyA", Some("valueA")),
                CacheAction::get("keyB", Some("valueB")),
            ],
        },
        TestCase {
            name: "TestSetNilValue".into(),
            capacity: 10,
            actions: vec![
                CacheAction::set_nil("nilKey"),
                CacheAction::get("nilKey", Some("")),
            ],
        },
        TestCase {
            name: "TestGetAfterEviction".into(),
            capacity: 2,
            actions: vec![
                CacheAction::set("1", "1"),
                CacheAction::set("2", "2"),
                CacheAction::set("3", "3"),
                CacheAction::get("1", None),
                CacheAction::get("2", Some("2")),
            ],
        },
        TestCase {
            name: "TestGetAfterLRU".into(),
            capacity: 2,
            actions: vec![
                CacheAction::set("1", "1"),
                CacheAction::set("2", "2"),
                CacheAction::get("1", Some("1")),
                CacheAction::set("3", "3"),
                CacheAction::get("1", Some("1")),
                CacheAction::get("3", Some("3")),
                CacheAction::get("2", None),
            ],
        },
        TestCase {
            name: "TestLRU_Capacity3".into(),
            capacity: 3,
            actions: vec![
                CacheAction::set("1", "1"),
                CacheAction::set("2", "2"),
                CacheAction::set("3", "3"),
                CacheAction::get("1", Some("1")),
                CacheAction::set("4", "4"),
                CacheAction::get("2", None),
                CacheAction::get("1", Some("1")),
                CacheAction::get("3", Some("3")),
                CacheAction::get("4", Some("4")),
            ],
        },
        TestCase {
            name: "TestLRU_VaryingAccesses".into(),
            capacity: 3,
            actions: vec![
                CacheAction::set("1", "1"),
                CacheAction::set("2", "2"),
                CacheAction::set("3", "3"),
                CacheAction::get("1", Some("1")),
                CacheAction::get("2", Some("2")),
                CacheAction::set("4", "4"),
                CacheAction::get("3", None),
                CacheAction::get("1", Some("1")),
                CacheAction::get("2", Some("2")),
                CacheAction::get("4", Some("4")),
            ],
        },
    ];

    for tt in tests {
        let mut cache: memoization::MemoCache<String> = memoization::new_memo_cache(tt.capacity);
        for action in &tt.actions {
            let key = HString(action.key.clone());
            match action.action_type {
                ActionType::Set => {
                    cache.set(&key, action.value.clone().unwrap_or_default());
                }
                ActionType::Get => {
                    let got = cache.get(&key).cloned();
                    let want = action.expected_value.clone().unwrap_or_default();
                    assert_eq!(
                        got.unwrap_or_default(),
                        want,
                        "{}: Get({})",
                        tt.name,
                        action.key
                    );
                }
            }
        }
    }
}

/// Deterministic replay of the upstream `FuzzCache` seed corpus.
#[test]
fn fuzz_cache_seeds() {
    let seeds: Vec<Vec<u8>> = vec![
        b"7\x010\x0000000020".to_vec(),
        vec![0, 0, 0, 0],
        vec![1, 0, 0, 1],
        vec![2, 0],
    ];

    for seed in seeds {
        if seed.is_empty() {
            continue;
        }
        let mut cache: memoization::MemoCache<i64> = memoization::new_memo_cache(10);
        let mut expected_values: HashMap<i64, i64> = HashMap::new();
        let mut access_order: Vec<i64> = Vec::new();

        let mut i = 0;
        while i < seed.len() {
            let op_code = seed[i] % 4;
            i += 1;
            match op_code {
                0 | 1 => {
                    if i + 3 > seed.len() {
                        break;
                    }
                    let key = u16::from_be_bytes([seed[i], seed[i + 1]]) as i64;
                    let value = seed[i + 2] as i64;
                    i += 3;
                    access_order.retain(|k| *k != key);
                    cache.set(&HInt(key), value);
                    expected_values.insert(key, value);
                    access_order.push(key);
                    if access_order.len() > cache.capacity() {
                        let evicted = access_order.remove(0);
                        expected_values.remove(&evicted);
                    }
                }
                2 => {
                    if i >= seed.len() {
                        break;
                    }
                    let key = seed[i] as i64;
                    i += 1;
                    let expected = expected_values.get(&key).copied().unwrap_or(0);
                    if expected_values.contains_key(&key) {
                        access_order.retain(|k| *k != key);
                        access_order.push(key);
                    }
                    let got = cache.get(&HInt(key)).copied().unwrap_or(0);
                    assert_eq!(got, expected, "Get({key}) = {got}, want {expected}");
                }
                3 => {
                    if i >= seed.len() {
                        break;
                    }
                    let new_size = seed[i] as usize;
                    i += 1;
                    if new_size == 0 {
                        break;
                    }
                    cache = memoization::new_memo_cache(new_size);
                    expected_values.clear();
                    access_order.clear();
                }
                _ => unreachable!(),
            }
        }
    }
}
