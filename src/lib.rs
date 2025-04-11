/// 数位板事件服务端(供外部程序使用)
pub mod dispatch;
/// 数位板驱动
pub mod driver;
/// HUD(head-up display) 界面
pub mod hud;
/// 屏幕叠加层接口(显示 `光标` 和 `HUD`)
pub mod overlay;
/// 原始输入接口(如 `USB` 和 `蓝牙`)
pub mod raw;
/// 数位板事件内部分发器
pub mod router;
/// 数位板事件抽象层
pub mod statement;
