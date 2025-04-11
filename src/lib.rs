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

// `screen_overlay`要做的事情就是给每个显示器都创建一个全屏overlay
// 然后通过DMA或者什么东西暴露出接口，由`hud_interface`渲染每个overlay的界面
// 至于光标要不要单独整一个overlay.. 如果移动它的效率很高，而且开销比重新渲染更低，那可以考虑这样

// 至于光标的设计嘛，我在想一种动态的光标：笔悬在空中时，它显示为一个空心圆形，
// 支持倾斜感应的话，这个圆形可以相应的倾斜为椭圆，倾斜角对应圆形的变换程度，用深浅两个颜色代表具体的倾斜方向
// 笔按下之后，椭圆立刻过渡成实心圆形, 它的半径取决于按压力度，实心圆形内部绘制一个扇形图案代表倾斜角度和方向

// 需要能够处理接入多个数位板的情况，每个光标可以使用不同颜色标注，光标旁可以显示文字
// 当然还有不同屏幕，甚至有人喜欢给不同的屏幕设置不同的缩放比例
// HACK: wayland协议并不支持分数缩放，看样子这是混成器自己搞的奇怪东西，把buffer放大又缩小

// HUD `hud_interface` 用来显示提示信息(我挺喜欢 osu!lazer 那个风格), 比如数位板接入，拔出等事件
// 当然，三星的 s pen 也可以抄抄，比如按下笔上的按钮之后弹出快捷菜单
// 数位板上通常会有button和wheel, 它们可以用来唤起 HUD
// HUD 应该且只能由数位板操控，所以这就是为什么我还create了`event_router`

// `event_router`作为`event_model`到`event_dispatcher`的桥梁，但是`event_router`可以在特定情况下阻止事件向后传递(比如唤起了HUD)
// 但是为了最大限度的互操作性和扩展性, `event_router` 并不会真的把事件拦在路上，它会给由内部处理的事件加个tag，代表程序不应该响应它
// 至于为什么要这样设计，那是因为我还打算整个自己的 `tabletd API`

// `tabletd API` 是个极其变态的东西，它允许 `tabletd` 作为服务端向 `tabletd` 客户端转发数位板事件，又名「远程数位板」，
// 而且它可以走各种socket，只要保证对面能连上就行
// `tabletd API` 会发送所有的事件，除非用户设置了过滤条件

// `event_dispatcher` 是数位板事件的出口，它一般会和 `wayland`, `libinput`, `tabletd API` 等接口对接，
// 当然，被拦截的事件只会通过 `tabletd API` 发出去，不然 HUD 就像一个透明窗口，事件全都流出给下层窗口了

// `event_model` 是一个极其抽象的数位板，我希望它能覆盖到市面上所有不带屏的数位板的全部功能，因为数位屏不是数位板，万一你带触摸呢

// `input_devices` 包括了连接到数位板的各种方式，比如USB和蓝牙，和`tabletd API`的各种传输途径(http, tcp, udp(?), unix socket 甚至 `iroh` 等等)，
// 当然，最常见的蓝牙其实传输的是绝对鼠标，看样子也发来了压感，按键等数据
// 但是我需要完全接管这个设备，不能让它被 bluetoothctl(bluez) 之类的系统服务把它变成 /dev 下的input设备(这样会极其麻烦)

// `tablet_driver` 这就是整个项目最容易的部分，数位板驱动
// 当然也包括 `tabletd API`
// 这部分可以照抄 `opentabletdriver`
// 妈的我挖坑`tabletd`就是因为`opentabletdriver`进展巨慢，gui和gui库一起爆炸了，有几率使gnome崩溃，而且不合我的pr

// TODO: 数位板 -> 屏幕的映射
// 我真的要让映射可以跨越屏幕嘛？那HUD该显示在哪个屏上？光标呢？如果多个屏幕有不同的缩放比例呢？
// 然后我需要给每个数位板赋予一个单独的ID
// HACK: 怎么让设备的USB和蓝牙都指向同一个设备ID

// 杂项
// HACK: 如果`tabletd`的运行环境下得不到`WAYLAND_DISPLAY`之类的环境变量，
// 那它应该去暴力查找所有可能的 wayland socket，结果发现系统下启动了n个不同的混成器，它该怎么办

// TODO: GUI
// 就像 `opentabletdriver`，我需要想想GUI是浏览器里打开合适还是 native app 合适
