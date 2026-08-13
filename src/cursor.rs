//! Cleanroom Rust port of upstream Go source file: `cursor/cursor.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! <public-docs>
//! # Cursor
//!
//! A virtual cursor to support the textinput and textarea elements.
//! </public-docs>

use rusty_bubbletea::model::{Cmd, Msg};
use rusty_lipgloss::Style;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_BLINK_SPEED: Duration = Duration::from_millis(530);

/// Internal ID management. Used during animating to ensure that frame
/// messages are received only by spinner components that sent them.
static LAST_ID: AtomicI64 = AtomicI64::new(0);

fn next_id() -> i32 {
    (LAST_ID.fetch_add(1, Ordering::SeqCst)) as i32
}

/// initialBlinkMsg initializes cursor blinking.
#[derive(Debug)]
struct InitialBlinkMsg;

/// BlinkMsg signals that the cursor should blink. It contains metadata that
/// allows us to tell if the blink message is the one we're expecting.
#[derive(Debug, Clone)]
pub struct BlinkMsg {
    id: i32,
    tag: i32,
}

/// blinkCanceled is sent when a blink operation is canceled.
#[derive(Debug)]
struct BlinkCanceled;

/// Mode describes the behavior of the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The cursor blinks.
    Blink,
    /// The cursor is static.
    Static,
    /// The cursor is hidden.
    Hide,
}

impl Mode {
    /// Returns the cursor mode in a human-readable format. This method is
    /// provisional and for informational purposes only.
    pub fn to_string(&self) -> &'static str {
        match self {
            Mode::Blink => "blink",
            Mode::Static => "static",
            Mode::Hide => "hidden",
        }
    }
}

/// Model is the Bubble Tea model for this cursor element.
#[derive(Clone)]
pub struct Model {
    /// Style styles the cursor block.
    pub style: Style,

    /// TextStyle is the style used for the cursor when it is blinking
    /// (hidden), i.e. displaying normal text.
    pub text_style: Style,

    /// BlinkSpeed is the speed at which the cursor blinks. This has no effect
    /// unless [`Mode::Blink`] is set.
    pub blink_speed: Duration,

    /// IsBlinked is the state of the cursor blink. When true, the cursor is
    /// hidden.
    pub is_blinked: bool,

    /// char is the character under the cursor
    char: String,

    /// The ID of this Model as it relates to other cursors
    id: i32,

    /// focus indicates whether the containing input is focused
    focus: bool,

    /// Used to manage cursor blink cancellation.
    blink_cancel: Option<Arc<AtomicBool>>,

    /// The ID of the blink message we're expecting to receive.
    blink_tag: i32,

    /// mode determines the behavior of the cursor
    mode: Mode,
}

impl fmt::Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("cursor::Model")
            .field("mode", &self.mode)
            .field("focus", &self.focus)
            .field("is_blinked", &self.is_blinked)
            .finish()
    }
}

/// New creates a new model with default settings.
pub fn new() -> Model {
    Model {
        id: next_id(),
        blink_speed: DEFAULT_BLINK_SPEED,
        is_blinked: true,
        mode: Mode::Blink,
        style: Style::new(),
        text_style: Style::new(),
        char: String::new(),
        focus: false,
        blink_cancel: None,
        blink_tag: 0,
    }
}

impl Model {
    /// Update updates the cursor.
    pub fn update(&mut self, msg: &dyn Msg) -> Cmd {
        if msg.as_any().downcast_ref::<InitialBlinkMsg>().is_some() {
            if self.mode != Mode::Blink || !self.focus {
                return None;
            }
            return self.blink();
        }

        if msg
            .as_any()
            .downcast_ref::<rusty_bubbletea::focus::FocusMsg>()
            .is_some()
        {
            return self.focus();
        }

        if msg
            .as_any()
            .downcast_ref::<rusty_bubbletea::focus::BlurMsg>()
            .is_some()
        {
            self.blur();
            return None;
        }

        if let Some(m) = msg.as_any().downcast_ref::<BlinkMsg>() {
            // We're choosy about whether to accept blinkMsgs so that our
            // cursor only blinks exactly when it should.

            // Is this model blink-able?
            if self.mode != Mode::Blink || !self.focus {
                return None;
            }

            // Were we expecting this blink message?
            if m.id != self.id || m.tag != self.blink_tag {
                return None;
            }

            let mut cmd = None;
            if self.mode == Mode::Blink {
                self.is_blinked = !self.is_blinked;
                cmd = self.blink();
            }
            return cmd;
        }

        if msg.as_any().downcast_ref::<BlinkCanceled>().is_some() {
            // no-op
            return None;
        }

        None
    }

    /// Mode returns the model's cursor mode. For available cursor modes, see
    /// [`Mode`].
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// SetMode sets the model's cursor mode. This method returns a command.
    ///
    /// For available cursor modes, see [`Mode`].
    pub fn set_mode(&mut self, mode: Mode) -> Cmd {
        self.mode = mode;
        self.is_blinked = mode == Mode::Hide || !self.focus;
        if mode == Mode::Blink {
            return Some(Box::new(|| Some(Box::new(InitialBlinkMsg))));
        }
        None
    }

    /// Blink is a command used to manage cursor blinking.
    pub fn blink(&mut self) -> Cmd {
        if self.mode != Mode::Blink {
            return None;
        }

        if let Some(cancel) = &self.blink_cancel {
            cancel.store(true, Ordering::SeqCst);
        }

        let cancel = Arc::new(AtomicBool::new(false));
        self.blink_cancel = Some(cancel.clone());

        self.blink_tag += 1;
        let blink_msg = BlinkMsg {
            id: self.id,
            tag: self.blink_tag,
        };
        let speed = self.blink_speed;

        Some(Box::new(move || {
            std::thread::sleep(speed);
            if cancel.load(Ordering::SeqCst) {
                Some(Box::new(BlinkCanceled))
            } else {
                Some(Box::new(blink_msg))
            }
        }))
    }

    /// Focus focuses the cursor to allow it to blink if desired.
    pub fn focus(&mut self) -> Cmd {
        self.focus = true;
        // show the cursor unless we've explicitly hidden it
        self.is_blinked = self.mode == Mode::Hide;

        if self.mode == Mode::Blink && self.focus {
            return self.blink();
        }
        None
    }

    /// Blur blurs the cursor.
    pub fn blur(&mut self) {
        self.focus = false;
        self.is_blinked = true;
    }

    /// SetChar sets the character under the cursor.
    pub fn set_char(&mut self, char: &str) {
        self.char = char.to_string();
    }

    /// View displays the cursor.
    pub fn view(&self) -> String {
        if self.is_blinked {
            self.text_style.clone().inline(true).render(&self.char)
        } else {
            self.style
                .clone()
                .inline(true)
                .reverse(true)
                .render(&self.char)
        }
    }
}

/// Blink is a command used to initialize cursor blinking.
pub fn blink() -> Box<dyn Msg> {
    Box::new(InitialBlinkMsg)
}
