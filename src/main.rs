// Capri Bar — integration 1: draw the island and nothing else.
//
// No interaction, no widgets, no state. This binary needs a real desktop
// session; on the headless development machine the island is rendered with
// `slint-viewer --screenshot` instead.

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = Island::new()?;
    ui.run()
}
