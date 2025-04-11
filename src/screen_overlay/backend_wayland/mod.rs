use std::{fs::File, os::fd::AsFd};

use wayland_client::{
    Connection, Dispatch, QueueHandle, delegate_noop,
    protocol::{
        wl_buffer, wl_compositor, wl_output, wl_region, wl_registry, wl_shm, wl_shm_pool,
        wl_surface,
    },
};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

pub fn test_overlay() {
    let conn = Connection::connect_to_env().unwrap();

    let mut event_queue = conn.new_event_queue();
    let qhandle = event_queue.handle();

    let display = conn.display();
    display.get_registry(&qhandle, ());

    let mut state = State {
        running: true,
        base_surface: None,
        buffer: None,
        input_region: None,
        output: None,
        layer_surface: None,
        _zwlr_layer_shell: None,
    };

    while state.running {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
}

struct State {
    running: bool,
    base_surface: Option<wl_surface::WlSurface>,
    buffer: Option<wl_buffer::WlBuffer>,
    input_region: Option<wl_region::WlRegion>,
    _zwlr_layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    output: Option<wl_output::WlOutput>,
    layer_surface: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match &interface[..] {
                "wl_compositor" => {
                    println!("wl_compositor");
                    let compositor = registry.bind::<wl_compositor::WlCompositor, _, _>(
                        name,
                        version,
                        qhandle,
                        (),
                    );

                    let surface = compositor.create_surface(qhandle, ());

                    let input_region = compositor.create_region(qhandle, ());
                    surface.set_input_region(Some(&input_region));

                    state.base_surface = Some(surface);
                    state.input_region = Some(input_region);
                }
                "wl_shm" => {
                    println!("wl_shm");
                    let shm = registry.bind::<wl_shm::WlShm, _, _>(name, version, qhandle, ());
                    let (init_w, init_h) = (320, 240);
                    let mut file = tempfile::tempfile().unwrap();
                    draw(&mut file, (init_w, init_h));
                    let pool =
                        shm.create_pool(file.as_fd(), (init_w * init_h * 4) as i32, qhandle, ());

                    let buffer = pool.create_buffer(
                        0,
                        init_w as i32,
                        init_h as i32,
                        (init_w * 4) as i32,
                        wl_shm::Format::Argb8888,
                        qhandle,
                        (),
                    );

                    state.buffer = Some(buffer.clone());
                    if let Some(surface) = &state.base_surface {
                        surface.attach(Some(&buffer), 0, 0);
                        surface.commit();
                    } else {
                        println!("state.base_surface is None")
                    }
                }
                "wl_output" => {
                    println!("wl_output");
                    let output =
                        registry.bind::<wl_output::WlOutput, _, _>(name, version, qhandle, ());
                    state.output = Some(output)
                }
                "zwlr_layer_shell_v1" => {
                    println!("zwlr_layer_shell_v1");
                    let layer_shell = registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(
                        name,
                        version,
                        qhandle,
                        (),
                    );

                    if let Some(surface) = &state.base_surface {
                        let layer_surface = layer_shell.get_layer_surface(
                            &surface,
                            state.output.as_ref(),
                            zwlr_layer_shell_v1::Layer::Overlay,
                            "tabletd overlay (wayland backend)".to_string(),
                            qhandle,
                            (),
                        );
                        layer_surface.set_size(320, 240);
                        layer_surface.set_anchor(
                            zwlr_layer_surface_v1::Anchor::Top
                                | zwlr_layer_surface_v1::Anchor::Left,
                        );
                        layer_surface.set_exclusive_zone(-1);
                        layer_surface.set_margin(0, 0, 0, 0);
                        layer_surface.set_keyboard_interactivity(
                            zwlr_layer_surface_v1::KeyboardInteractivity::None,
                        );
                        state.layer_surface = Some(layer_surface)
                    } else {
                        println!("state.base_surface is None")
                    }
                }
                "wl_seat" => {
                    println!("wl_seat");
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        layer_surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: <zwlr_layer_surface_v1::ZwlrLayerSurfaceV1 as wayland_client::Proxy>::Event,
        _: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width: _,
                height: _,
            } => {
                layer_surface.ack_configure(serial);
                if let Some(surface) = &state.base_surface {
                    surface.commit();
                }
            }
            zwlr_layer_surface_v1::Event::Closed => {}
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for State {
    fn event(
        state: &mut Self,
        output: &wl_output::WlOutput,
        event: <wl_output::WlOutput as wayland_client::Proxy>::Event,
        _: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        // TODO: 读取显示器信息
        // https://wayland.app/protocols/wayland#wl_output
        match event {
            wl_output::Event::Geometry {
                x,
                y,
                physical_width,
                physical_height,
                subpixel,
                make,
                model,
                transform,
            } => {}
            wl_output::Event::Mode {
                flags,
                width,
                height,
                refresh,
            } => {}
            wl_output::Event::Done => {}
            wl_output::Event::Scale { factor } => {}
            wl_output::Event::Name { name } => {}
            wl_output::Event::Description { description } => {}
            _ => {}
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
// delegate_noop!(State: ignore zwlr_layer_surface_v1::ZwlrLayerSurfaceV1);
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
