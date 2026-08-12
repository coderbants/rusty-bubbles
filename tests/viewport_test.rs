//! Cleanroom Rust port of upstream Go source file: `viewport/viewport_test.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! Viewport navigation, highlight matching, sizing and golden-output tests.
//! The upstream tests are white-box; where the Rust port keeps internals
//! private, assertions are adapted to the public API (`x_offset()`,
//! `set_content_lines`, `highlights()`, rendered `view()`).

mod common;

use charming_bubbles::viewport::{self, GutterContext, HighlightInfo};
use charming_lipgloss::border::Border;
use charming_x_ansi::util;
use std::collections::HashMap;


const TEXT_CONTENT_LIST: &str = "57 Precepts of narcissistic comedy character Zote from an awesome \"Hollow knight\" game (https://store.steampowered.com/app/367520/Hollow_Knight/).
Precept One: 'Always Win Your Battles'. Losing a battle earns you nothing and teaches you nothing. Win your battles, or don't engage in them at all!

Precept Two: 'Never Let Them Laugh at You'. Fools laugh at everything, even at their superiors. But beware, laughter isn't harmless! Laughter spreads like a disease, and soon everyone is laughing at you. You need to strike at the source of this perverse merriment quickly to stop it from spreading.
Precept Three: 'Always Be Rested'. Fighting and adventuring take their toll on your body. When you rest, your body strengthens and repairs itself. The longer you rest, the stronger you become.
Precept Four: 'Forget Your Past'. The past is painful, and thinking about your past can only bring you misery. Think about something else instead, such as the future, or some food.
Precept Five: 'Strength Beats Strength'. Is your opponent strong? No matter! Simply overcome their strength with even more strength, and they'll soon be defeated.
Precept Six: 'Choose Your Own Fate'. Our elders teach that our fate is chosen for us before we are even born. I disagree.
Precept Seven: 'Mourn Not the Dead'. When we die, do things get better for us or worse? There's no way to tell, so we shouldn't bother mourning. Or celebrating for that matter.
Precept Eight: 'Travel Alone'. You can rely on nobody, and nobody will always be loyal. Therefore, nobody should be your constant companion.
Precept Nine: 'Keep Your Home Tidy'. Your home is where you keep your most prized possession - yourself. Therefore, you should make an effort to keep it nice and clean.
Precept Ten: 'Keep Your Weapon Sharp'. I make sure that my weapon, 'Life Ender', is kept well-sharpened at all times. This makes it much easier to cut things.
Precept Eleven: 'Mothers Will Always Betray You'. This Precept explains itself.
Precept Twelve: 'Keep Your Cloak Dry'. If your cloak gets wet, dry it as soon as you can. Wearing wet cloaks is unpleasant, and can lead to illness.
Precept Thirteen: 'Never Be Afraid'. Fear can only hold you back. Facing your fears can be a tremendous effort. Therefore, you should just not be afraid in the first place.
Precept Fourteen: 'Respect Your Superiors'. If someone is your superior in strength or intellect or both, you need to show them your respect. Don't ignore them or laugh at them.
Precept Fifteen: 'One Foe, One Blow'. You should only use a single blow to defeat an enemy. Any more is a waste. Also, by counting your blows as you fight, you'll know how many foes you've defeated.";

fn new_vt(width: usize, height: usize) -> viewport::Model {
    viewport::new(vec![viewport::with_width(width), viewport::with_height(height)])
}

#[test]
fn test_new() {
    let m = new_vt(10, 10);
    assert_eq!(m.mouse_wheel_delta, 3, "default MouseWheelDelta should be 3, got {}", m.mouse_wheel_delta);
    assert!(m.mouse_wheel_enabled, "mouse wheel should be enabled by default");
}

#[test]
fn test_set_horizontal_step() {
    let mut m = new_vt(10, 10);
    // Default step: scrolling right by the step from x=0 has no effect.
    m.set_content("Some line that is longer than width");
    m.scroll_right(6);
    assert_eq!(m.x_offset(), 6, "default step scroll should move 6 columns");

    m.set_horizontal_step(8);
    m.set_x_offset(0);
    m.scroll_right(8);
    assert_eq!(m.x_offset(), 8, "horizontalStep should be 8, got {}", m.x_offset());

    // No negative step: setting 0 means scrolling does nothing.
    m.set_horizontal_step(0);
    m.set_x_offset(0);
    m.scroll_right(0);
    assert_eq!(m.x_offset(), 0, "horizontalStep should be 0, got {}", m.x_offset());
}

#[test]
fn test_move_left() {
    let zero_position = 0;

    // Zero position: scrolling left at offset 0 stays at 0.
    let mut m = new_vt(10, 10);
    assert_eq!(m.x_offset(), zero_position, "default indent should be {}, got {}", zero_position, m.x_offset());
    m.scroll_left(6);
    assert_eq!(m.x_offset(), zero_position, "indent should be {}, got {}", zero_position, m.x_offset());

    // Move: offset 12, scroll left by 6 -> 6. (Upstream sets the private
    // longestLineWidth so the offset is not clamped; here a long content
    // plays the same role.)
    let mut m = new_vt(10, 10);
    m.set_content("Some line that is longer than width");
    m.set_x_offset(6 * 2);
    m.scroll_left(6);
    assert_eq!(m.x_offset(), 6, "indent should be 6, got {}", m.x_offset());
}

#[test]
fn test_move_right() {
    let zero_position = 0;

    let mut m = new_vt(10, 10);
    m.set_content("Some line that is longer than width");
    assert_eq!(m.x_offset(), zero_position, "default indent should be {}, got {}", zero_position, m.x_offset());

    m.scroll_right(6);
    assert_eq!(m.x_offset(), 6, "indent should be 6, got {}", m.x_offset());
}

#[test]
fn test_reset_indent() {
    let zero_position = 0;

    let mut m = new_vt(10, 10);
    m.set_x_offset(500);
    m.set_x_offset(zero_position);
    assert_eq!(m.x_offset(), zero_position, "indent should be {}, got {}", zero_position, m.x_offset());
}

#[test]
fn test_visible_lines() {
    let default_list: Vec<String> = TEXT_CONTENT_LIST.split('\n').map(|s| s.to_string()).collect();

    // Empty list: the view renders blank lines.
    let m = new_vt(10, 10);
    let view = m.view();
    for line in view.split('\n') {
        assert!(line.trim().is_empty(), "view should be empty, got {}", m.view());
    }

    // List of 10 lines, trimmed to width.
    let number_of_lines = 10;
    let mut m = new_vt(10, number_of_lines);
    m.set_content(&default_list.join("\n"));
    let view = m.view();
    let lines: Vec<&str> = view.split('\n').collect();
    assert_eq!(lines.len(), number_of_lines, "view should have {number_of_lines} lines, got {}", lines.len());
    let last_item_idx = number_of_lines - 1;
    let should_get: String = default_list[last_item_idx].chars().take(m.width()).collect();
    assert_eq!(lines[last_item_idx], should_get, "{last_item_idx}th list item should be '{should_get}', got '{}'", lines[last_item_idx]);

    // List with y offset.
    let mut m = new_vt(10, number_of_lines);
    m.set_content(&default_list.join("\n"));
    m.set_y_offset(5);
    let view = m.view();
    let lines: Vec<&str> = view.split('\n').collect();
    assert_eq!(lines.len(), number_of_lines, "view should have {number_of_lines} lines, got {}", lines.len());
    assert_ne!(lines[0], default_list[0], "first item of list should not be the first item of initial list because of Y offset");
    let last_item_idx = number_of_lines - 1;
    let should_get: String = default_list[m.y_offset() + last_item_idx].chars().take(m.width()).collect();
    assert_eq!(lines[last_item_idx], should_get, "{last_item_idx}th list item should be '{should_get}', got '{}'", lines[last_item_idx]);

    // List with y offset and horizontal scroll. (Upstream's white-box test
    // sets the private `lines` field directly, leaving `longestLineWidth`
    // zero so lines are not cut; here `set_content_lines` records the
    // longest line, so the lines are always cut to the viewport width —
    // the same truncation the `ScrollRight` path applies.)
    let mut m = new_vt(10, number_of_lines);
    m.set_content_lines(&default_list);
    m.set_y_offset(7);
    let view = m.view();
    let lines: Vec<&str> = view.split('\n').collect();
    assert_eq!(lines.len(), number_of_lines, "view should have {number_of_lines} lines, got {}", lines.len());
    let last_item = number_of_lines - 1;
    let default_last_item = default_list.len() - 1;
    let cut: String = default_list[default_last_item].chars().take(m.width()).collect();
    assert_eq!(lines[last_item], cut, "{last_item}th list item should be the width-cut version");
    assert!(lines[0].starts_with("Precept"), "first list item has to have prefix Precept");

    m.scroll_right(6);
    let view = m.view();
    let lines: Vec<&str> = view.split('\n').collect();
    let prefix: String = "Precept".chars().skip(m.x_offset()).collect();
    assert!(lines[0].starts_with(&prefix), "first list item has to have prefix {prefix}, get {}", lines[0]);
    let cut: String = util::cut(&default_list[default_last_item], m.x_offset(), m.x_offset() + m.width());
    assert_eq!(lines[last_item], cut, "last item should be offset-cut, got {}", lines[last_item]);

    m.scroll_left(6);
    let view = m.view();
    let lines: Vec<&str> = view.split('\n').collect();
    assert!(lines[0].starts_with("Precept"), "first list item has to have prefix Precept");
    let cut: String = default_list[default_last_item].chars().take(m.width()).collect();
    assert_eq!(lines[last_item], cut, "{last_item}th list item should be the width-cut version");

    // List with 2-cell symbols and horizontal scroll. (Upstream sets the
    // private longestLineWidth=30 hack; here the lines are long enough that
    // horizontal scrolling is possible: 15 graphemes x 2 cells = 30 cells.)
    let horizontal_step = 5;
    let init_list: Vec<String> = vec![
        "あいうえおかきくけこさしすせそ".to_string(),
        "Aあいうえおかきくけこさしすせそ".to_string(),
        "あいうえおかきくけこさしすせそ".to_string(),
        "Aあいうえおかきくけこさしすせそ".to_string(),
    ];
    let number_of_lines = init_list.len();
    let mut m = new_vt(20, number_of_lines);
    m.set_content_lines(&init_list);
    let view = m.view();
    let lines: Vec<&str> = view.split('\n').collect();
    assert_eq!(lines.len(), number_of_lines, "view should have {number_of_lines} lines, got {}", lines.len());
    let last_item_idx = number_of_lines - 1;
    let cut: String = charming_x_ansi::cut(&init_list[last_item_idx], 0, 20);
    assert_eq!(lines[last_item_idx].trim_end(), cut, "{last_item_idx}th list item should be the width-cut version");

    m.scroll_right(horizontal_step);
    let view = m.view();
    let lines: Vec<&str> = view.split('\n').collect();
    for (i, line) in lines.iter().enumerate() {
        let cut: String = charming_x_ansi::cut(&init_list[i], 5, 25);
        assert_eq!(line.trim_end(), cut, "line must be `{cut}`, get `{line}`");
    }

    m.scroll_left(horizontal_step);
    let view = m.view();
    let lines: Vec<&str> = view.split('\n').collect();
    for (i, line) in lines.iter().enumerate() {
        let cut: String = charming_x_ansi::cut(&init_list[i], 0, 20);
        assert_eq!(line.trim_end(), cut, "line must be `{cut}`, get `{line}`");
    }

    // Move left a second time does not change lines if indent == 0.
    m.set_x_offset(0);
    m.scroll_left(horizontal_step);
    let view = m.view();
    let lines: Vec<&str> = view.split('\n').collect();
    for (i, line) in lines.iter().enumerate() {
        let cut: String = charming_x_ansi::cut(&init_list[i], 0, 20);
        assert_eq!(line.trim_end(), cut, "line must be `{cut}`, get `{line}`");
    }
}

#[test]
fn test_right_overscroll() {
    let content = "Content is short";
    let mut m = new_vt(content.len() + 1, 5);
    m.set_content(content);
    for _ in 0..10 {
        m.scroll_right(6);
    }
    let view = m.view();
    let lines: Vec<&str> = view.split('\n').collect();
    assert_eq!(lines[0].trim_end(), content, "visible line should stay the same as content");
}

fn test_highlights(content: &str, re: &regex::Regex, expect: Vec<HighlightInfo>) {
    let mut vt = new_vt(100, 100);
    vt.set_content(content);

    let content = vt.get_content();
    let matches: Vec<Vec<usize>> = re.find_iter(&content).map(|m| vec![m.start(), m.end()]).collect();
    vt.set_highlights(&matches);

    assert_eq!(
        vt.highlights(),
        expect.as_slice(),
        "\nexpect: {expect:?}\n   got: {:?}",
        vt.highlights()
    );

    if re.as_str().contains('\n') {
        return; // cannot check text when regex has span lines
    }

    for hi in &expect {
        for (line, hl) in &hi.lines {
            let cut = util::cut(&vt.get_content().split('\n').nth(*line).unwrap_or("").to_string(), hl.0, hl.1);
            if !re.is_match(&cut) {
                panic!("expect to match '{}', got '{cut}': line: {line}, cut: {hl:?}", re.as_str());
            }
        }
    }
}

#[test]
fn test_matches_to_highlights() {
    let content = "hello
world

with empty rows

wide chars: あいうえおafter

爱开源 • Charm does open source

Charm热爱开源 • Charm loves open source
";

    let cases: Vec<(&str, Vec<HighlightInfo>)> = vec![
        (
            "hello",
            vec![HighlightInfo {
                line_start: 0,
                line_end: 0,
                lines: HashMap::from([(0, (0, 5))]),
            }],
        ),
        (
            "l",
            vec![
                HighlightInfo {
                    line_start: 0,
                    line_end: 0,
                    lines: HashMap::from([(0, (2, 3))]),
                },
                HighlightInfo {
                    line_start: 0,
                    line_end: 0,
                    lines: HashMap::from([(0, (3, 4))]),
                },
                HighlightInfo {
                    line_start: 1,
                    line_end: 1,
                    lines: HashMap::from([(1, (3, 4))]),
                },
                HighlightInfo {
                    line_start: 9,
                    line_end: 9,
                    lines: HashMap::from([(9, (22, 23))]),
                },
            ],
        ),
        (
            "lo\nwo",
            vec![HighlightInfo {
                line_start: 0,
                line_end: 1,
                lines: HashMap::from([(0, (3, 6)), (1, (0, 2))]),
            }],
        ),
        (
            "lo\n",
            vec![HighlightInfo {
                line_start: 0,
                line_end: 0,
                lines: HashMap::from([(0, (3, 6))]),
            }],
        ),
        (
            "ith",
            vec![HighlightInfo {
                line_start: 3,
                line_end: 3,
                lines: HashMap::from([(3, (1, 4))]),
            }],
        ),
        (
            "with",
            vec![HighlightInfo {
                line_start: 3,
                line_end: 3,
                lines: HashMap::from([(3, (0, 4))]),
            }],
        ),
        (
            "after",
            vec![HighlightInfo {
                line_start: 5,
                line_end: 5,
                lines: HashMap::from([(5, (22, 27))]),
            }],
        ),
        (
            "Charm",
            vec![
                HighlightInfo {
                    line_start: 7,
                    line_end: 7,
                    lines: HashMap::from([(7, (9, 14))]),
                },
                HighlightInfo {
                    line_start: 9,
                    line_end: 9,
                    lines: HashMap::from([(9, (0, 5))]),
                },
                HighlightInfo {
                    line_start: 9,
                    line_end: 9,
                    lines: HashMap::from([(9, (16, 21))]),
                },
            ],
        ),
    ];

    for (pattern, expect) in cases {
        let re = regex::Regex::new(pattern).unwrap();
        test_highlights(content, &re, expect);
    }
}

#[test]
fn test_sizing() {
    let lines: Vec<String> = TEXT_CONTENT_LIST.split('\n').map(|s| s.to_string()).collect();

    // view-40x100percent
    let width = 40;
    let height = lines.len() + 2;
    let mut vt = new_vt(width, height);
    vt.style = vt.style.clone().border(Border::rounded(), &[true, true, true, true]);
    vt.set_content(TEXT_CONTENT_LIST);
    let view = vt.view();
    assert_eq!(
        (charming_lipgloss::size::width(&view), charming_lipgloss::size::height(&view)),
        (width, height),
        "view size should be {width} x {height}"
    );
    common::assert_golden(&view, "TestSizing/view-40x100percent.golden");

    // view-50x15-softwrap
    let (width, height) = (50, 15);
    let mut vt = new_vt(width, height);
    vt.soft_wrap = true;
    vt.style = vt.style.clone().border(Border::rounded(), &[true, true, true, true]);
    vt.set_content(TEXT_CONTENT_LIST);
    let view = vt.view();
    assert_eq!(
        (charming_lipgloss::size::width(&view), charming_lipgloss::size::height(&view)),
        (width, height),
        "view size should be {width} x {height}"
    );
    common::assert_golden(&vt.view(), "TestSizing/view-50x15-softwrap-at-top.golden");
    vt.scroll_down(1);
    common::assert_golden(&vt.view(), "TestSizing/view-50x15-softwrap-scrolled-plus-1.golden");
    vt.scroll_down(1);
    common::assert_golden(&vt.view(), "TestSizing/view-50x15-softwrap-scrolled-plus-2.golden");
    vt.goto_bottom();
    common::assert_golden(&vt.view(), "TestSizing/view-50x15-softwrap-at-bottom.golden");

    // view-50x15-softwrap-gutter
    let (width, height) = (50, 15);
    let mut vt = new_vt(width, height);
    vt.soft_wrap = true;
    vt.style = vt.style.clone().border(Border::rounded(), &[true, true, true, true]);
    vt.left_gutter_func = Some(Box::new(|_ctx: GutterContext| -> String { "  ".to_string() }));
    vt.set_content(TEXT_CONTENT_LIST);
    assert_eq!(
        (charming_lipgloss::size::width(&vt.view()), charming_lipgloss::size::height(&vt.view())),
        (width, height),
        "view size should be {width} x {height}"
    );
    common::assert_golden(&vt.view(), "TestSizing/view-50x15-softwrap-gutter-at-top.golden");
    vt.scroll_down(1);
    assert_eq!(
        (charming_lipgloss::size::width(&vt.view()), charming_lipgloss::size::height(&vt.view())),
        (width, height),
        "view size should be {width} x {height}"
    );
    common::assert_golden(&vt.view(), "TestSizing/view-50x15-softwrap-gutter-scrolled-plus-1.golden");
    vt.scroll_down(1);
    common::assert_golden(&vt.view(), "TestSizing/view-50x15-softwrap-gutter-scrolled-plus-2.golden");
    vt.goto_bottom();
    common::assert_golden(&vt.view(), "TestSizing/view-50x15-softwrap-gutter-at-bottom.golden");

    // view-40x1-softwrap
    let (width, height) = (40 + 2, 1 + 2);
    let mut vt = new_vt(width, height);
    vt.soft_wrap = true;
    vt.style = vt.style.clone().border(Border::rounded(), &[true, true, true, true]);
    vt.set_content(TEXT_CONTENT_LIST);
    let view = vt.view();
    assert_eq!(
        (charming_lipgloss::size::width(&view), charming_lipgloss::size::height(&view)),
        (width, height),
        "view size should be {width} x {height}"
    );
    common::assert_golden(&view, "TestSizing/view-40x1-softwrap.golden");
    vt.scroll_down(1);
    common::assert_golden(&vt.view(), "TestSizing/view-40x1-softwrap-scrolled-plus-1.golden");
    vt.scroll_down(1);
    common::assert_golden(&vt.view(), "TestSizing/view-40x1-softwrap-scrolled-plus-2.golden");
    vt.goto_bottom();
    common::assert_golden(&vt.view(), "TestSizing/view-40x1-softwrap-at-bottom.golden");

    // view-50x15-content-lines
    let content: Vec<String> = vec!["57 Precepts of narcissistic comedy character Zote from an\nawesome \"Hollow knight\" game".to_string()];
    let mut vt = new_vt(50, 15);
    vt.set_content_lines(&content);
    common::assert_golden(&vt.view(), "TestSizing/view-50x15-content-lines.golden");

    // 0x0, 1x0, 0x1 — ensure no panics.
    let mut vt = new_vt(0, 0);
    vt.set_content(TEXT_CONTENT_LIST);
    let _ = vt.view();
    let mut vt = new_vt(1, 0);
    vt.set_content(TEXT_CONTENT_LIST);
    let _ = vt.view();
    let mut vt = new_vt(0, 1);
    vt.set_content(TEXT_CONTENT_LIST);
    let _ = vt.view();
}

/// Port of the upstream `BenchmarkView` (no Go-style benches on stable
/// Rust): a performance smoke test that renders the viewport repeatedly.
/// Ignored by default so CI stays fast; run with `--ignored` to sanity
/// check rendering throughput.
#[test]
#[ignore]
fn benchmark_view() {
    let cases: Vec<(usize, usize, bool)> = vec![(30, 15, false), (100, 100, false), (30, 15, true), (100, 100, true)];
    for (w, h, soft_wrap) in cases {
        let mut vt = new_vt(w, h);
        vt.soft_wrap = soft_wrap;
        vt.set_content(TEXT_CONTENT_LIST);
        let mut acc = 0usize;
        for _ in 0..2000 {
            acc += vt.view().len();
        }
        assert!(acc > 0);
    }
}
