//! The Slint platform the island runs on.
//!
//! Slint reaches a windowing system through [`Platform`] and [`WindowAdapter`]. This
//! module answers those with the hand-made Win32 window in [`crate::island_window`]
//! instead of winit, which is what lets the island window be born without the
//! styles a tiling window manager claims.
//!
//! Everything above this layer is ordinary Slint: the `.slint` language, properties,
//! animations, callbacks and models know nothing about any of it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use slint::platform::software_renderer::{RepaintBufferType, SoftwareRenderer};
use slint::platform::{Platform, PlatformError, Renderer, WindowAdapter, WindowEvent};
use slint::{PhysicalSize, Window};

use windows_sys::Win32::UI::WindowsAndMessaging::{
    MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx, QS_ALLINPUT,
};

use crate::island_window::{IslandWindow, process_pending_messages};

/// How often the island may be redrawn while something is animating.
const FRAME_TIME: Duration = Duration::from_millis(16);

/// The platform: it owns the island window and runs its message loop.
pub struct IslandPlatform {
    window_adapter: RefCell<Option<Rc<IslandWindowAdapter>>>,
}

impl IslandPlatform {
    pub fn new() -> Self {
        Self {
            window_adapter: RefCell::new(None),
        }
    }
}

impl Platform for IslandPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let adapter = IslandWindowAdapter::create()?;
        *self.window_adapter.borrow_mut() = Some(adapter.clone());
        Ok(adapter)
    }

    fn run_event_loop(&self) -> Result<(), PlatformError> {
        let adapter = self
            .window_adapter
            .borrow()
            .clone()
            .ok_or_else(|| PlatformError::Other("the island window was never created".into()))?;
        run_event_loop(&adapter)
    }
}

/// The bridge between Slint's window and the native island window.
struct IslandWindowAdapter {
    window: Window,
    renderer: SoftwareRenderer,
    island: Rc<IslandWindow>,
    size: PhysicalSize,
    redraw_requested: Cell<bool>,
}

impl IslandWindowAdapter {
    fn create() -> Result<Rc<Self>, PlatformError> {
        let island = IslandWindow::create()?;
        let (width, height) = island.physical_size();
        let scale_factor = island.scale_factor();

        // The renderer keeps the buffer between frames, which is true: the drawing
        // surface is this window's DIB and it is never moved.
        let adapter = Rc::new_cyclic(|weak_self| Self {
            window: Window::new(weak_self.clone()),
            renderer: SoftwareRenderer::new_with_repaint_buffer_type(RepaintBufferType::ReusedBuffer),
            island,
            size: PhysicalSize::new(width, height),
            redraw_requested: Cell::new(true),
        });

        // Slint must know the real scale factor before it lays out or draws anything.
        adapter
            .window
            .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor });

        Ok(adapter)
    }

    fn draw(&self) -> Result<(), PlatformError> {
        self.island
            .with_line_buffer(|lines| self.renderer.render_by_line(lines));
        self.island.present()
    }
}

impl WindowAdapter for IslandWindowAdapter {
    fn window(&self) -> &Window {
        &self.window
    }

    fn size(&self) -> PhysicalSize {
        self.size
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }

    fn set_visible(&self, visible: bool) -> Result<(), PlatformError> {
        if visible {
            self.island.show();
        } else {
            self.island.hide();
        }
        self.island.wake();
        Ok(())
    }

    fn request_redraw(&self) {
        self.redraw_requested.set(true);
        // Wake the message loop: an idle island is asleep with no timer running.
        self.island.wake();
    }
}

/// Runs until the island window is closed.
///
/// The loop advances Slint, draws when there is something to draw, and then sleeps
/// until the next Slint timer, the next animation frame, or the next message --
/// which is why a resting island costs nothing.
fn run_event_loop(adapter: &Rc<IslandWindowAdapter>) -> Result<(), PlatformError> {
    loop {
        if !process_pending_messages() {
            return Ok(());
        }

        slint::platform::update_timers_and_animations();

        let animating = adapter.window().has_active_animations();
        if adapter.redraw_requested.replace(false) || animating {
            adapter.draw()?;
        }

        let timeout = if animating {
            Some(FRAME_TIME)
        } else {
            slint::platform::duration_until_next_timer_update()
        };
        wait_for_events(timeout);
    }
}

/// Blocks until a Win32 message arrives or `timeout` elapses, leaving the messages
/// queued for the next iteration.
fn wait_for_events(timeout: Option<Duration>) {
    let milliseconds = match timeout {
        Some(duration) => duration.as_millis().min(u32::MAX as u128) as u32,
        None => u32::MAX, // INFINITE
    };
    unsafe {
        MsgWaitForMultipleObjectsEx(0, std::ptr::null(), milliseconds, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
    }
}
