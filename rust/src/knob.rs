#![allow(clippy::needless_pass_by_value)] // False positives with `impl ToString`

use std::collections::VecDeque;
use std::f64::consts::TAU;
use std::hash::Hash;
use std::ops::RangeInclusive;

use crate::egui::*;

#[derive(PartialEq, Eq)]
pub enum KnobType {
    Input,
    Output,
    Undefined,
}
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
    response: Option<Response>,
    get_set_value: GetSetValue<'a>,
    speed: f64,
    name: &'static str,
    prefix: String,
    suffix: String,
    clamp_range: RangeInclusive<f64>,
    min_decimals: usize,
    max_decimals: Option<usize>,
    custom_formatter: Option<NumFormatter<'a>>,
    knob_color: Color32,
    knob_selected: bool,
    id: Id,
    knob_type: KnobType,
}

impl<'a> Knob<'a> {
    pub fn new<Num: emath::Numeric>(value: &'a mut Num, knob_id: Id) -> Self {
        let mut slf = Self::from_get_set(move |v: Option<f64>| {
            if let Some(v) = v {
                *value = Num::from_f64(v);
            }
            value.to_f64()
        });
        slf.id = knob_id;

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
            response: None,
            get_set_value: Box::new(get_set_value),
            name: " ",
            speed: 1.0,
            prefix: Default::default(),
            suffix: Default::default(),
            clamp_range: f64::NEG_INFINITY..=f64::INFINITY,
            min_decimals: 0,
            max_decimals: None,
            custom_formatter: None,
            knob_color: Color32::BLACK,
            knob_selected: false,
            knob_type: KnobType::Undefined,

            id: Id::null(),
        }
    }

    pub fn response(mut self, response: Response) -> Self {
        self.response = Some(response);
        self
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

    pub fn color(mut self, color: Color32) -> Self {
        self.knob_color = color;
        self
    }

    pub fn name(mut self, new_name: &'static str) -> Self {
        self.name = new_name;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.knob_selected = selected;
        self
    }

    pub fn with_type(mut self, knob_type: KnobType) -> Self {
        self.knob_type = knob_type;
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
            knob_color,
            ..
        } = self;
        let shift = ui.input(|i| i.modifiers.shift);
        let ctrl = ui.input(|i| i.modifiers.ctrl);
        let _is_slow_speed = shift;

        let old_value = get(&mut get_set_value);
        let value = clamp_to_range(old_value, clamp_range.clone());
        if old_value != value {
            set(&mut get_set_value, value);
        }
        let mut response = {
            let (rect, response) = if let Some(res) = self.response {
                (res.rect, res)
            } else {
                ui.allocate_at_least(Vec2::new(20.0, 20.0), Sense::click_and_drag())
            };
            let hovered = response.hovered();
            let history_size = 60;
            let mut data_buf = ui.memory_mut(|memory| {
                let data_buf = memory.data.get_temp_mut_or_insert_with(response.id, || {
                    VecDeque::with_capacity(history_size)
                });
                data_buf.clone()
            });

            let base_alpha = 0.0;
            let shadow_color = knob_color.gamma_multiply(
                base_alpha
                    + ((value / clamp_range.end()) as f32 * (1.0f32 - base_alpha as f32))
                        .clamp(0., 1.),
            );
            if self.knob_selected {
                ui.painter().circle(
                    rect.center(),
                    rect.width() / 1.5,
                    Color32::TRANSPARENT,
                    Stroke::from((2.0, Color32::RED)),
                );
            }
            ui.painter().add(
                Frame::none()
                    .shadow(epaint::Shadow {
                        extrusion: 6.0,
                        color: shadow_color,
                    })
                    .rounding(rect.width() * 0.5)
                    .paint(Rect::from_center_size(rect.center(), rect.size() * 0.8)),
            );
            ui.painter().circle(
                rect.center(),
                rect.width() / 2.0,
                if hovered {
                    Color32::WHITE
                } else {
                    Color32::GRAY.gamma_multiply(0.7)
                },
                Stroke::from((0.0, Color32::GRAY)),
            );
            ui.painter().circle(
                rect.center(),
                0.8 * rect.width() / 2.0,
                Color32::from_gray(20),
                Stroke::from((0.0, Color32::DARK_GRAY)),
            );
            if self.knob_type == KnobType::Output {
                // ui.painter().circle(
                //     rect.center(),
                //     rect.width() / 2.0,
                //     if hovered {
                //         Color32::WHITE
                //     } else {
                //         Color32::GRAY.gamma_multiply(0.7)
                //     },
                //     Stroke::from((0.0, Color32::GRAY)),
                // );
                ui.painter().circle(
                    rect.center(),
                    0.3 * rect.width() / 2.0,
                    Color32::from_gray(200),
                    Stroke::from((0.0, Color32::DARK_GRAY)),
                );
            }
            // draw_knob_text(ui, "hej", Color32::RED, rect);
            let angle_start = (TAU * 1.0 / 3.0) as f32;
            if data_buf.len() >= history_size {
                data_buf.pop_back();
            }
            data_buf.push_front(value);
            ui.memory_mut(|memory| {
                memory.data.insert_temp(response.id, data_buf.clone());
            });
            for (idx, value) in data_buf.iter().enumerate() {
                let angle: f32 = ((value / clamp_range.end()) * TAU) as f32 + angle_start;
                let n_points = ((angle - angle_start) * 20.0).ceil() as i8;
                let points: Vec<_> = (0..n_points)
                    .map(|i| {
                        let phi =
                            angle_start + (angle - angle_start) * (i as f32) / (n_points as f32);
                        let p = rect.center()
                            + 0.9 * rect.width() / 2.0 * Vec2::new(phi.cos(), phi.sin());
                        p
                    })
                    .collect();
                let t = idx as f32 / history_size as f32;
                let bias = 20.;
                //let t = 1. / (1. + t * bias);
                let t = (-bias * t).exp();
                let alpha = (t.clamp(0., 1.) * 256.) as u8;
                let col = Color32::LIGHT_BLUE;
                let col = Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), alpha);

                let stroke_width_bias = 15.;
                //let t = 1. / (1. + t * bias);
                let stroke_width = 1. - (-stroke_width_bias * t).exp();

                // println!("t: {t}, idx: {idx} alpha {alpha} color: {col:?}");
                for window in points.windows(2) {
                    let start_end = [window[0], window[1]];
                    ui.painter()
                        .line_segment(start_end, Stroke::from((stroke_width * 2.0, col)));
                }
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
            let input_popup_id = ui.make_persistent_id(self.id);
            if response.clicked_by(PointerButton::Middle) {
                ui.memory_mut(|mem| {
                    let toggle = mem
                        .data
                        .get_temp_mut_or_insert_with(input_popup_id, || false);
                    *toggle = !*toggle;
                });
            }
            if ui.input(|i| i.key_pressed(Key::Enter)) {
                ui.memory_mut(|mem| {
                    let toggle = mem
                        .data
                        .get_temp_mut_or_insert_with(input_popup_id, || false);
                    *toggle = false;
                });
            }
            if ui.memory(|mem| mem.data.get_temp(input_popup_id).unwrap_or_default()) {
                Area::new(input_popup_id)
                    .order(Order::Foreground)
                    .constrain(true)
                    .fixed_pos(response.rect.left_bottom())
                    .pivot(Align2::LEFT_TOP)
                    .show(ui.ctx(), |ui| {
                        let text_id = ui.make_persistent_id(input_popup_id);
                        let mut text: String = ui
                            .memory(|mem| mem.data.get_temp(text_id))
                            .unwrap_or_default();
                        ui.text_edit_singleline(&mut text);
                        ui.memory_mut(|mem| {
                            let new_text = mem
                                .data
                                .get_temp_mut_or_insert_with(text_id, || String::new());
                            *new_text = text.clone();
                        });
                        if let Some((maybe_nom, maybe_denom)) = text.rsplit_once("/") {
                            if let Ok(nom) = maybe_nom.parse::<i32>() {
                                if let Ok(denom) = maybe_denom.parse::<i32>() {
                                    set(&mut get_set_value, (nom as f64) / (denom as f64));
                                }
                            }
                        } else if let Ok(factor) = text.parse::<f64>() {
                            set(&mut get_set_value, factor);
                        }
                    });
            }
            // });
            if response.clicked() {
            } else if response.dragged_by(PointerButton::Secondary) {
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
            if hovered || response.dragged_by(PointerButton::Secondary) {
                show_tooltip_at_pointer(ui.ctx(), Id::new("value"), |ui| {
                    ui.label(format!("{}", value));
                });
            }
            draw_knob_text(
                ui,
                self.name.as_str(),
                if hovered {
                    Color32::WHITE
                } else {
                    Color32::GRAY
                },
                if hovered { 12.0 } else { 6.0 },
                response.rect,
            );

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

pub fn draw_knob_text(ui: &mut Ui, name: &str, color: Color32, font_size: f32, knob_rect: Rect) {
    let font_style = FontId::new(font_size, FontFamily::Proportional);
    // let font = ui.style().text_styles.get(&TextStyle::Small).unwrap();
    let job =
        epaint::text::LayoutJob::simple_singleline(name.to_string(), font_style.clone(), color);
    let galley = ui.fonts(|fonts| fonts.layout_job(job));
    let avg_height = galley.rows[0].glyphs.iter().map(|g| g.size.y).sum::<f32>()
        / galley.rows[0].glyphs.len() as f32;
    let radius = avg_height * 0.5 + knob_rect.width() * 0.5;
    let angle_width = galley.rows[0]
        .glyphs
        .iter()
        .last()
        .unwrap()
        .logical_rect()
        .left_top()
        .x
        / radius;
    for glyph in &galley.rows[0].glyphs {
        let glyph_job = epaint::text::LayoutJob::simple_singleline(
            glyph.chr.to_string(),
            font_style.clone(),
            color,
        );

        let glyph_galley = ui.fonts(|fonts| fonts.layout_job(glyph_job));
        let x = glyph.logical_rect().left_top().x;
        let angle = x / radius + std::f32::consts::PI * 0.5 - angle_width * 0.5;
        let x = angle.cos() * radius;
        let y = angle.sin() * radius;
        let glyph_local_distance_to_top_left = glyph
            .logical_rect()
            .center()
            .distance(glyph.logical_rect().left_top());
        let rotated_glyph_top_left = Vec2::new(
            angle.cos() * glyph_local_distance_to_top_left,
            angle.sin() * glyph_local_distance_to_top_left,
        );

        let center_pos = knob_rect.center().to_vec2() - Vec2::new(x, y);
        let mut text_shape = epaint::TextShape::new(
            (center_pos - rotated_glyph_top_left).to_pos2(),
            glyph_galley,
        );
        text_shape.angle = angle - core::f32::consts::PI * 0.5;
        // let mut text_shape = epaint::TextShape::new(
        // (knob_rect.left_top().to_vec2() + glyph.pos.to_vec2()).to_pos2(),
        // glyph_galley,
        // );
        ui.painter().add(text_shape);
    }
}
