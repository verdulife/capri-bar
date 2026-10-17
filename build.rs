// Compiles the Slint UI at build time, so there is no interpreter at runtime.

fn main() {
    slint_build::compile("ui/island.slint").expect("failed to compile ui/island.slint");
}
