pub mod backend_drm;
/// # Wayland overlay backend
///
/// `wayland` 后端, 基于 [`wlr layer shell`](https://wayland.app/protocols/wlr-layer-shell-unstable-v1) 实现
///
/// ## 兼容性
///
/// for short, 不支持 [`GNOME`](https://gitlab.gnome.org/GNOME/mutter/-/issues/973)
///
/// https://wayland.app/protocols/wlr-layer-shell-unstable-v1#compositor-support
pub mod backend_wayland;
pub mod backend_x11;
pub mod cursor;
pub mod hud;
