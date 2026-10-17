// Capri Bar — the island.
//
// Integration 2: the island lives in a window this process creates and owns, so it is born
// outside the layout of a tiling window manager and the app no longer depends on Slint's winit
// backend. Integration 3 adds the first widget: the resting island shows the hour.
//
// The island still does not respond to the pointer: no interaction, no expanded state yet.

mod clock;
mod island_platform;
mod island_window;

use std::rc::Rc;

slint::include_modules!();

#[cfg(not(windows))]
compile_error!(
    "The island window is written against Win32 (src/island_window.rs). Linux is a destination, not today's target."
);

fn main() -> Result<(), slint::PlatformError> {
    // The platform creates the native window, so it has to be in place before the component
    // asks for one.
    slint::platform::set_platform(Box::new(island_platform::IslandPlatform::new())).map_err(
        |error| slint::PlatformError::Other(format!("could not install the island platform: {error:?}")),
    )?;

    let ui = Island::new()?;
    ui.set_time(clock::current_time().into());

    // The island wakes only when what it shows changes: one single-shot timer armed at the next
    // minute boundary, re-armed from its own callback.
    let tick = Rc::new(slint::Timer::default());
    arm_clock(&ui.as_weak(), &tick);

    ui.run()
}

/// Arms one single-shot timer for the next change of the displayed value.
fn arm_clock(island: &slint::Weak<Island>, tick: &Rc<slint::Timer>) {
    let island = island.clone();
    let tick_in_callback = Rc::downgrade(tick);
    tick.start(
        slint::TimerMode::SingleShot,
        clock::until_next_minute(),
        move || {
            let (Some(island), Some(tick)) = (island.upgrade(), tick_in_callback.upgrade()) else {
                return;
            };
            island.set_time(clock::current_time().into());
            arm_clock(&island.as_weak(), &tick);
        },
    );
}
