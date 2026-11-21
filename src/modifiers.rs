use std::{error::Error, time::Duration};

use gpui::{App, Task, Window};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    protocol::{wl_keyboard, wl_registry, wl_seat},
};
use x11rb::{
    connection::Connection as _,
    protocol::{
        Event, xinput,
        xproto::{AtomEnum, ConnectionExt as _},
    },
    rust_connection::RustConnection,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type KeyEvent = (&'static str, bool);

fn modifier(code: u32) -> Option<&'static str> {
    // Linux input-event-codes.h values; X11 adds the XKB offset of eight.
    Some(match code {
        29 => "lctrl",
        97 => "rctrl",
        42 => "lshift",
        54 => "rshift",
        56 => "lalt",
        100 => "ralt",
        125 => "lsuper",
        126 => "rsuper",
        _ => return None,
    })
}

#[derive(Default)]
struct WaylandInput {
    surface_id: u32,
    focused: bool,
    events: Vec<KeyEvent>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandInput {
    fn event(
        _: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        queue: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
            && interface == "wl_seat"
        {
            registry.bind::<wl_seat::WlSeat, _, _>(name, version.min(5), queue, ());
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WaylandInput {
    fn event(
        _: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        queue: &QueueHandle<Self>,
    ) {
        use wayland_client::WEnum;
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(caps),
        } = event
            && caps.contains(wl_seat::Capability::Keyboard)
        {
            seat.get_keyboard(queue, ());
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for WaylandInput {
    fn event(
        input: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use wayland_client::WEnum;
        match event {
            wl_keyboard::Event::Enter { surface, .. } => {
                input.focused = surface.id().protocol_id() == input.surface_id
            }
            wl_keyboard::Event::Leave { .. } => input.focused = false,
            wl_keyboard::Event::Key {
                key,
                state: WEnum::Value(state),
                ..
            } if input.focused => {
                if let Some(id) = modifier(key) {
                    input
                        .events
                        .push((id, state == wl_keyboard::KeyState::Pressed));
                }
            }
            _ => {}
        }
    }
}

struct X11Input {
    connection: RustConnection,
    pid_atom: u32,
    subscribed: bool,
}

impl X11Input {
    fn new() -> Result<Self> {
        let (connection, _) = x11rb::connect(None)?;
        let pid_atom = connection.intern_atom(false, b"_NET_WM_PID")?.reply()?.atom;
        Ok(Self {
            connection,
            pid_atom,
            subscribed: false,
        })
    }

    fn poll(&mut self, active: bool) -> Result<Vec<KeyEvent>> {
        use xinput::{ConnectionExt as _, XIEventMask};
        let connection = &self.connection;
        // GPUI 0.2.2 does not expose its X11 window handle. Verify the focused
        // window belongs to this process before subscribing to that window only.
        if !self.subscribed && active {
            let window = connection.get_input_focus()?.reply()?.focus;
            let property = connection
                .get_property(false, window, self.pid_atom, AtomEnum::CARDINAL, 0, 1)?
                .reply()?;
            if property.value32().and_then(|mut values| values.next()) == Some(std::process::id()) {
                const ALL_MASTER_DEVICES: u16 = 1;
                connection
                    .xinput_xi_select_events(
                        window,
                        &[xinput::EventMask {
                            deviceid: ALL_MASTER_DEVICES,
                            mask: vec![XIEventMask::KEY_PRESS | XIEventMask::KEY_RELEASE],
                        }],
                    )?
                    .check()?;
                self.subscribed = true;
            }
        }
        let mut events = Vec::new();
        while let Some(event) = connection.poll_for_event()? {
            let (code, pressed) = match event {
                Event::XinputKeyPress(event) => (event.detail, true),
                Event::XinputKeyRelease(event) => (event.detail, false),
                _ => continue,
            };
            if active && let Some(id) = code.checked_sub(8).and_then(modifier) {
                events.push((id, pressed));
            }
        }
        Ok(events)
    }
}

enum Input {
    Wayland(Connection, EventQueue<WaylandInput>, WaylandInput),
    X11(Box<X11Input>),
}

impl Input {
    fn new(window: &Window) -> Result<Self> {
        if gpui::guess_compositor() != "Wayland" {
            return Ok(Self::X11(Box::new(X11Input::new()?)));
        }
        let (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(surface)) = (
            window
                .display_handle()
                .map_err(|_| "No window display handle")?
                .as_raw(),
            HasWindowHandle::window_handle(window)
                .map_err(|_| "No window surface handle")?
                .as_raw(),
        ) else {
            return Err("Expected a Wayland window".into());
        };
        // The GPUI platform owns the display. This guest queue is used only on
        // its main thread, and the application keeps the display alive until it exits.
        let backend = unsafe {
            wayland_client::backend::Backend::from_foreign_display(display.display.as_ptr().cast())
        };
        let id = unsafe {
            wayland_client::backend::ObjectId::from_ptr(
                wl_surface_interface(),
                surface.surface.as_ptr().cast(),
            )
        }?;
        let connection = Connection::from_backend(backend);
        let queue = connection.new_event_queue();
        connection.display().get_registry(&queue.handle(), ());
        connection.flush()?;
        Ok(Self::Wayland(
            connection,
            queue,
            WaylandInput {
                surface_id: id.protocol_id(),
                ..Default::default()
            },
        ))
    }

    fn poll(&mut self, active: bool) -> Result<Vec<KeyEvent>> {
        match self {
            Self::Wayland(connection, queue, input) => {
                queue.dispatch_pending(input)?;
                connection.flush()?;
                Ok(std::mem::take(&mut input.events))
            }
            Self::X11(input) => input.poll(active),
        }
    }
}

fn wl_surface_interface() -> &'static wayland_client::backend::protocol::Interface {
    wayland_client::protocol::wl_surface::WlSurface::interface()
}

pub fn listen(
    window: &Window,
    cx: &App,
    apply: impl Fn(Vec<KeyEvent>, &mut App) + 'static,
) -> Task<()> {
    window.spawn(cx, async move |cx| {
        let mut input = match cx.update(|window, _| Input::new(window)) {
            Ok(Ok(input)) => input,
            Ok(Err(error)) => {
                eprintln!("Could not initialize modifier input: {error}");
                return;
            }
            Err(_) => return,
        };
        loop {
            match cx.update(|window, cx| {
                let active = window.is_window_active();
                let events = input.poll(active)?;
                if active && !events.is_empty() {
                    apply(events, cx);
                }
                Ok::<(), Box<dyn Error>>(())
            }) {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    eprintln!("Modifier input stopped: {error}");
                    break;
                }
                Err(_) => break,
            }
            cx.background_executor()
                .timer(Duration::from_millis(12))
                .await;
        }
    })
}
