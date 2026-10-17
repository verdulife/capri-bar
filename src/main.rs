// Capri Bar — the island.
//
// Integration 2: the island lives in a window this process creates and owns, so it is
// born outside the layout of a tiling window manager and the app no longer depends on
// Slint's winit backend. What the island *is* remains ui/island.slint, untouched.
//
// The island still draws and nothing else: no interaction, no widgets, no state.

mod island_platform;
mod island_window;

slint::include_modules!();

#[cfg(not(windows))]
compile_error!(
    "The island window is written against Win32 (src/island_window.rs). Linux is a destination, not today's target."
);

fn main() -> Result<(), slint::PlatformError> {
    // The platform creates the native window, so it has to be in place before the
    // component asks for one.
    slint::platform::set_platform(Box::new(island_platform::IslandPlatform::new())).map_err(
        |error| slint::PlatformError::Other(format!("could not install the island platform: {error:?}")),
    )?;

    let ui = Island::new()?;
    ui.run()
}
