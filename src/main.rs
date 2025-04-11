use tabletd::screen_overlay;

fn main() {
    println!("Hello, world!");

    screen_overlay::backend_wayland::test_overlay();
}
