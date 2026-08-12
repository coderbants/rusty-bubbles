//! Cleanroom Rust port of upstream Go source file: `timer/timer.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! <public-docs>
//! # Timer
//!
//! A simple timeout component.
//! </public-docs>

use crate::internal::duration::duration_string;
use charming_bubbletea::commands;
use charming_bubbletea::model::{Cmd, Msg};
use std::fmt;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

static LAST_ID: AtomicI64 = AtomicI64::new(0);

fn next_id() -> i32 {
    (LAST_ID.fetch_add(1, Ordering::SeqCst)) as i32
}

/// Option is a configuration option in [`new`]. For example:
///
/// ```rust
/// # use charming_bubbles::timer;
/// # use std::time::Duration;
/// let timer = timer::new(Duration::from_secs(10), vec![timer::with_interval(Duration::from_secs(5))]);
/// ```
pub type Option = Box<dyn FnOnce(&mut Model)>;

/// WithInterval is an option for setting the interval between ticks. Pass as
/// an argument to [`new`].
pub fn with_interval(interval: Duration) -> Option {
    Box::new(move |m: &mut Model| {
        m.interval = interval;
    })
}

/// StartStopMsg is used to start and stop the timer.
#[derive(Debug, Clone)]
pub struct StartStopMsg {
    /// The ID of the timer the message is intended for.
    pub id: i32,
    running: bool,
}

/// TickMsg is a message that is sent on every timer tick.
#[derive(Debug, Clone)]
pub struct TickMsg {
    /// ID is the identifier of the timer that sends the message. This makes
    /// it possible to determine which timer a tick belongs to when there
    /// are multiple timers running.
    ///
    /// Note, however, that a timer will reject ticks from other timers, so
    /// it's safe to flow all TickMsgs through all timers and have them still
    /// behave appropriately.
    pub id: i32,

    /// Timeout returns whether or not this tick is a timeout tick. You can
    /// alternatively listen for TimeoutMsg.
    pub timeout: bool,

    tag: i32,
}

/// TimeoutMsg is a message that is sent once when the timer times out.
///
/// It's a convenience message sent alongside a TickMsg with the Timeout value
/// set to true.
#[derive(Debug, Clone)]
pub struct TimeoutMsg {
    /// The ID of the timer that timed out.
    pub id: i32,
}

/// Model of the timer component.
#[derive(Clone)]
pub struct Model {
    /// How long until the timer expires.
    pub timeout: Duration,

    /// How long to wait before every tick. Defaults to 1 second.
    pub interval: Duration,

    id: i32,
    tag: i32,
    running: bool,
}

impl fmt::Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("timer::Model")
            .field("id", &self.id)
            .field("timeout", &self.timeout)
            .field("running", &self.running)
            .finish()
    }
}

/// New creates a new timer with the given timeout and default 1s interval.
pub fn new(timeout: Duration, opts: Vec<Option>) -> Model {
    let mut m = Model {
        timeout,
        interval: Duration::from_secs(1),
        running: true,
        id: next_id(),
        tag: 0,
    };
    for opt in opts {
        opt(&mut m);
    }
    m
}

impl Model {
    /// ID returns the model's identifier. This can be used to determine if
    /// messages belong to this timer instance when there are multiple timers.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Running returns whether or not the timer is running. If the timer has
    /// timed out this will always return false.
    pub fn running(&self) -> bool {
        if self.timedout() || !self.running {
            return false;
        }
        true
    }

    /// Timedout returns whether or not the timer has timed out.
    pub fn timedout(&self) -> bool {
        self.timeout <= Duration::ZERO
    }

    /// Init starts the timer.
    pub fn init(&mut self) -> Cmd {
        self.tick()
    }

    /// Update handles the timer tick.
    pub fn update(&mut self, msg: &dyn Msg) -> Cmd {
        if let Some(m) = msg.as_any().downcast_ref::<StartStopMsg>() {
            if m.id != 0 && m.id != self.id {
                return None;
            }
            self.running = m.running;
            return self.tick();
        }

        if let Some(m) = msg.as_any().downcast_ref::<TickMsg>() {
            if !self.running() || (m.id != 0 && m.id != self.id) {
                return None;
            }

            // If a tag is set, and it's not the one we expect, reject the
            // message. This prevents the ticker from receiving too many
            // messages and thus ticking too fast.
            if m.tag > 0 && m.tag != self.tag {
                return None;
            }

            self.timeout = self.timeout.saturating_sub(self.interval);
            let tick_cmd = self.tick();
            let timeout_cmd = self.timeout_msg();
            return commands::batch(vec![tick_cmd, timeout_cmd]);
        }

        None
    }

    /// View of the timer component.
    pub fn view(&self) -> String {
        duration_string(self.timeout)
    }

    /// Start resumes the timer. Has no effect if the timer has timed out.
    pub fn start(&mut self) -> Cmd {
        self.start_stop(true)
    }

    /// Stop pauses the timer. Has no effect if the timer has timed out.
    pub fn stop(&mut self) -> Cmd {
        self.start_stop(false)
    }

    /// Toggle stops the timer if it's running and starts it if it's stopped.
    pub fn toggle(&mut self) -> Cmd {
        self.start_stop(!self.running())
    }

    fn tick(&mut self) -> Cmd {
        let id = self.id;
        let tag = self.tag;
        let timeout = self.timedout();
        let interval = self.interval;
        commands::tick(interval, move |_| {
            Some(Box::new(TickMsg { id, tag, timeout }))
        })
    }

    fn timeout_msg(&self) -> Cmd {
        if !self.timedout() {
            return None;
        }
        let id = self.id;
        Some(Box::new(move || Some(Box::new(TimeoutMsg { id }))))
    }

    fn start_stop(&mut self, v: bool) -> Cmd {
        let id = self.id;
        Some(Box::new(move || {
            Some(Box::new(StartStopMsg { id, running: v }))
        }))
    }
}
