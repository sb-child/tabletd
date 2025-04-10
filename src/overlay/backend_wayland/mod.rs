use std::{fs::File, os::fd::AsFd};

use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum, delegate_noop,
    protocol::{
        wl_buffer, wl_compositor, wl_keyboard, wl_region, wl_registry, wl_seat, wl_shm,
        wl_shm_pool, wl_surface,
    },
};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1;
use wayland_server::protocol::wl_output;

pub fn test_overlay() {
    let conn = Connection::connect_to_env().unwrap();

    let mut event_queue = conn.new_event_queue();
    let qhandle = event_queue.handle();

    let display = conn.display();
    display.get_registry(&qhandle, ());

    let mut state = State {
        running: true,
        configured: false,
        base_surface: None,
        buffer: None,
        region: None,
        zwlr_layer_shell: None,
    };

    while state.running {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
}

struct State {
    running: bool,
    base_surface: Option<wl_surface::WlSurface>,
    buffer: Option<wl_buffer::WlBuffer>,
    region: Option<wl_region::WlRegion>,
    zwlr_layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    // wm_base: Option<xdg_wm_base::XdgWmBase>,
    // xdg_surface: Option<(xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel)>,
    configured: bool,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            println!("{}", interface);
            match &interface[..] {
                "wl_compositor" => {
                    println!("wl_compositor");
                    let compositor =
                        registry.bind::<wl_compositor::WlCompositor, _, _>(name, version, qh, ());

                    // let layer_shell = registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(
                    //     name,
                    //     version,
                    //     qh,
                    //     (),
                    // );

                    // let output = registry.bind::<wl_output::WlOutput, _, _>(name, version, qh, ());

                    let surface = compositor.create_surface(qh, ());
                    // let compositor =
                    //     registry.bind::<wl_compositor::WlCompositor, _, _>(name, 1, qh, ());
                    // let surface = compositor.create_surface(qh, ());

                    let region = compositor.create_region(qh, ());
                    region.add(0, 0, 1920, 1080);
                    surface.set_input_region(Some(&region));

                    // let layer_surface =
                    //     layer_shell.get_layer_surface(&surface, output, layer, namespace, qh, ());

                    state.base_surface = Some(surface);
                    state.region = Some(region);

                    // zwlr_layer_shell_v1::ZwlrLayerShellV1

                    // if state.wm_base.is_some() && state.xdg_surface.is_none() {
                    //     state.init_xdg_surface(qh);
                    // }
                }
                "wl_shm" => {
                    println!("wl_shm");
                    let shm = registry.bind::<wl_shm::WlShm, _, _>(name, version, qh, ());
                    let (init_w, init_h) = (320, 240);
                    let mut file = tempfile::tempfile().unwrap();
                    draw(&mut file, (init_w, init_h));
                    let pool = shm.create_pool(file.as_fd(), (init_w * init_h * 4) as i32, qh, ());

                    let buffer = pool.create_buffer(
                        0,
                        init_w as i32,
                        init_h as i32,
                        (init_w * 4) as i32,
                        wl_shm::Format::Argb8888,
                        qh,
                        (),
                    );

                    state.buffer = Some(buffer.clone());
                    if let Some(surface) = &state.base_surface {
                        surface.attach(Some(&buffer), 0, 0);
                        surface.commit();
                    }

                    // if state.configured {
                    //     let surface = state.base_surface.as_ref().unwrap();
                    //     surface.attach(Some(&buffer), 0, 0);
                    //     surface.commit();
                    // }
                }
                "wl_output" => {
                    println!("wl_output");
                    // registry.bind::<wl_seat::WlSeat, _, _>(name, 1, qh, ());
                }
                "zwlr_layer_shell_v1" => {
                    println!("zwlr_layer_shell_v1");
                    // registry.bind::<wl_seat::WlSeat, _, _>(name, 1, qh, ());
                }
                "wl_seat" => {
                    println!("wl_seat");
                    // registry.bind::<wl_seat::WlSeat, _, _>(name, 1, qh, ());
                }
                "xdg_wm_base" => {
                    // let wm_base = registry.bind::<xdg_wm_base::XdgWmBase, _, _>(name, 1, qh, ());
                    // state.wm_base = Some(wm_base);

                    // if state.base_surface.is_some() && state.xdg_surface.is_none() {
                    //     state.init_xdg_surface(qh);
                    // }
                }
                _ => {}
            }
        }
    }
}

delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_surface::WlSurface);
delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_buffer::WlBuffer);
delegate_noop!(State: ignore wl_region::WlRegion);
delegate_noop!(State: ignore zwlr_layer_shell_v1::ZwlrLayerShellV1);
// delegate_noop!(State: ignore wl_output::WlOutput);

fn draw(tmp: &mut File, (buf_x, buf_y): (u32, u32)) {
    use std::{cmp::min, io::Write};
    let mut buf = std::io::BufWriter::new(tmp);
    for y in 0..buf_y {
        for x in 0..buf_x {
            let a = 0xFF;
            let r = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let g = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let b = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);
            buf.write_all(&[b as u8, g as u8, r as u8, a as u8])
                .unwrap();
        }
    }
    buf.flush().unwrap();
}
