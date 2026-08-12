//! Cleanroom Rust port of upstream Go source file: `filepicker/filepicker.go`
//! Cleanroom Rust port of upstream Go source file: `filepicker/hidden_unix.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! <public-docs>
//! # FilePicker
//!
//! A file picker component for Bubble Tea applications.
//!
//! The humanized byte formatting is an inline port of
//! `github.com/dustin/go-humanize`'s `Bytes` function, and the permission
//! string is a port of Go's `os.FileMode::String`.
//! </public-docs>

use crate::key::{self, Binding};
use charming_bubbletea::key::KeyPressMsg;
use charming_bubbletea::model::{Cmd, Msg};
use charming_lipgloss::{self, Style, RIGHT};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};

static LAST_ID: AtomicI64 = AtomicI64::new(0);

fn next_id() -> i32 {
    (LAST_ID.fetch_add(1, Ordering::SeqCst)) as i32
}

/// New returns a new filepicker model with default styling and key bindings.
pub fn new() -> Model {
    Model {
        id: next_id(),
        current_directory: ".".to_string(),
        cursor: ">".to_string(),
        allowed_types: vec![],
        selected: 0,
        show_permissions: true,
        show_size: true,
        show_hidden: false,
        dir_allowed: false,
        file_allowed: true,
        auto_height: true,
        height: 0,
        max_idx: 0,
        min_idx: 0,
        selected_stack: Stack::new(),
        min_stack: Stack::new(),
        max_stack: Stack::new(),
        key_map: default_key_map(),
        styles: default_styles(),
        path: String::new(),
        files: vec![],
        file_selected: String::new(),
    }
}

/// errorMsg is sent when reading a directory fails.
#[derive(Debug)]
pub struct ErrorMsg(pub String);

/// readDirMsg is sent after a directory has been read.
#[derive(Debug)]
pub struct ReadDirMsg {
    id: i32,
    /// The entries of the directory.
    pub entries: Vec<DirEntry>,
}

/// DirEntry mirrors a single entry in a directory listing.
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// The name of the entry.
    pub name: String,
    /// Whether the entry is a directory.
    pub is_dir: bool,
    /// The size of the entry in bytes.
    pub size: u64,
    /// The Go-style permission string, e.g. "-rw-r--r--".
    pub mode_string: String,
    /// Whether the entry is a symlink.
    pub is_symlink: bool,
}

const MARGIN_BOTTOM: usize = 5;
const FILE_SIZE_WIDTH: usize = 7;
const PADDING_LEFT: usize = 2;

/// KeyMap defines key bindings for each user action.
#[derive(Debug, Clone)]
pub struct KeyMap {
    /// GoToTop binding.
    pub go_to_top: Binding,
    /// GoToLast binding.
    pub go_to_last: Binding,
    /// Down binding.
    pub down: Binding,
    /// Up binding.
    pub up: Binding,
    /// PageUp binding.
    pub page_up: Binding,
    /// PageDown binding.
    pub page_down: Binding,
    /// Back binding.
    pub back: Binding,
    /// Open binding.
    pub open: Binding,
    /// Select binding.
    pub select: Binding,
}

/// DefaultKeyMap defines the default keybindings.
pub fn default_key_map() -> KeyMap {
    KeyMap {
        go_to_top: key::new_binding(vec![key::with_keys(&["g"]), key::with_help("g", "first")]),
        go_to_last: key::new_binding(vec![key::with_keys(&["G"]), key::with_help("G", "last")]),
        down: key::new_binding(vec![
            key::with_keys(&["j", "down", "ctrl+n"]),
            key::with_help("j", "down"),
        ]),
        up: key::new_binding(vec![
            key::with_keys(&["k", "up", "ctrl+p"]),
            key::with_help("k", "up"),
        ]),
        page_up: key::new_binding(vec![
            key::with_keys(&["K", "pgup"]),
            key::with_help("pgup", "page up"),
        ]),
        page_down: key::new_binding(vec![
            key::with_keys(&["J", "pgdown"]),
            key::with_help("pgdown", "page down"),
        ]),
        back: key::new_binding(vec![
            key::with_keys(&["h", "backspace", "left", "esc"]),
            key::with_help("h", "back"),
        ]),
        open: key::new_binding(vec![
            key::with_keys(&["l", "right", "enter"]),
            key::with_help("l", "open"),
        ]),
        select: key::new_binding(vec![
            key::with_keys(&["enter"]),
            key::with_help("enter", "select"),
        ]),
    }
}

/// Styles defines the possible customizations for styles in the file picker.
#[derive(Debug, Clone)]
pub struct Styles {
    /// Style for the disabled cursor.
    pub disabled_cursor: Style,
    /// Style for the cursor.
    pub cursor: Style,
    /// Style for symlinks.
    pub symlink: Style,
    /// Style for directories.
    pub directory: Style,
    /// Style for files.
    pub file: Style,
    /// Style for disabled files.
    pub disabled_file: Style,
    /// Style for permissions.
    pub permission: Style,
    /// Style for the selected item.
    pub selected: Style,
    /// Style for disabled selected items.
    pub disabled_selected: Style,
    /// Style for file sizes.
    pub file_size: Style,
    /// Style for the empty directory view.
    pub empty_directory: Style,
}

/// DefaultStyles defines the default styling for the file picker.
pub fn default_styles() -> Styles {
    Styles {
        disabled_cursor: charming_lipgloss::new_style().foreground("247"),
        cursor: charming_lipgloss::new_style().foreground("212"),
        symlink: charming_lipgloss::new_style().foreground("36"),
        directory: charming_lipgloss::new_style().foreground("99"),
        file: charming_lipgloss::new_style(),
        disabled_file: charming_lipgloss::new_style().foreground("243"),
        disabled_selected: charming_lipgloss::new_style().foreground("247"),
        permission: charming_lipgloss::new_style().foreground("244"),
        selected: charming_lipgloss::new_style().foreground("212").bold(true),
        file_size: charming_lipgloss::new_style()
            .foreground("240")
            .width(FILE_SIZE_WIDTH)
            .align(&[RIGHT]),
        empty_directory: charming_lipgloss::new_style()
            .foreground("240")
            .padding_left(PADDING_LEFT)
            .set_string(&["Bummer. No Files Found."]),
    }
}

/// Model represents a file picker.
#[derive(Debug)]
pub struct Model {
    id: i32,

    /// Path is the path which the user has selected with the file picker.
    pub path: String,

    /// CurrentDirectory is the directory that the user is currently in.
    pub current_directory: String,

    /// AllowedTypes specifies which file types the user may select.
    /// If empty the user may select any file.
    pub allowed_types: Vec<String>,

    /// The key bindings for the file picker.
    pub key_map: KeyMap,
    files: Vec<DirEntry>,
    /// Whether to show permissions.
    pub show_permissions: bool,
    /// Whether to show file sizes.
    pub show_size: bool,
    /// Whether to show hidden files.
    pub show_hidden: bool,
    /// Whether directories can be selected.
    pub dir_allowed: bool,
    /// Whether files can be selected.
    pub file_allowed: bool,

    /// The currently selected file.
    pub file_selected: String,
    selected: usize,
    selected_stack: Stack,

    min_idx: usize,
    max_idx: usize,
    max_stack: Stack,
    min_stack: Stack,

    height: usize,
    /// Whether the height is automatically managed.
    pub auto_height: bool,

    /// The cursor string.
    pub cursor: String,
    /// The styles of the file picker.
    pub styles: Styles,
}

/// Stack is a simple LIFO stack of indices used to remember navigation
/// history.
#[derive(Debug, Default)]
pub struct Stack {
    slice: Vec<usize>,
}

impl Stack {
    fn new() -> Stack {
        Stack { slice: vec![] }
    }

    fn push(&mut self, i: usize) {
        self.slice.push(i);
    }

    fn pop(&mut self) -> usize {
        let res = self.slice[self.slice.len() - 1];
        self.slice.pop();
        res
    }

    fn length(&self) -> usize {
        self.slice.len()
    }
}

impl Model {
    fn push_view(&mut self, selected: usize, minimum: usize, maximum: usize) {
        self.selected_stack.push(selected);
        self.min_stack.push(minimum);
        self.max_stack.push(maximum);
    }

    fn pop_view(&mut self) -> (usize, usize, usize) {
        (
            self.selected_stack.pop(),
            self.min_stack.pop(),
            self.max_stack.pop(),
        )
    }

    fn read_dir_cmd(&self, path: &str, show_hidden: bool) -> Cmd {
        let path = path.to_string();
        let id = self.id;
        Some(Box::new(move || {
            let mut entries: Vec<DirEntry> = vec![];
            if let Ok(rd) = std::fs::read_dir(&path) {
                for entry in rd.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let metadata = entry.metadata();
                    let (is_dir, size, mode_string, is_symlink) = match metadata {
                        Ok(md) => {
                            let is_symlink = md.file_type().is_symlink();
                            (md.is_dir(), md.len(), file_mode_string(&md), is_symlink)
                        }
                        Err(_) => (false, 0, "----------".to_string(), false),
                    };
                    entries.push(DirEntry {
                        name,
                        is_dir,
                        size,
                        mode_string,
                        is_symlink,
                    });
                }
            }
            entries.sort_by(|a, b| {
                if a.is_dir == b.is_dir {
                    a.name.cmp(&b.name)
                } else {
                    b.is_dir.cmp(&a.is_dir)
                }
            });

            if show_hidden {
                return Some(Box::new(ReadDirMsg { id, entries }));
            }

            let mut sanitized: Vec<DirEntry> = vec![];
            for dir_entry in entries {
                let is_hidden = is_hidden(&dir_entry.name);
                if is_hidden {
                    continue;
                }
                sanitized.push(dir_entry);
            }
            Some(Box::new(ReadDirMsg {
                id,
                entries: sanitized,
            }))
        }))
    }

    /// SetHeight sets the height of the file picker.
    pub fn set_height(&mut self, h: usize) {
        self.height = h;
        if self.max_idx > self.height.saturating_sub(1) {
            self.max_idx = self.min_idx + self.height - 1;
        }
    }

    /// Height returns the height of the file picker.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Init initializes the file picker model.
    pub fn init(&self) -> Cmd {
        self.read_dir_cmd(&self.current_directory.clone(), self.show_hidden)
    }

    /// Update handles user interactions within the file picker model.
    pub fn update(&mut self, msg: &dyn Msg) -> Cmd {
        if let Some(m) = msg.as_any().downcast_ref::<ReadDirMsg>() {
            if m.id != self.id {
                return None;
            }
            self.files = m.entries.clone();
            self.max_idx = self.max_idx.max(self.height().saturating_sub(1));
            return None;
        }

        if msg.as_any().downcast_ref::<ErrorMsg>().is_some() {
            return None;
        }

        if let Some(m) = msg
            .as_any()
            .downcast_ref::<charming_bubbletea::screen::WindowSizeMsg>()
        {
            if self.auto_height {
                self.set_height(m.height - MARGIN_BOTTOM);
            }
            self.max_idx = self.height() - 1;
            return None;
        }

        if let Some(m) = msg.as_any().downcast_ref::<KeyPressMsg>() {
            let k = &m.0;
            if key::matches(k, std::slice::from_ref(&self.key_map.go_to_top)) {
                self.selected = 0;
                self.min_idx = 0;
                self.max_idx = self.height() - 1;
            } else if key::matches(k, std::slice::from_ref(&self.key_map.go_to_last)) {
                self.selected = self.files.len().saturating_sub(1);
                self.min_idx = self.files.len() - self.height();
                self.max_idx = self.files.len() - 1;
            } else if key::matches(k, std::slice::from_ref(&self.key_map.down)) {
                self.selected += 1;
                if self.selected >= self.files.len() {
                    self.selected = self.files.len().saturating_sub(1);
                }
                if self.selected > self.max_idx {
                    self.min_idx += 1;
                    self.max_idx += 1;
                }
            } else if key::matches(k, std::slice::from_ref(&self.key_map.up)) {
                self.selected = self.selected.saturating_sub(1);
                if self.selected < self.min_idx {
                    self.min_idx = self.min_idx.saturating_sub(1);
                    self.max_idx = self.max_idx.saturating_sub(1);
                }
            } else if key::matches(k, std::slice::from_ref(&self.key_map.page_down)) {
                self.selected += self.height();
                if self.selected >= self.files.len() {
                    self.selected = self.files.len().saturating_sub(1);
                }
                self.min_idx += self.height();
                self.max_idx += self.height();

                if self.max_idx >= self.files.len() {
                    self.max_idx = self.files.len() - 1;
                    self.min_idx = self.max_idx - self.height();
                }
            } else if key::matches(k, std::slice::from_ref(&self.key_map.page_up)) {
                self.selected = self.selected.saturating_sub(self.height());
                self.min_idx = self.min_idx.saturating_sub(self.height());
                self.max_idx = self.max_idx.saturating_sub(self.height());

                if self.min_idx == 0 {
                    // minIdx < 0 => 0; maxIdx = minIdx + Height
                    self.max_idx = self.min_idx + self.height();
                }
            } else if key::matches(k, std::slice::from_ref(&self.key_map.back)) {
                self.current_directory = Path::new(&self.current_directory)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "/".to_string());
                if self.selected_stack.length() > 0 {
                    let (s, mn, mx) = self.pop_view();
                    self.selected = s;
                    self.min_idx = mn;
                    self.max_idx = mx;
                } else {
                    self.selected = 0;
                    self.min_idx = 0;
                    self.max_idx = self.height() - 1;
                }
                return self.read_dir_cmd(&self.current_directory.clone(), self.show_hidden);
            } else if key::matches(k, std::slice::from_ref(&self.key_map.open)) {
                if self.files.is_empty() {
                    return None;
                }

                let f = self.files[self.selected].clone();
                let is_symlink = f.is_symlink;
                let mut is_dir = f.is_dir;

                if is_symlink {
                    let symlink_path = PathBuf::from(&self.current_directory)
                        .join(&f.name)
                        .canonicalize()
                        .unwrap_or_default();
                    if symlink_path.is_dir() {
                        is_dir = true;
                    }
                }

                if ((!is_dir && self.file_allowed) || (is_dir && self.dir_allowed))
                    && key::matches(k, std::slice::from_ref(&self.key_map.select))
                {
                    // Select the current path as the selection
                    self.path = PathBuf::from(&self.current_directory)
                        .join(&f.name)
                        .to_string_lossy()
                        .to_string();
                }

                if !is_dir {
                    return None;
                }

                self.current_directory = PathBuf::from(&self.current_directory)
                    .join(&f.name)
                    .to_string_lossy()
                    .to_string();
                self.push_view(self.selected, self.min_idx, self.max_idx);
                self.selected = 0;
                self.min_idx = 0;
                self.max_idx = self.height() - 1;
                return self.read_dir_cmd(&self.current_directory.clone(), self.show_hidden);
            }
        }
        None
    }

    /// View returns the view of the file picker.
    pub fn view(&self) -> String {
        if self.files.is_empty() {
            let v = self
                .styles
                .empty_directory
                .clone()
                .set_string(&["Bummer. No Files Found."])
                .height(self.height())
                .max_height(self.height())
                .render("");
            return v;
        }
        let mut s = String::new();

        for (i, f) in self.files.iter().enumerate() {
            if i < self.min_idx || i > self.max_idx {
                continue;
            }

            let mut symlink_path = String::new();
            let is_symlink = f.is_symlink;
            let size = humanize_bytes(f.size);
            let name = &f.name;

            if is_symlink {
                symlink_path = PathBuf::from(&self.current_directory)
                    .join(name)
                    .canonicalize()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
            }

            let disabled = !self.can_select(name) && !f.is_dir;

            if self.selected == i {
                let mut selected = String::new();
                if self.show_permissions {
                    selected += " ";
                    selected += &f.mode_string;
                }
                if self.show_size {
                    selected += &format!(
                        "{:>width$}",
                        size,
                        width = self.styles.file_size.get_width()
                    );
                }
                selected += " ";
                selected += name;
                if is_symlink {
                    selected += " → ";
                    selected += &symlink_path;
                }
                if disabled {
                    s += &self.styles.disabled_cursor.clone().render(&self.cursor);
                    s += &self.styles.disabled_selected.clone().render(&selected);
                } else {
                    s += &self.styles.cursor.clone().render(&self.cursor);
                    s += &self.styles.selected.clone().render(&selected);
                }
                s += "\n";
                continue;
            }

            let mut style = self.styles.file.clone();
            if f.is_dir {
                style = self.styles.directory.clone();
            } else if is_symlink {
                style = self.styles.symlink.clone();
            } else if disabled {
                style = self.styles.disabled_file.clone();
            }

            let mut file_name = style.render(name);
            s += &self.styles.cursor.clone().render(" ");
            if is_symlink {
                file_name += " → ";
                file_name += &symlink_path;
            }
            if self.show_permissions {
                s += " ";
                s += &self.styles.permission.clone().render(&f.mode_string);
            }
            if self.show_size {
                s += &self.styles.file_size.clone().render(&size);
            }
            s += " ";
            s += &file_name;
            s += "\n";
        }

        for _ in charming_lipgloss::size::height(&s)..=self.height() {
            s += "\n";
        }

        s
    }

    /// DidSelectFile returns whether a user has selected a file (on this
    /// msg).
    pub fn did_select_file(&self, msg: &dyn Msg) -> (bool, String) {
        let (did_select, path) = self.did_select_file_inner(msg);
        if did_select && self.can_select(&path) {
            return (true, path);
        }
        (false, String::new())
    }

    /// DidSelectDisabledFile returns whether a user tried to select a
    /// disabled file (on this msg).
    pub fn did_select_disabled_file(&self, msg: &dyn Msg) -> (bool, String) {
        let (did_select, path) = self.did_select_file_inner(msg);
        if did_select && !self.can_select(&path) {
            return (true, path);
        }
        (false, String::new())
    }

    fn did_select_file_inner(&self, msg: &dyn Msg) -> (bool, String) {
        if self.files.is_empty() {
            return (false, String::new());
        }
        let m = msg.as_any().downcast_ref::<KeyPressMsg>();
        match m {
            Some(m) => {
                // If the msg does not match the Select keymap then this
                // could not have been a selection.
                if !key::matches(&m.0, std::slice::from_ref(&self.key_map.select)) {
                    return (false, String::new());
                }

                // The key press was a selection, let's confirm whether the
                // current file could be selected or used for navigating
                // deeper into the stack.
                let f = &self.files[self.selected];
                let is_symlink = f.is_symlink;
                let mut is_dir = f.is_dir;

                if is_symlink {
                    let symlink_path = PathBuf::from(&self.current_directory)
                        .join(&f.name)
                        .canonicalize()
                        .unwrap_or_default();
                    if symlink_path.is_dir() {
                        is_dir = true;
                    }
                }

                if ((!is_dir && self.file_allowed) || (is_dir && self.dir_allowed))
                    && !self.path.is_empty()
                {
                    return (true, self.path.clone());
                }

                (false, String::new())
            }
            None => (false, String::new()),
        }
    }

    fn can_select(&self, file: &str) -> bool {
        if self.allowed_types.is_empty() {
            return true;
        }

        for ext in &self.allowed_types {
            if file.ends_with(ext.as_str()) {
                return true;
            }
        }
        false
    }

    /// HighlightedPath returns the path of the currently highlighted file
    /// or directory.
    pub fn highlighted_path(&self) -> String {
        if self.files.is_empty() || self.selected >= self.files.len() {
            return String::new();
        }
        PathBuf::from(&self.current_directory)
            .join(&self.files[self.selected].name)
            .to_string_lossy()
            .to_string()
    }
}

/// isHidden returns whether the given name is hidden: a leading dot on Unix
/// (port of `hidden_unix.go`), or the `FILE_ATTRIBUTE_HIDDEN` attribute on
/// Windows (port of `hidden_windows.go`). The name is resolved relative to
/// the current working directory on Windows, mirroring the upstream call
/// `IsHidden(dirEntry.Name())`.
#[cfg(not(windows))]
fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

/// Windows port of `hidden_windows.go`: `GetFileAttributes` + the
/// `FILE_ATTRIBUTE_HIDDEN` flag.
#[cfg(windows)]
fn is_hidden(name: &str) -> bool {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetFileAttributesW(lp_file_name: *const u16) -> u32;
    }

    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    const INVALID_FILE_ATTRIBUTES: u32 = u32::MAX;

    let wide: Vec<u16> = std::ffi::OsStr::new(name)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let attributes = GetFileAttributesW(wide.as_ptr());
        attributes != INVALID_FILE_ATTRIBUTES && (attributes & FILE_ATTRIBUTE_HIDDEN) != 0
    }
}

/// humanizeBytes formats a byte count the way Go's go-humanize `Bytes` does
/// (with the space removed, matching the filepicker's `Replace(..., " ", "")`).
fn humanize_bytes(size: u64) -> String {
    if size < 10 {
        return format!("{} B", size);
    }
    let base = 1000.0f64;
    let sizes = ["B", "kB", "MB", "GB", "TB", "PB", "EB"];
    let n = (size as f64).log(base).floor() as usize;
    let suffix = sizes[n.min(sizes.len() - 1)];
    let val = (size as f64 / base.powi(n as i32) * 10.0 + 0.5).floor() / 10.0;
    if val < 10.0 {
        return format!("{:.1} {}", val, suffix).replace(' ', "");
    }
    format!("{:.0} {}", val, suffix).replace(' ', "")
}

/// fileModeString renders Go's `os.FileMode::String()` permission string,
/// e.g. "drwxr-xr-x" or "-rw-r--r--".
#[cfg(unix)]
fn file_mode_string(md: &std::fs::Metadata) -> String {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};

    let mut s = String::with_capacity(10);
    let mode = md.mode();

    // File type bit.
    let kind = md.file_type();
    if kind.is_dir() {
        s.push('d');
    } else if kind.is_symlink() {
        s.push('L');
    } else if kind.is_char_device() {
        s.push('c');
    } else if kind.is_block_device() {
        s.push('b');
    } else if kind.is_fifo() {
        s.push('p');
    } else if kind.is_socket() {
        s.push('S');
    } else {
        s.push('-');
    }

    // Permission bits.
    let perm = mode & 0o7777;
    let chars = ['r', 'w', 'x'];
    for i in 0..9 {
        if perm & (0o400 >> i) != 0 {
            s.push(chars[i % 3]);
        } else {
            s.push('-');
        }
        if i == 2 {
            if perm & 0o4000 != 0 {
                let c = s.pop().unwrap();
                s.push(if c == 'x' { 's' } else { 'S' });
            }
        } else if i == 5 {
            if perm & 0o2000 != 0 {
                let c = s.pop().unwrap();
                s.push(if c == 'x' { 's' } else { 'S' });
            }
        } else if i == 8 && perm & 0o1000 != 0 {
            let c = s.pop().unwrap();
            s.push(if c == 'x' { 't' } else { 'T' });
        }
    }
    s
}

#[cfg(not(unix))]
fn file_mode_string(_md: &std::fs::Metadata) -> String {
    "-rw-r--r--".to_string()
}
