pub mod drm_util;
use drm::control::{Device, Mode, connector, crtc};
// use drm::Device;
use gbm::{BufferObjectFlags, Device as GbmDevice};

pub fn test_overlay() {
    // 打开 DRM 设备
    let device = drm_util::device::Card::open_global();

    // 创建一个新的 DRM 平面
    // let plane = device.create_plane().unwrap();

    // 设置平面的格式和大小
    // plane.set_format(drm::Format::Argb8888).unwrap();
    // plane.set_size(1024, 768).unwrap();

    drm_util::capability::enable_client_cap(&device);
    drm_util::capability::get_driver_cap(&device);

    println!("enable_client_cap done");

    // let cursor_surface = gbm_device
    //     .create_surface::<()>(256, 256, gbm::Format::Argb8888, BufferObjectFlags::CURSOR)
    //     .unwrap();

    fn test_display<T: drm::Device + drm::control::Device>(
        device: &T,
        crtc_handle: crtc::Handle,
        conn: connector::Handle,
        mode: Option<Mode>,
    ) {
        let gbm_device = GbmDevice::new(device).unwrap();

        println!("gbm backend {}", gbm_device.backend_name());

        let supported = gbm_device.is_format_supported(
            gbm::Format::Argb8888,
            BufferObjectFlags::SCANOUT | BufferObjectFlags::WRITE,
        );
        println!("{}", supported);

        // let cursor_surface = gbm_device
        //     .create_surface::<()>(512, 512, gbm::Format::Argb8888, BufferObjectFlags::SCANOUT)
        //     .unwrap();

        let mut bo = gbm_device
            .create_buffer_object::<()>(
                512,
                512,
                gbm::Format::Argb8888,
                BufferObjectFlags::SCANOUT | BufferObjectFlags::WRITE, // BufferObjectFlags::SCANOUT | BufferObjectFlags::WRITE,
            )
            .unwrap();

        let buffer = {
            let mut buffer = Vec::new();
            for i in 0..512 {
                for _ in 0..512 {
                    buffer.push(if i % 2 == 0 { 0 } else { 255 });
                }
            }
            buffer
        };

        bo.write(&buffer).unwrap();

        let fb = gbm_device.add_framebuffer(&bo, 32, 32).unwrap();

        gbm_device
            .set_crtc(crtc_handle, Some(fb), (0, 0), &[conn], mode)
            .unwrap();
    }

    // ---

    let resources = device.resource_handles().unwrap();

    for connector_handle in resources.connectors() {
        let connector = device.get_connector(*connector_handle, false).unwrap();
        if connector.state() != drm::control::connector::State::Connected {
            continue;
        }
        println!("connector handle {connector_handle:?}");
        println!("connector info {connector:?}");
        if let Some(encoder_handle) = connector.current_encoder() {
            let encoder = device.get_encoder(encoder_handle).unwrap();
            println!("encoder info {encoder:?}");
            if let Some(crtc_handle) = encoder.crtc() {
                let crtc = device.get_crtc(crtc_handle).unwrap();
                println!("crtc info {crtc:?}");
                test_display(&device, crtc_handle, *connector_handle, crtc.mode());
                println!("");
                for plane_handle in device.plane_handles().unwrap() {
                    let plane = device.get_plane(plane_handle).unwrap();
                    if plane.crtc() != Some(crtc_handle) {
                        continue;
                    }
                    println!("plane info {plane:?}");
                    let fb_handle = plane.framebuffer().unwrap();
                    let fb = device.get_planar_framebuffer(fb_handle).unwrap();
                    println!("fb info {fb:?}");
                    // display
                    println!("");
                }
            }
        }
    }

    // let surface = gbm_device
    //     .create_surface(1024, 768, gbm::Format::Abgr8888, BufferObjectFlags::CURSOR)
    //     .unwrap();

    // // 将 GBM 表面附加到 DRM 平面
    // plane.attach_surface(&surface).unwrap();

    // // 渲染内容到平面上
    // //...

    // // 提交平面
    // plane.commit().unwrap();
}
