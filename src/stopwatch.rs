//! Cleanroom Rust port of upstream Go source file: `stopwatch/stopwatch.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! <public-docs>
//! # Stopwatch
//!
//! A simple stopwatch component.
//! </public-docs>

use crate::internal::duration::duration_string;
use rusty_bubbletea::commands;
use rusty_bubbletea::model::{Cmd, Msg};
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
/// # use rusty_bubbles::stopwatch;
/// # use std::time::Duration;
/// let timer = stopwatch::new(vec![stopwatch::with_interval(Duration::from_secs(5))]);
/// ```
pub type Option = Box<dyn FnOnce(&mut Model)>;

/// WithInterval is an option for setting the interval between ticks. Pass as
/// an argument to [`new`].
pub fn with_interval(interval: Duration) -> Option {
    Box::new(move |m: &mut Model| {
        m.interval = interval;
    })
}

/// TickMsg is a message that is sent on every timer tick.
#[derive(Debug, Clone)]
pub struct TickMsg {
    /// ID is the identifier of the stopwatch that sends the message. This
    /// makes it possible to determine which stopwatch a tick belongs to when
    /// there are multiple stopwatches running.
    ///
    /// Note, however, that a stopwatch will reject ticks from other
    /// stopwatches, so it's safe to flow all TickMsgs through all stopwatches
    /// and have them still behave appropriately.
    pub id: i32,
    tag: i32,
}

/// StartStopMsg is sent when the stopwatch should start or stop.
#[derive(Debug, Clone)]
pub struct StartStopMsg {
    /// The ID of the stopwatch the message is intended for.
    pub id: i32,
    running: bool,
}

/// ResetMsg is sent when the stopwatch should reset.
#[derive(Debug, Clone)]
pub struct ResetMsg {
    /// The ID of the stopwatch the message is intended for.
    pub id: i32,
}

/// Model for the stopwatch component.
#[derive(Clone)]
pub struct Model {
    d: Duration,
    id: i32,
    tag: i32,
    running: bool,

    /// How long to wait before every tick. Defaults to 1 second.
    pub interval: Duration,
}

impl fmt::Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("stopwatch::Model")
            .field("id", &self.id)
            .field("running", &self.running)
            .field("d", &self.d)
            .finish()
    }
}

/// New creates a new stopwatch with 1s interval.
pub fn new(opts: Vec<Option>) -> Model {
    let mut m = Model {
        id: next_id(),
        interval: Duration::from_secs(1),
        d: Duration::ZERO,
        tag: 0,
        running: false,
    };

    for opt in opts {
        opt(&mut m);
    }
    m
}

impl Model {
    /// ID returns the unique ID of the model.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Init starts the stopwatch.
    pub fn init(&mut self) -> Cmd {
        self.start()
    }

    /// Start starts the stopwatch.
    pub fn start(&mut self) -> Cmd {
        let start_msg: Box<dyn Msg> = Box::new(StartStopMsg {
            id: self.id,
            running: true,
        });
        let tick_cmd = tick(self.id, self.tag, self.interval);
        commands::sequence(vec![Some(Box::new(move || Some(start_msg))), tick_cmd])
    }

    /// Stop stops the stopwatch.
    pub fn stop(&mut self) -> Cmd {
        let id = self.id;
        Some(Box::new(move || {
            Some(Box::new(StartStopMsg { id, running: false }))
        }))
    }

    /// Toggle stops the stopwatch if it is running and starts it if it is
    /// stopped.
    pub fn toggle(&mut self) -> Cmd {
        if self.running {
            return self.stop();
        }
        self.start()
    }

    /// Reset resets the stopwatch to 0.
    pub fn reset(&mut self) -> Cmd {
        let id = self.id;
        Some(Box::new(move || Some(Box::new(ResetMsg { id }))))
    }

    /// Running returns true if the stopwatch is running or false if it is
    /// stopped.
    pub fn running(&self) -> bool {
        self.running
    }

    /// Update handles the timer tick.
    pub fn update(&mut self, msg: &dyn Msg) -> Cmd {
        if let Some(m) = msg.as_any().downcast_ref::<StartStopMsg>() {
            if m.id != self.id {
                return None;
            }
            self.running = m.running;
            return None;
        }

        if let Some(m) = msg.as_any().downcast_ref::<ResetMsg>() {
            if m.id != self.id {
                return None;
            }
            self.d = Duration::ZERO;
            return None;
        }

        if let Some(m) = msg.as_any().downcast_ref::<TickMsg>() {
            if !self.running || m.id != self.id {
                return None;
            }

            // If a tag is set, and it's not the one we expect, reject the
            // message. This prevents the stopwatch from receiving too many
            // messages and thus ticking too fast.
            if m.tag > 0 && m.tag != self.tag {
                return None;
            }

            self.d += self.interval;
            self.tag += 1;
            return tick(self.id, self.tag, self.interval);
        }

        None
    }

    /// Elapsed returns the time elapsed.
    pub fn elapsed(&self) -> Duration {
        self.d
    }

    /// View of the timer component.
    pub fn view(&self) -> String {
        duration_string(self.d)
    }
}

fn tick(id: i32, tag: i32, d: Duration) -> Cmd {
    commands::tick(d, move |_| Some(Box::new(TickMsg { id, tag })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stopwatch_lifecycle() {
        let mut m = new(vec![with_interval(Duration::from_millis(50))]);
        assert_eq!(m.elapsed(), Duration::ZERO);
        assert!(!m.running());
        assert_eq!(m.view(), "0s");
        assert!(format!("{m:?}").contains("stopwatch::Model"));

        // Init / start
        let cmd = m.init();
        assert!(cmd.is_some());

        // Update StartStopMsg
        m.update(&StartStopMsg {
            id: m.id(),
            running: true,
        });
        assert!(m.running());

        // Mismatched StartStopMsg ignored
        m.update(&StartStopMsg {
            id: m.id() + 999,
            running: false,
        });
        assert!(m.running());

        // TickMsg
        let tick_cmd = m.update(&TickMsg {
            id: m.id(),
            tag: m.tag,
        });
        assert!(tick_cmd.is_some());
        assert_eq!(m.elapsed(), Duration::from_millis(50));
        assert_eq!(m.view(), "50ms");

        // Stale tag ignored
        let stale = m.update(&TickMsg {
            id: m.id(),
            tag: 999,
        });
        assert!(stale.is_none());

        // Stop
        let stop_cmd = m.stop();
        assert!(stop_cmd.is_some());
        m.update(&StartStopMsg {
            id: m.id(),
            running: false,
        });
        assert!(!m.running());

        // Toggle from stopped starts
        let toggle_cmd = m.toggle();
        assert!(toggle_cmd.is_some());

        // Toggle from running stops
        m.running = true;
        let toggle_cmd2 = m.toggle();
        assert!(toggle_cmd2.is_some());

        // Reset
        let reset_cmd = m.reset();
        assert!(reset_cmd.is_some());
        m.update(&ResetMsg { id: m.id() });
        assert_eq!(m.elapsed(), Duration::ZERO);

        // Mismatched ResetMsg ignored
        m.d = Duration::from_secs(10);
        m.update(&ResetMsg { id: m.id() + 999 });
        assert_eq!(m.elapsed(), Duration::from_secs(10));
    }
}
