#[cfg(not(target_os = "linux"))]
compile_error!("Keyboard Tester is Linux-only.");

mod background_input;
mod keyboard;
mod modifiers;
mod pin;

use gpui::{
    App, Application, Bounds, Context, Div, FocusHandle, KeyDownEvent, KeyUpEvent,
    ModifiersChangedEvent, Subscription, TitlebarOptions, Window, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use keyboard::{Key, ROWS, Tester, layout_keys};

const SURFACE: u32 = 0x191b1a;
const BORDER: u32 = 0x3a403d;
const INK: u32 = 0xf1f4f2;
const MUTED: u32 = 0xa9b5ae;
const READY: u32 = 0x262b28;
const TESTED: u32 = 0x285341;
const HELD: u32 = 0x87ebae;
const GAP: f32 = 4.0;
const PADDING: f32 = 24.0;
const MIN_OPACITY: u8 = 30;
const MAX_OPACITY: u8 = 100;
const OPACITY_STEP: u8 = 10;

struct KeyboardView {
    tester: Tester,
    focus: FocusHandle,
    caps_lock: bool,
    _activation: Subscription,
    _modifiers: gpui::Task<()>,
    pinned: Option<pin::Placement>,
    pin_error: Option<String>,
    background_input: Option<gpui::Task<()>>,
    opacity: u8,
}

impl KeyboardView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window);
        let activation = cx.observe_window_activation(window, |this, window, cx| {
            if this.background_input.is_some() {
                return;
            }
            if !window.is_window_active() {
                this.tester.clear_held();
            } else {
                this.caps_lock = window.capslock().on;
            }
            cx.notify();
        });
        let view = cx.weak_entity();
        let modifiers = modifiers::listen(window, cx, move |events, cx| {
            let _ = view.update(cx, |this, cx| {
                if this.background_input.is_some() {
                    return;
                }
                for (key, pressed) in events {
                    if pressed {
                        this.tester.press(key, false);
                    } else {
                        this.tester.release(key);
                    }
                }
                cx.notify();
            });
        });
        Self {
            tester: Tester::default(),
            focus,
            caps_lock: window.capslock().on,
            _activation: activation,
            _modifiers: modifiers,
            pinned: None,
            pin_error: None,
            background_input: None,
            opacity: MAX_OPACITY,
        }
    }

    fn start_background_input(&mut self, window: &Window, cx: &mut Context<Self>) {
        let Some(path) = std::env::var_os("KEYBOARD_TESTER_DEVICE") else {
            return;
        };
        let keyboard = match background_input::Keyboard::open(path.as_ref()) {
            Ok(keyboard) => keyboard,
            Err(error) => {
                self.pin_error = Some(format!("Background input: {error}"));
                return;
            }
        };
        let view = cx.weak_entity();
        self.tester.last = None;
        self.background_input = Some(window.spawn(cx, async move |cx| {
            const SAMPLE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(8);
            loop {
                cx.background_executor().timer(SAMPLE_INTERVAL).await;
                let held = keyboard.held();
                if !matches!(
                    cx.update(|window, cx| view.update(cx, |this, cx| {
                        match held {
                            Ok(held) => {
                                if this.tester.held != held {
                                    this.tester.tested.extend(held.iter().cloned());
                                    this.tester.held = held;
                                    cx.notify();
                                }
                                true
                            }
                            Err(error) => {
                                this.background_input = None;
                                this.tester.clear_held();
                                this.caps_lock = window.capslock().on;
                                this.pin_error = Some(format!("Background input: {error}"));
                                cx.notify();
                                false
                            }
                        }
                    })),
                    Ok(Ok(true))
                ) {
                    break;
                }
            }
        }));
    }

    fn opacity_control(&self, cx: &mut Context<Self>) -> Div {
        let step_button = |increase, id: &'static str, label| {
            let enabled = if increase {
                self.opacity < MAX_OPACITY
            } else {
                self.opacity > MIN_OPACITY
            };
            div()
                .id(id)
                .w(px(24.0))
                .text_center()
                .rounded_md()
                .border_1()
                .border_color(rgb(BORDER))
                .text_color(rgb(if enabled { INK } else { MUTED }))
                .when(enabled, |button| {
                    button
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(READY)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.opacity = if increase {
                                (this.opacity + OPACITY_STEP).min(MAX_OPACITY)
                            } else {
                                (this.opacity - OPACITY_STEP).max(MIN_OPACITY)
                            };
                            cx.notify();
                        }))
                })
                .child(label)
        };
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_1()
            .text_sm()
            .child(div().text_color(rgb(MUTED)).child("Opacity"))
            .child(step_button(false, "opacity-down", "\u{2212}"))
            .child(
                div()
                    .w(px(36.0))
                    .text_center()
                    .child(format!("{}%", self.opacity)),
            )
            .child(step_button(true, "opacity-up", "+"))
    }

    fn keycap(&self, key: &Key, unit: f32, compact: bool) -> Div {
        let width = key.width * unit;
        if key.id.is_empty() {
            return div().w(px(width)).flex_shrink_0();
        }
        let held = self.tester.held.contains(key.id);
        let tested = self.tester.tested.contains(key.id);
        let (background, text, border) = if held {
            (HELD, SURFACE, HELD)
        } else if tested {
            (TESTED, INK, 0x417a61)
        } else {
            (READY, INK, BORDER)
        };
        div()
            .w(px(width - GAP))
            .mr(px(GAP))
            .h(px(if compact { 27.0 } else { 40.0 }))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .border_1()
            .border_color(rgb(border))
            .bg(rgb(background))
            .text_color(rgb(text))
            .text_size(px(if compact { 10.0 } else { 12.0 }))
            .child(key.label)
    }
}

impl Render for KeyboardView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let layout = layout_keys();
        let total = layout.len();
        let tested = layout
            .iter()
            .filter(|&&id| self.tester.tested.contains(id))
            .count();
        let compact = self.pinned.is_some();
        let padding = if compact { 10.0 } else { PADDING };
        let unit = ((f32::from(window.viewport_size().width) - 2.0 * padding)
            / ROWS[0].iter().map(|key| key.width).sum::<f32>())
        .clamp(22.0, 57.0);

        div()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if this.background_input.is_some() {
                    return;
                }
                this.tester.press(&event.keystroke.key, event.is_held);
                cx.notify();
            }))
            .on_key_up(cx.listener(|this, event: &KeyUpEvent, _, cx| {
                if this.background_input.is_some() {
                    return;
                }
                this.tester.release(&event.keystroke.key);
                cx.notify();
            }))
            .on_modifiers_changed(cx.listener(|this, event: &ModifiersChangedEvent, _, cx| {
                if this.background_input.is_some() {
                    return;
                }
                if this.caps_lock != event.capslock.on {
                    this.caps_lock = event.capslock.on;
                    this.tester.set_caps_lock(this.caps_lock);
                }
                cx.notify();
            }))
            .size_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            .items_center()
            .bg(gpui::Hsla::from(rgb(SURFACE))
                .opacity(f32::from(self.opacity) / f32::from(MAX_OPACITY)))
            .text_color(rgb(INK))
            .p(px(padding))
            .child(
                div()
                    .w_full()
                    .max_w(px(1042.0))
                    .flex()
                    .flex_col()
                    .gap(px(if compact { 6.0 } else { 16.0 }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Keyboard Tester"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(self.opacity_control(cx))
                                    .children(pin::available().then(|| {
                                        div()
                                            .id("pin")
                                            .cursor_pointer()
                                            .border_1()
                                            .rounded_md()
                                            .border_color(rgb(if compact { HELD } else { BORDER }))
                                            .px_3()
                                            .py_1()
                                            .text_sm()
                                            .hover(|style| style.bg(rgb(READY)))
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                let result = if let Some(placement) = &this.pinned {
                                                    placement.restore().map(|()| None)
                                                } else {
                                                    pin::Placement::pin().map(Some)
                                                };
                                                match result {
                                                    Ok(placement) => {
                                                        this.pinned = placement;
                                                        this.pin_error = None;
                                                        if this.pinned.is_some() {
                                                            this.start_background_input(window, cx);
                                                        } else if this
                                                            .background_input
                                                            .take()
                                                            .is_some()
                                                        {
                                                            this.tester.clear_held();
                                                            this.caps_lock = window.capslock().on;
                                                        }
                                                        this.focus.focus(window);
                                                    }
                                                    Err(error) => {
                                                        this.pin_error = Some(error.to_string())
                                                    }
                                                }
                                                cx.notify();
                                            }))
                                            .child(if compact { "Unpin" } else { "Pin" })
                                    }))
                                    .child(
                                        div()
                                            .id("reset")
                                            .cursor_pointer()
                                            .rounded_md()
                                            .border_1()
                                            .border_color(rgb(BORDER))
                                            .px_3()
                                            .py_1()
                                            .text_sm()
                                            .hover(|style| style.bg(rgb(READY)))
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.tester = Tester::default();
                                                this.caps_lock = window.capslock().on;
                                                cx.notify();
                                            }))
                                            .child("Reset"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child("ANSI TKL")
                            .child(format!("{tested} / {total} tested")),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(GAP))
                            .children(ROWS.iter().map(|row| {
                                div()
                                    .flex()
                                    .children(row.iter().map(|key| self.keycap(key, unit, compact)))
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_t_1()
                            .border_color(rgb(BORDER))
                            .pt(px(if compact { 3.0 } else { 12.0 }))
                            .text_sm()
                            .child(div().flex().gap_4().children(
                                [(READY, "Untested"), (TESTED, "Tested"), (HELD, "Held")].map(
                                    |(color, label)| {
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .size_3()
                                                    .rounded_sm()
                                                    .bg(rgb(color))
                                                    .border_1()
                                                    .border_color(rgb(BORDER)),
                                            )
                                            .child(label)
                                    },
                                ),
                            ))
                            .child(if self.background_input.is_some() {
                                format!("Background input · {} held", self.tester.held.len())
                            } else if window.is_window_active() {
                                format!("{} held", self.tester.held.len())
                            } else {
                                "Not focused".to_owned()
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .text_sm()
                            .child(div().text_color(rgb(MUTED)).child("Last event"))
                            .child(
                                self.pin_error
                                    .clone()
                                    .or_else(|| self.tester.last.clone())
                                    .unwrap_or_else(|| "None".into()),
                            ),
                    ),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1000.0), px(500.0)), cx);
        if let Err(error) = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(480.0), px(275.0))),
                window_background: WindowBackgroundAppearance::Transparent,
                app_id: Some("keyboard-tester".into()),
                titlebar: Some(TitlebarOptions {
                    title: Some("Keyboard Tester".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| KeyboardView::new(window, cx)),
        ) {
            eprintln!("Could not open Keyboard Tester: {error:#}");
            std::process::exit(1);
        }
        cx.activate(true);
    });
}
