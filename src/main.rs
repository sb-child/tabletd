use tabletd::overlay;

fn main() {
    println!("Hello, world!");

    overlay::backend_wayland::test_overlay();
}
