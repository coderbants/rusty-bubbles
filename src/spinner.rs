//! Cleanroom Rust port of upstream Go source file: `spinner/spinner.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! <public-docs>
//! # Spinner
//!
//! A spinner component for Bubble Tea applications.
//! </public-docs>

use charming_bubbletea::commands;
use charming_bubbletea::model::{Cmd, Msg};
use charming_lipgloss::Style;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, SystemTime};

/// Internal ID management. Used during animating to ensure that frame messages
/// are received only by spinner components that sent them.
static LAST_ID: AtomicI64 = AtomicI64::new(0);

fn next_id() -> i32 {
    (LAST_ID.fetch_add(1, Ordering::SeqCst)) as i32
}

/// Spinner is a set of frames used in animating the spinner.
#[derive(Debug, Clone)]
pub struct Spinner {
    /// The frames of the spinner animation.
    pub frames: Vec<String>,
    /// The frames-per-second rate at which the spinner animates.
    pub fps: Duration,
}

/// Some spinners to choose from. You could also make your own.
pub fn line() -> Spinner {
    Spinner {
    frames: vec![
        "|".to_string(),
        "/".to_string(),
        "-".to_string(),
        "\\".to_string(),
    ],
    fps: Duration::from_millis(100),
    }
}

/// A spinner of braille dots.
pub fn dot() -> Spinner {
    Spinner {
    frames: vec![
        "⣾ ".to_string(),
        "⣽ ".to_string(),
        "⣻ ".to_string(),
        "⢿ ".to_string(),
        "⡿ ".to_string(),
        "⣟ ".to_string(),
        "⣯ ".to_string(),
        "⣷ ".to_string(),
    ],
    fps: Duration::from_millis(100),
    }
}

/// A mini braille-dot spinner.
pub fn mini_dot() -> Spinner {
    Spinner {
    frames: vec![
        "⠋".to_string(),
        "⠙".to_string(),
        "⠹".to_string(),
        "⠸".to_string(),
        "⠼".to_string(),
        "⠴".to_string(),
        "⠦".to_string(),
        "⠧".to_string(),
        "⠇".to_string(),
        "⠏".to_string(),
    ],
    fps: Duration::from_millis(83),
    }
}

/// A jumping spinner.
pub fn jump() -> Spinner {
    Spinner {
    frames: vec![
        "⢄".to_string(),
        "⢂".to_string(),
        "⢁".to_string(),
        "⡁".to_string(),
        "⡈".to_string(),
        "⡐".to_string(),
        "⡠".to_string(),
    ],
    fps: Duration::from_millis(100),
    }
}

/// A pulsing block spinner.
pub fn pulse() -> Spinner {
    Spinner {
    frames: vec![
        "█".to_string(),
        "▓".to_string(),
        "▒".to_string(),
        "░".to_string(),
    ],
    fps: Duration::from_millis(125),
    }
}

/// A points spinner.
pub fn points() -> Spinner {
    Spinner {
    frames: vec![
        "∙∙∙".to_string(),
        "●∙∙".to_string(),
        "∙●∙".to_string(),
        "∙∙●".to_string(),
    ],
    fps: Duration::from_millis(142),
    }
}

/// A globe spinner.
pub fn globe() -> Spinner {
    Spinner {
    frames: vec![
        "🌍".to_string(),
        "🌎".to_string(),
        "🌏".to_string(),
    ],
    fps: Duration::from_millis(250),
    }
}

/// A moon spinner.
pub fn moon() -> Spinner {
    Spinner {
    frames: vec![
        "🌑".to_string(),
        "🌒".to_string(),
        "🌓".to_string(),
        "🌔".to_string(),
        "🌕".to_string(),
        "🌖".to_string(),
        "🌗".to_string(),
        "🌘".to_string(),
    ],
    fps: Duration::from_millis(125),
    }
}

/// A monkey spinner.
pub fn monkey() -> Spinner {
    Spinner {
    frames: vec![
        "🙈".to_string(),
        "🙉".to_string(),
        "🙊".to_string(),
    ],
    fps: Duration::from_millis(333),
    }
}

/// A meter spinner.
pub fn meter() -> Spinner {
    Spinner {
    frames: vec![
        "▱▱▱".to_string(),
        "▰▱▱".to_string(),
        "▰▰▱".to_string(),
        "▰▰▰".to_string(),
        "▰▰▱".to_string(),
        "▰▱▱".to_string(),
        "▱▱▱".to_string(),
    ],
    fps: Duration::from_millis(142),
    }
}

/// A hamburger spinner.
pub fn hamburger() -> Spinner {
    Spinner {
    frames: vec![
        "☱".to_string(),
        "☲".to_string(),
        "☴".to_string(),
        "☲".to_string(),
    ],
    fps: Duration::from_millis(333),
    }
}

/// An ellipsis spinner.
pub fn ellipsis() -> Spinner {
    Spinner {
    frames: vec![
        "".to_string(),
        ".".to_string(),
        "..".to_string(),
        "...".to_string(),
    ],
    fps: Duration::from_millis(333),
    }
}

/// Model contains the state for the spinner. Use [`new`] to create new models
/// rather than using Model as a struct literal.
pub struct Model {
    /// Spinner settings to use. See type [`Spinner`].
    pub spinner: Spinner,

    /// Style sets the styling for the spinner. Most of the time you'll just
    /// want foreground and background coloring, and potentially some padding.
    pub style: Style,

    frame: usize,
    id: i32,
    tag: i32,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("spinner::Model")
            .field("id", &self.id)
            .field("frame", &self.frame)
            .finish()
    }
}

/// ID returns the spinner's unique ID.
impl Model {
    /// ID returns the spinner's unique ID.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Update is the Tea update function.
    pub fn update(&mut self, msg: &dyn Msg) -> Cmd {
        if let Some(m) = msg.as_any().downcast_ref::<TickMsg>() {
            // If an ID is set, and the ID doesn't belong to this spinner,
            // reject the message.
            if m.id > 0 && m.id != self.id {
                return None;
            }

            // If a tag is set, and it's not the one we expect, reject the
            // message. This prevents the spinner from receiving too many
            // messages and thus spinning too fast.
            if m.tag > 0 && m.tag != self.tag {
                return None;
            }

            self.frame += 1;
            if self.frame >= self.spinner.frames.len() {
                self.frame = 0;
            }

            self.tag += 1;
            return self.tick(self.id, self.tag);
        }
        None
    }

    /// View renders the model's view.
    pub fn view(&self) -> String {
        if self.frame >= self.spinner.frames.len() {
            return "(error)".to_string();
        }

        self.style.render(&self.spinner.frames[self.frame])
    }

    /// Tick is the command used to advance the spinner one frame. Use this
    /// command to effectively start the spinner.
    pub fn tick_msg(&self) -> TickMsg {
        TickMsg {
            // The time at which the tick occurred.
            time: SystemTime::now(),

            // The ID of the spinner that this message belongs to. This can
            // be helpful when routing messages, however bear in mind that
            // spinners will ignore messages that don't contain ID by
            // default.
            id: self.id,

            tag: self.tag,
        }
    }

    fn tick(&self, id: i32, tag: i32) -> Cmd {
        let fps = self.spinner.fps;
        commands::tick(fps, move |t| {
            Some(Box::new(TickMsg {
                time: t,
                id,
                tag,
            }))
        })
    }
}

/// TickMsg indicates that the timer has ticked and we should render a frame.
#[derive(Debug, Clone)]
pub struct TickMsg {
    /// The time at which the tick occurred.
    pub time: SystemTime,
    tag: i32,
    /// The ID of the spinner that this message belongs to.
    pub id: i32,
}

/// Option is used to set options in [`new`]. For example:
///
/// ```rust
/// # use charming_bubbles::spinner;
/// let spinner = spinner::new(vec![spinner::with_spinner(spinner::dot())]);
/// ```
pub type Option = Box<dyn FnOnce(&mut Model)>;

/// WithSpinner is an option to set the spinner. Pass this to [`new`].
pub fn with_spinner(spinner: Spinner) -> Option {
    Box::new(move |m: &mut Model| {
        m.spinner = spinner;
    })
}

/// WithStyle is an option to set the spinner style. Pass this to [`new`].
pub fn with_style(style: Style) -> Option {
    Box::new(move |m: &mut Model| {
        m.style = style;
    })
}

/// New returns a model with default values.
pub fn new(opts: Vec<Option>) -> Model {
    let mut m = Model {
        spinner: line(),
        id: next_id(),
        frame: 0,
        tag: 0,
        style: Style::new(),
    };

    for opt in opts {
        opt(&mut m);
    }

    m
}
