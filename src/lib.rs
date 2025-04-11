/// 提供数位板事件对外分发的服务端接口
pub mod event_dispatcher;

/// 数位板驱动相关逻辑实现
pub mod tablet_driver;

/// HUD (Head-Up Display) 界面逻辑
pub mod hud_interface;

/// 屏幕叠加层接口，用于显示光标和 HUD
pub mod screen_overlay;

/// 原始输入接口实现（如 USB 和蓝牙设备）
pub mod input_devices;

/// 数位板事件的内部路由逻辑
pub mod event_router;

/// 数位板事件的抽象层，定义事件模型
pub mod event_model;
