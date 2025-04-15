use wayland_client::protocol::{wl_buffer, wl_region, wl_surface};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;

/// 存储WaylandOverlay需要的表面信息
#[derive(Clone)]
pub struct SurfaceInfo {
    pub id: u32,
    pub width: i32,
    pub height: i32,
    pub name: Option<String>,
    pub scale_factor: i32,
}

/// Surface内部信息，包含Wayland对象
#[derive(Clone)]
pub struct RawSurfaceInfo {
    id: u32,
    surface: wl_surface::WlSurface,
    layer_surface: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    input_region: wl_region::WlRegion,
    buffer: Option<wl_buffer::WlBuffer>,
}
