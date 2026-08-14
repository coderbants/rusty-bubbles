//! Cleanroom Rust port of upstream Go source file: `progress/progress.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! <public-docs>
//! # Progress
//!
//! A simple progress bar for Bubble Tea applications.
//!
//! The spring-based animation is an inline port of
//! `github.com/charmbracelet/harmonica` (a simplified damped harmonic
//! oscillator), which the upstream progress component uses for animated
//! transitions.
//! </public-docs>

use rusty_bubbletea::commands;
use rusty_bubbletea::model::{Cmd, Msg};
use rusty_lipgloss::{self, Color, Style};
use rusty_x_ansi;
use std::fmt;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

/// ColorFunc is a function that can be used to dynamically fill the progress
/// bar based on the current percentage. total is the total filled percentage,
/// and current is the current percentage that is actively being filled with a
/// color.
pub type ColorFunc = Box<dyn Fn(f64, f64) -> Color + Send + Sync>;

/// Internal ID management. Used during animating to assure that frame
/// messages can only be received by progress components that sent them.
static LAST_ID: AtomicI64 = AtomicI64::new(0);

fn next_id() -> i32 {
    (LAST_ID.fetch_add(1, Ordering::SeqCst)) as i32
}

/// DefaultFullCharHalfBlock is the default character used to fill the
/// progress bar. It is a half block, which allows more granular color
/// blending control, by having a different foreground and background color,
/// doubling blending resolution.
pub const DEFAULT_FULL_CHAR_HALF_BLOCK: char = '▌';

/// DefaultFullCharFullBlock can also be used as a fill character for the
/// progress bar. Use this to disable the higher resolution blending which is
/// enabled when using [`DEFAULT_FULL_CHAR_HALF_BLOCK`].
pub const DEFAULT_FULL_CHAR_FULL_BLOCK: char = '█';

/// DefaultEmptyCharBlock is the default character used to fill the empty
/// portion of the progress bar.
pub const DEFAULT_EMPTY_CHAR_BLOCK: char = '░';

const FPS: u64 = 60;
const DEFAULT_WIDTH: usize = 40;
const DEFAULT_FREQUENCY: f64 = 18.0;
const DEFAULT_DAMPING: f64 = 1.0;

/// defaultBlendStart is the start of the default color blend (purple haze).
pub fn default_blend_start() -> Color {
    Color::parse("#5A56E0")
}

/// defaultBlendEnd is the end of the default color blend (neon pink).
pub fn default_blend_end() -> Color {
    Color::parse("#EE6FF8")
}

/// defaultFullColor is the default "filled" color (blueberry).
pub fn default_full_color() -> Color {
    Color::parse("#7571F9")
}

/// defaultEmptyColor is the default "empty" color (slate gray).
pub fn default_empty_color() -> Color {
    Color::parse("#606060")
}

/// Option is used to set options in [`new`]. For example:
///
/// ```rust
/// # use rusty_bubbles::progress;
/// # use rusty_lipgloss::Color;
/// let progress = progress::new(vec![
///     progress::with_colors(&[Color::parse("#5A56E0"), Color::parse("#EE6FF8")]),
///     progress::without_percentage(),
/// ]);
/// ```
/// Option is the type of configuration option that can be passed to [`new`].
/// (Named `Option` to mirror upstream; use `std::option::Option` for
/// optional values.)
pub type Option = Box<dyn FnOnce(&mut Model)>;

/// WithDefaultBlend sets a default blend of colors, which is a blend of
/// purple haze to neon pink.
pub fn with_default_blend() -> Option {
    with_colors(&[default_blend_start(), default_blend_end()])
}

/// WithColors sets the colors to use to fill the progress bar. Depending on
/// the number of colors passed in, will determine whether to use a solid fill
/// or a blend of colors.
///
/// - 0 colors: clears all previously set colors, setting them back to
///   defaults.
/// - 1 color: uses a solid fill with the given color.
/// - 2+ colors: uses a blend of the provided colors.
pub fn with_colors(colors: &[Color]) -> Option {
    let colors = colors.to_vec();
    if colors.is_empty() {
        return Box::new(|m: &mut Model| {
            m.full_color = default_full_color();
            m.blend = None;
            m.color_func = None;
        });
    }
    if colors.len() == 1 {
        return Box::new(move |m: &mut Model| {
            m.full_color = colors[0].clone();
            m.color_func = None;
            m.blend = None;
        });
    }
    Box::new(move |m: &mut Model| {
        m.blend = Some(colors.clone());
    })
}

/// WithColorFunc sets a function that can be used to dynamically fill the
/// progress bar based on the current percentage. total is the total filled
/// percentage, and current is the current percentage that is actively being
/// filled with a color. When specified, this overrides any other defined
/// colors and scaling.
///
/// Example: A progress bar that changes color based on the total completed
/// percentage:
///
/// ```rust
/// # use rusty_bubbles::progress;
/// # use rusty_lipgloss::Color;
/// progress::with_color_func(Box::new(|total, _current| {
///     if total <= 0.3 {
///         return Color::parse("#FF0000");
///     }
///     if total <= 0.7 {
///         return Color::parse("#00FF00");
///     }
///     Color::parse("#0000FF")
/// }));
/// ```
pub fn with_color_func(fn_: ColorFunc) -> Option {
    Box::new(move |m: &mut Model| {
        m.color_func = Some(fn_);
        m.blend = None;
    })
}

/// WithFillCharacters sets the characters used to construct the full and
/// empty components of the progress bar.
pub fn with_fill_characters(full: char, empty: char) -> Option {
    Box::new(move |m: &mut Model| {
        m.full = full;
        m.empty = empty;
    })
}

/// WithoutPercentage hides the numeric percentage.
pub fn without_percentage() -> Option {
    Box::new(|m: &mut Model| {
        m.show_percentage = false;
    })
}

/// WithWidth sets the initial width of the progress bar. Note that you can
/// also set the width via the `width` property, which can come in handy if
/// you're waiting for a `tea.WindowSizeMsg`.
pub fn with_width(w: usize) -> Option {
    Box::new(move |m: &mut Model| {
        m.set_width(w);
    })
}

/// WithSpringOptions sets the initial frequency and damping options for the
/// progress bar's built-in spring-based animation. Frequency corresponds to
/// speed, and damping to bounciness.
pub fn with_spring_options(frequency: f64, damping: f64) -> Option {
    Box::new(move |m: &mut Model| {
        m.set_spring_options(frequency, damping);
        m.spring_customized = true;
    })
}

/// WithScaled sets whether to scale the blend/gradient to fit the width of
/// only the filled portion of the progress bar. The default is false, which
/// means the percentage must be 100% to see the full color blend/gradient.
///
/// This is ignored when not using blending/multiple colors.
pub fn with_scaled(enabled: bool) -> Option {
    Box::new(move |m: &mut Model| {
        m.scale_blend = enabled;
    })
}

/// FrameMsg indicates that an animation step should occur.
#[derive(Debug, Clone)]
pub struct FrameMsg {
    id: i32,
    tag: i32,
}

/// Model stores values we'll use when rendering the progress bar.
pub struct Model {
    /// An identifier to keep us from receiving messages intended for other
    /// progress bars.
    id: i32,

    /// An identifier to keep us from receiving frame messages too quickly.
    tag: i32,

    /// Total width of the progress bar, including percentage, if set.
    width: usize,

    /// "Filled" sections of the progress bar.
    pub full: char,
    /// The color used for the filled sections.
    pub full_color: Color,

    /// "Empty" sections of the progress bar.
    pub empty: char,
    /// The color used for the empty sections.
    pub empty_color: Color,

    /// Settings for rendering the numeric percentage.
    pub show_percentage: bool,
    /// A fmt string for a float.
    pub percent_format: String,
    /// The style used for the percentage.
    pub percentage_style: Style,

    /// Members for animated transitions.
    spring: Spring,
    spring_customized: bool,
    /// percent currently displaying
    percent_shown: f64,
    /// percent to which we're animating
    target_percent: f64,
    velocity: f64,

    /// Blend of colors to use. When None, we use full_color.
    blend: std::option::Option<Vec<Color>>,

    /// When true, we scale the blended colors to fit the width of the filled
    /// section of the progress bar. When false, the width of the blend will
    /// be set to the full width of the progress bar.
    scale_blend: bool,

    /// color_func is used to dynamically fill the progress bar based on the
    /// current percentage.
    color_func: std::option::Option<ColorFunc>,
}

impl fmt::Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("progress::Model")
            .field("id", &self.id)
            .field("width", &self.width)
            .field("target_percent", &self.target_percent)
            .finish()
    }
}

/// New returns a model with default values.
pub fn new(opts: Vec<Option>) -> Model {
    let mut m = Model {
        id: next_id(),
        tag: 0,
        width: DEFAULT_WIDTH,
        full: DEFAULT_FULL_CHAR_HALF_BLOCK,
        full_color: default_full_color(),
        empty: DEFAULT_EMPTY_CHAR_BLOCK,
        empty_color: default_empty_color(),
        show_percentage: true,
        percent_format: " %3.0f%%".to_string(),
        percentage_style: Style::new(),
        spring: Spring::identity(),
        spring_customized: false,
        percent_shown: 0.0,
        target_percent: 0.0,
        velocity: 0.0,
        blend: None,
        scale_blend: false,
        color_func: None,
    };

    for opt in opts {
        opt(&mut m);
    }

    if !m.spring_customized {
        m.set_spring_options(DEFAULT_FREQUENCY, DEFAULT_DAMPING);
    }

    m
}

impl Model {
    /// Update is used to animate the progress bar during transitions. Use
    /// [`set_percent`](Self::set_percent) to create the command you'll need
    /// to trigger the animation.
    ///
    /// If you're rendering with [`view_as`](Self::view_as) you won't need
    /// this.
    pub fn update(&mut self, msg: &dyn Msg) -> Cmd {
        if let Some(m) = msg.as_any().downcast_ref::<FrameMsg>() {
            if m.id != self.id || m.tag != self.tag {
                return None;
            }

            // If we've more or less reached equilibrium, stop updating.
            if !self.is_animating() {
                return None;
            }

            let (pos, vel) =
                self.spring
                    .update(self.percent_shown, self.velocity, self.target_percent);
            self.percent_shown = pos;
            self.velocity = vel;
            return self.next_frame();
        }
        None
    }

    /// SetSpringOptions sets the frequency and damping for the current
    /// spring. Frequency corresponds to speed, and damping to bounciness.
    pub fn set_spring_options(&mut self, frequency: f64, damping: f64) {
        self.spring = Spring::new(
            Duration::from_secs(1).as_secs_f64() / FPS as f64,
            frequency,
            damping,
        );
    }

    /// Percent returns the current visible percentage on the model. This is
    /// only relevant when you're animating the progress bar.
    ///
    /// If you're rendering with [`view_as`](Self::view_as) you won't need
    /// this.
    pub fn percent(&self) -> f64 {
        self.target_percent
    }

    /// SetPercent sets the percentage state of the model as well as a
    /// command necessary for animating the progress bar to this new
    /// percentage.
    ///
    /// If you're rendering with [`view_as`](Self::view_as) you won't need
    /// this.
    pub fn set_percent(&mut self, p: f64) -> Cmd {
        self.target_percent = p.clamp(0.0, 1.0);
        self.tag += 1;
        self.next_frame()
    }

    /// IncrPercent increments the percentage by a given amount, returning a
    /// command necessary to animate the progress bar to the new percentage.
    ///
    /// If you're rendering with [`view_as`](Self::view_as) you won't need
    /// this.
    pub fn incr_percent(&mut self, v: f64) -> Cmd {
        self.set_percent(self.percent() + v)
    }

    /// DecrPercent decrements the percentage by a given amount, returning a
    /// command necessary to animate the progress bar to the new percentage.
    ///
    /// If you're rendering with [`view_as`](Self::view_as) you won't need
    /// this.
    pub fn decr_percent(&mut self, v: f64) -> Cmd {
        self.set_percent(self.percent() - v)
    }

    /// View renders an animated progress bar in its current state. To render
    /// a static progress bar based on your own calculations use
    /// [`view_as`](Self::view_as) instead.
    pub fn view(&self) -> String {
        self.view_as(self.percent_shown)
    }

    /// ViewAs renders the progress bar with a given percentage.
    pub fn view_as(&self, percent: f64) -> String {
        let mut b = String::new();
        let percent_view = self.percentage_view(percent);
        self.bar_view(&mut b, percent, rusty_x_ansi::string_width(&percent_view));
        b.push_str(&percent_view);
        b
    }

    /// SetWidth sets the width of the progress bar.
    pub fn set_width(&mut self, w: usize) {
        self.width = w;
    }

    /// Width returns the width of the progress bar.
    pub fn width(&self) -> usize {
        self.width
    }

    /// IsAnimating returns false if the progress bar reached equilibrium and
    /// is no longer animating.
    pub fn is_animating(&self) -> bool {
        let dist = (self.percent_shown - self.target_percent).abs();
        !(dist < 0.001 && self.velocity < 0.01)
    }

    fn next_frame(&self) -> Cmd {
        let id = self.id;
        let tag = self.tag;
        commands::tick(Duration::from_secs(1) / (FPS as u32), move |_| {
            Some(Box::new(FrameMsg { id, tag }))
        })
    }

    fn bar_view(&self, b: &mut String, percent: f64, text_width: usize) {
        let tw = self.width.saturating_sub(text_width); // total width
        let mut fw = ((tw as f64) * percent).round() as usize; // filled width

        fw = fw.min(tw);

        let is_half_block = self.full == DEFAULT_FULL_CHAR_HALF_BLOCK;

        if let Some(color_func) = &self.color_func {
            let mut style = Style::new();
            let mut current: f64;
            let half_block_perc = 0.5 / (tw as f64);
            for i in 0..fw {
                current = (i as f64) / (tw as f64);
                style = style.foreground_color(color_func(percent, current));
                if is_half_block {
                    let bg = color_func(percent, (current + half_block_perc).min(1.0));
                    style = style.background_color(bg);
                }
                b.push_str(&style.render(&self.full.to_string()));
            }
        } else if let Some(blend) = &self.blend {
            let mut multiplier = 1;
            if is_half_block {
                multiplier = 2;
            }

            let blend_colors = if self.scale_blend {
                rusty_lipgloss::blending::blend_1d(fw * multiplier, blend)
            } else {
                rusty_lipgloss::blending::blend_1d(tw * multiplier, blend)
            };

            // Blend fill.
            let mut blend_index = 0;
            for i in 0..fw {
                if !is_half_block {
                    b.push_str(
                        &Style::new()
                            .foreground_color(blend_colors[i].clone())
                            .render(&self.full.to_string()),
                    );
                    continue;
                }

                b.push_str(
                    &Style::new()
                        .foreground_color(blend_colors[blend_index].clone())
                        .background_color(blend_colors[blend_index + 1].clone())
                        .render(&self.full.to_string()),
                );
                blend_index += 2;
            }
        } else {
            // Solid fill.
            let repeat = self.full.to_string().repeat(fw);
            b.push_str(
                &Style::new()
                    .foreground_color(self.full_color.clone())
                    .render(&repeat),
            );
        }

        // Empty fill.
        let n = tw - fw;
        let repeat = self.empty.to_string().repeat(n);
        b.push_str(
            &Style::new()
                .foreground_color(self.empty_color.clone())
                .render(&repeat),
        );
    }

    fn percentage_view(&self, percent: f64) -> String {
        if !self.show_percentage {
            return String::new();
        }
        let percent = percent.clamp(0.0, 1.0);
        // Go's fmt.Sprintf with ` %3.0f%%` (leading space, right-aligned,
        // width 3, no decimals, escaped '%').
        let percentage = format!(" {:3.0}%", percent * 100.0);
        self.percentage_style
            .clone()
            .inline(true)
            .render(&percentage)
    }
}

/// A simplified damped harmonic oscillator, ported inline from
/// `github.com/charmbracelet/harmonica` (itself ported from Ryan Juckett's
/// simple damped harmonic motion).
#[derive(Debug, Clone, Copy)]
struct Spring {
    pos_pos_coef: f64,
    pos_vel_coef: f64,
    vel_pos_coef: f64,
    vel_vel_coef: f64,
}

impl Spring {
    fn identity() -> Spring {
        Spring {
            pos_pos_coef: 1.0,
            pos_vel_coef: 0.0,
            vel_pos_coef: 0.0,
            vel_vel_coef: 1.0,
        }
    }

    /// NewSpring initializes a new Spring, computing the parameters needed to
    /// simulate a damped spring over a given period of time.
    fn new(delta_time: f64, angular_frequency: f64, damping_ratio: f64) -> Spring {
        const EPSILON: f64 = f64::EPSILON;
        // Keep values in a legal range.
        let angular_frequency = angular_frequency.max(0.0);
        let damping_ratio = damping_ratio.max(0.0);

        // If there is no angular frequency, the spring will not move and we
        // can return identity.
        if angular_frequency < EPSILON {
            return Spring::identity();
        }

        if damping_ratio > 1.0 + EPSILON {
            // Over-damped.
            let za = -angular_frequency * damping_ratio;
            let zb = angular_frequency * (damping_ratio * damping_ratio - 1.0).sqrt();
            let z1 = za - zb;
            let z2 = za + zb;

            let e1 = (z1 * delta_time).exp();
            let e2 = (z2 * delta_time).exp();

            let inv_two_zb = 1.0 / (2.0 * zb); // = 1 / (z2 - z1)

            let e1_over_two_zb = e1 * inv_two_zb;
            let e2_over_two_zb = e2 * inv_two_zb;

            let z1e1_over_two_zb = z1 * e1_over_two_zb;
            let z2e2_over_two_zb = z2 * e2_over_two_zb;

            Spring {
                pos_pos_coef: e1_over_two_zb * z2 - z2e2_over_two_zb + e2,
                pos_vel_coef: -e1_over_two_zb + e2_over_two_zb,
                vel_pos_coef: (z1e1_over_two_zb - z2e2_over_two_zb + e2) * z2,
                vel_vel_coef: -z1e1_over_two_zb + z2e2_over_two_zb,
            }
        } else if damping_ratio < 1.0 - EPSILON {
            // Under-damped.
            let omega_zeta = angular_frequency * damping_ratio;
            let alpha = angular_frequency * (1.0 - damping_ratio * damping_ratio).sqrt();

            let exp_term = (-omega_zeta * delta_time).exp();
            let cos_term = (alpha * delta_time).cos();
            let sin_term = (alpha * delta_time).sin();

            let inv_alpha = 1.0 / alpha;

            let exp_sin = exp_term * sin_term;
            let exp_cos = exp_term * cos_term;
            let exp_omega_zeta_sin_over_alpha = exp_term * omega_zeta * sin_term * inv_alpha;

            Spring {
                pos_pos_coef: exp_cos + exp_omega_zeta_sin_over_alpha,
                pos_vel_coef: exp_sin * inv_alpha,
                vel_pos_coef: -exp_sin * alpha - omega_zeta * exp_omega_zeta_sin_over_alpha,
                vel_vel_coef: exp_cos - exp_omega_zeta_sin_over_alpha,
            }
        } else {
            // Critically damped.
            let exp_term = (-angular_frequency * delta_time).exp();
            let time_exp = delta_time * exp_term;
            let time_exp_freq = time_exp * angular_frequency;

            Spring {
                pos_pos_coef: time_exp_freq + exp_term,
                pos_vel_coef: time_exp,
                vel_pos_coef: -angular_frequency * time_exp_freq,
                vel_vel_coef: -time_exp_freq + exp_term,
            }
        }
    }

    /// Update updates position and velocity values against a given target
    /// value.
    fn update(&self, pos: f64, vel: f64, equilibrium_pos: f64) -> (f64, f64) {
        let old_pos = pos - equilibrium_pos; // update in equilibrium relative space
        let old_vel = vel;

        let new_pos = old_pos * self.pos_pos_coef + old_vel * self.pos_vel_coef + equilibrium_pos;
        let new_vel = old_pos * self.vel_pos_coef + old_vel * self.vel_vel_coef;

        (new_pos, new_vel)
    }
}
