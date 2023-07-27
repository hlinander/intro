#![allow(clippy::needless_pass_by_value)] // False positives with `impl ToString`

use std::f64::consts::TAU;
use std::hash::Hash;
use std::ops::RangeInclusive;

use crate::egui::*;

// ----------------------------------------------------------------------------

/// Same state for all [`Knob`]s.
#[derive(Clone, Debug, Default)]
pub(crate) struct MonoState {
    last_dragged_id: Option<Id>,
    last_dragged_value: Option<f64>,
    /// For temporary edit of a [`Knob`] value.
    /// Couples with the current focus id.
    edit_string: Option<String>,
}

impl MonoState {
    pub(crate) fn end_frame(&mut self, input: &InputState) {
        if input.pointer.any_pressed() || input.pointer.any_released() {
            self.last_dragged_id = None;
            self.last_dragged_value = None;
        }
    }
}

// ----------------------------------------------------------------------------

type NumFormatter<'a> = Box<dyn 'a + Fn(f64, RangeInclusive<usize>) -> String>;

// ----------------------------------------------------------------------------

/// Combined into one function (rather than two) to make it easier
/// for the borrow checker.
type GetSetValue<'a> = Box<dyn 'a + FnMut(Option<f64>) -> f64>;

fn get(get_set_value: &mut GetSetValue<'_>) -> f64 {
    (get_set_value)(None)
}

fn set(get_set_value: &mut GetSetValue<'_>, value: f64) {
    (get_set_value)(Some(value));
}

/// A numeric value that you can change by dragging the number. More compact than a [`Slider`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut my_f32: f32 = 0.0;
/// ui.add(egui::Knob::new(&mut my_f32).speed(0.1));
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Knob<'a> {
    get_set_value: GetSetValue<'a>,
    speed: f64,
    prefix: String,
    suffix: String,
    clamp_range: RangeInclusive<f64>,
    min_decimals: usize,
    max_decimals: Option<usize>,
    custom_formatter: Option<NumFormatter<'a>>,
    id: Id,
}

impl<'a> Knob<'a> {
    pub fn new<Num: emath::Numeric>(value: &'a mut Num) -> Self {
        let slf = Self::from_get_set(move |v: Option<f64>| {
            if let Some(v) = v {
                *value = Num::from_f64(v);
            }
            value.to_f64()
        });

        if Num::INTEGRAL {
            slf.max_decimals(0)
                .clamp_range(Num::MIN..=Num::MAX)
                .speed(0.25)
        } else {
            slf
        }
    }

    pub fn from_get_set(get_set_value: impl 'a + FnMut(Option<f64>) -> f64) -> Self {
        Self {
            get_set_value: Box::new(get_set_value),
            speed: 1.0,
            prefix: Default::default(),
            suffix: Default::default(),
            clamp_range: f64::NEG_INFINITY..=f64::INFINITY,
            min_decimals: 0,
            max_decimals: None,
            custom_formatter: None,
            id: Id::null(),
        }
    }

    /// How much the value changes when dragged one point (logical pixel).
    pub fn speed(mut self, speed: impl Into<f64>) -> Self {
        self.speed = speed.into();
        self
    }

    /// Clamp incoming and outgoing values to this range.
    pub fn clamp_range<Num: emath::Numeric>(mut self, clamp_range: RangeInclusive<Num>) -> Self {
        self.clamp_range = clamp_range.start().to_f64()..=clamp_range.end().to_f64();
        self
    }

    /// Show a prefix before the number, e.g. "x: "
    pub fn prefix(mut self, prefix: impl ToString) -> Self {
        self.prefix = prefix.to_string();
        self
    }

    /// Add a suffix to the number, this can be e.g. a unit ("°" or " m")
    pub fn suffix(mut self, suffix: impl ToString) -> Self {
        self.suffix = suffix.to_string();
        self
    }

    // TODO(emilk): we should also have a "min precision".
    /// Set a minimum number of decimals to display.
    /// Normally you don't need to pick a precision, as the slider will intelligently pick a precision for you.
    /// Regardless of precision the slider will use "smart aim" to help the user select nice, round values.
    pub fn min_decimals(mut self, min_decimals: usize) -> Self {
        self.min_decimals = min_decimals;
        self
    }

    // TODO(emilk): we should also have a "max precision".
    /// Set a maximum number of decimals to display.
    /// Values will also be rounded to this number of decimals.
    /// Normally you don't need to pick a precision, as the slider will intelligently pick a precision for you.
    /// Regardless of precision the slider will use "smart aim" to help the user select nice, round values.
    pub fn max_decimals(mut self, max_decimals: usize) -> Self {
        self.max_decimals = Some(max_decimals);
        self
    }

    pub fn max_decimals_opt(mut self, max_decimals: Option<usize>) -> Self {
        self.max_decimals = max_decimals;
        self
    }

    /// Set an exact number of decimals to display.
    /// Values will also be rounded to this number of decimals.
    /// Normally you don't need to pick a precision, as the slider will intelligently pick a precision for you.
    /// Regardless of precision the slider will use "smart aim" to help the user select nice, round values.
    pub fn fixed_decimals(mut self, num_decimals: usize) -> Self {
        self.min_decimals = num_decimals;
        self.max_decimals = Some(num_decimals);
        self
    }

    pub fn with_id(mut self, id: impl Hash) -> Self {
        self.id = Id::new(id);
        self
    }

    /// Set custom formatter defining how numbers are converted into text.
    ///
    /// A custom formatter takes a `f64` for the numeric value and a `RangeInclusive<usize>` representing
    /// the decimal range i.e. minimum and maximum number of decimal places shown.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_i64: i64 = 0;
    /// ui.add(egui::Knob::new(&mut my_i64).custom_formatter(|n, _| format!("{:X}", n as i64)));
    /// # });
    /// ```
    pub fn custom_formatter(
        mut self,
        formatter: impl 'a + Fn(f64, RangeInclusive<usize>) -> String,
    ) -> Self {
        self.custom_formatter = Some(Box::new(formatter));
        self
    }
}

impl<'a> Widget for Knob<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            mut get_set_value,
            speed,
            clamp_range,
            prefix,
            suffix,
            ..
        } = self;

        let shift = ui.input(|i| i.modifiers.shift);
        let ctrl = ui.input(|i| i.modifiers.ctrl);
        let is_slow_speed = shift;

        let old_value = get(&mut get_set_value);
        let value = clamp_to_range(old_value, clamp_range.clone());
        if old_value != value {
            set(&mut get_set_value, value);
        }
        let mut response = {
            let (rect, response) =
                ui.allocate_at_least(Vec2::new(20.0, 20.0), Sense::click_and_drag());
            ui.painter().circle(
                rect.center(),
                rect.width() / 2.0,
                Color32::DARK_GREEN,
                Stroke::from((2.0, Color32::BLACK)),
            );
            ui.painter().circle(
                rect.center(),
                0.8 * rect.width() / 2.0,
                Color32::GRAY,
                Stroke::from((2.0, Color32::BLACK)),
            );
            let angle_start = (TAU * 1.0 / 3.0) as f32;
            let angle: f32 = ((value / clamp_range.end()) * TAU) as f32 + angle_start;
            let n_points = ((angle - angle_start) * 20.0).ceil() as i8;
            let points: Vec<_> = (0..n_points)
                .map(|i| {
                    let phi = angle_start + (angle - angle_start) * (i as f32) / (n_points as f32);
                    let p =
                        rect.center() + 0.9 * rect.width() / 2.0 * Vec2::new(phi.cos(), phi.sin());
                    p
                })
                .collect();
            for window in points.windows(2) {
                let start_end = [window[0], window[1]];
                ui.painter()
                    .line_segment(start_end, Stroke::from((2.0, Color32::LIGHT_BLUE)));
            }

            let mut response = response.on_hover_cursor(CursorIcon::Grab);

            if ui.style().explanation_tooltips {
                response = response .on_hover_text(format!(
                    "{}{}{}\nDrag to edit or click to enter a value.\nPress 'Shift' while dragging for better control.",
                    prefix,
                    value as f32, // Show full precision value on-hover. TODO(emilk): figure out f64 vs f32
                    suffix
                ));
            }

            if response.clicked() {
            } else if response.dragged() {
                ui.output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);

                let mdelta = response.drag_delta();
                let delta_points = mdelta.x - mdelta.y; // Increase to the right and up

                let mut speed = speed;
                if shift {
                    speed *= 0.1;
                }
                if ctrl {
                    speed *= 0.1;
                }

                let delta_value = delta_points as f64 * speed;

                if delta_value != 0.0 {
                    set(&mut get_set_value, value + delta_value);
                }
            } else if response.has_focus() {
                let change = ui.input(|i| i.num_presses(Key::ArrowUp)) as f64
                    + ui.input(|i| i.num_presses(Key::ArrowRight)) as f64
                    - ui.input(|i| i.num_presses(Key::ArrowDown)) as f64
                    - ui.input(|i| i.num_presses(Key::ArrowLeft)) as f64;

                if change != 0.0 {
                    let new_value = value + speed * change;
                    let new_value = clamp_to_range(new_value, clamp_range);
                    set(&mut get_set_value, new_value);
                }
            }

            response
        };

        if get(&mut get_set_value) != old_value {
            response.mark_changed();
        }

        response.widget_info(|| WidgetInfo::drag_value(value));
        response
    }
}

fn clamp_to_range(x: f64, range: RangeInclusive<f64>) -> f64 {
    x.clamp(
        range.start().min(*range.end()),
        range.start().max(*range.end()),
    )
}