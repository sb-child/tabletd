pub mod surface_info;
use std::{
    collections::HashMap,
    fs::File,
    os::fd::AsFd,
    sync::{Arc, Mutex},
};

use tokio::sync::{mpsc, oneshot};
use wayland_client::{
    Connection, Dispatch, QueueHandle, delegate_noop,
    protocol::{
        wl_buffer, wl_compositor, wl_output, wl_region, wl_registry, wl_shm, wl_shm_pool,
        wl_surface,
    },
};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

mod surface_state;

use surface_state::SurfaceState;

#[derive(Debug)]
pub struct DisplayInfo {
    width: u32,
    height: u32,
    scale_factor: i32,
    name: String,
}

enum DisplayCommand {
    GetDmaBuffer(oneshot::Sender<()>),
    GetInfo(oneshot::Sender<DisplayInfo>),
}

pub struct Display {
    channel: mpsc::Sender<DisplayCommand>,
}

impl Display {
    pub async fn get_dma_buffer(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (tx, rx) = oneshot::channel();
        self.channel.send(DisplayCommand::GetDmaBuffer(tx)).await?;
        Ok(rx.await?)
    }

    pub async fn get_info(&self) -> Result<DisplayInfo, Box<dyn std::error::Error>> {
        let (tx, rx) = oneshot::channel();
        self.channel.send(DisplayCommand::GetInfo(tx)).await?;
        Ok(rx.await?)
    }
}

/// WaylandOverlay层支持的命令
enum OverlayCommand {
    GetNextDisplay(oneshot::Sender<Option<SurfaceInfo>>),
    GetCurrentDisplay(oneshot::Sender<Option<SurfaceInfo>>),
    ReleaseDisplay(u32),
}

/// WaylandOverlay 代表在Wayland下实现的屏幕叠加层
/// 用于显示光标和HUD界面
pub struct WaylandOverlay {
    command_tx: mpsc::Sender<OverlayCommand>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl WaylandOverlay {
    /// 创建一个新的WaylandOverlay实例
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::channel(32);

        // 启动后台任务来处理Wayland事件
        let task_handle = tokio::spawn(async move {
            let state = Arc::new(Mutex::new(SurfaceState::new()));

            // 创建一个tokio通道用于启动创建displays的任务
            let (create_tx, mut create_rx) = mpsc::channel::<()>(1);
            let state_clone = Arc::clone(&state);

            // 发送初始信号以创建displays
            let _ = create_tx.send(()).await;

            // 创建任务来处理Wayland连接
            let wayland_task = tokio::task::spawn_blocking(move || {
                if let Some(()) = create_rx.blocking_recv() {
                    // 在阻塞线程中执行Wayland连接和事件处理
                    if let Ok(conn) = Connection::connect_to_env() {
                        let mut event_queue = conn.new_event_queue();
                        let qhandle = event_queue.handle();

                        // 获取显示
                        let display = conn.display();
                        display.get_registry(&qhandle, ());

                        // 创建state
                        let mut wayland_state = WaylandEventState {
                            running: true,
                            compositor: None,
                            shm: None,
                            layer_shell: None,
                            outputs: HashMap::new(),
                            surfaces: HashMap::new(),
                            registry_done: false,
                        };

                        // 第一步：获取所有接口和显示器
                        println!("获取Wayland接口和显示器信息...");
                        while !wayland_state.registry_done
                            || wayland_state.outputs.is_empty()
                            || !wayland_state.all_outputs_have_size()
                        {
                            if let Err(e) = event_queue.blocking_dispatch(&mut wayland_state) {
                                println!("Wayland事件处理错误: {:?}", e);
                                break;
                            }
                        }

                        // 第二步：为每个显示器创建overlay
                        println!("为{}个显示器创建overlay", wayland_state.outputs.len());
                        for (id, output_info) in &wayland_state.outputs {
                            // 跳过尺寸为0x0的显示器
                            if !output_info.has_valid_size {
                                println!("跳过尺寸无效的显示器 #{}", id);
                                continue;
                            }

                            println!("为显示器 {} 创建overlay", id);

                            if let (Some(ref compositor), Some(ref layer_shell)) = (
                                wayland_state.compositor.as_ref(),
                                wayland_state.layer_shell.as_ref(),
                            ) {
                                // 创建基础surface
                                let surface = compositor.create_surface(&qhandle, ());

                                // 创建输入区域（使overlay不捕获输入）
                                let input_region = compositor.create_region(&qhandle, ());
                                surface.set_input_region(Some(&input_region));

                                // 创建layer_surface
                                let layer_surface = layer_shell.get_layer_surface(
                                    &surface,
                                    Some(&output_info.output),
                                    zwlr_layer_shell_v1::Layer::Overlay,
                                    format!("tabletd overlay"),
                                    &qhandle,
                                    (),
                                );

                                // 使用显示器实际尺寸
                                let width = output_info.width.unwrap();
                                let height = output_info.height.unwrap();

                                // 配置layer_surface
                                layer_surface.set_size(width as u32, height as u32);
                                layer_surface.set_anchor(
                                    zwlr_layer_surface_v1::Anchor::Top
                                        | zwlr_layer_surface_v1::Anchor::Left
                                        | zwlr_layer_surface_v1::Anchor::Right
                                        | zwlr_layer_surface_v1::Anchor::Bottom,
                                );
                                layer_surface.set_exclusive_zone(-1);
                                layer_surface.set_margin(0, 0, 0, 0);
                                layer_surface.set_keyboard_interactivity(
                                    zwlr_layer_surface_v1::KeyboardInteractivity::None,
                                );

                                // 初始化提交surface
                                surface.commit();

                                // 保存surface信息
                                println!("保存surface #{}信息", *id);
                                wayland_state.surfaces.insert(
                                    *id,
                                    RawSurfaceInfo {
                                        id: *id,
                                        surface,
                                        layer_surface,
                                        input_region,
                                        buffer: None,
                                    },
                                );

                                // 更新共享状态
                                if let Ok(mut state) = state_clone.lock() {
                                    // state
                                    //     .raw_surfaces
                                    //     .insert(*id, wayland_state.surfaces[id].clone());

                                    // 同时更新用于公开API的表面信息
                                    // state.surfaces.insert(
                                    //     *id,
                                    //     SurfaceInfo {
                                    //         id: *id,
                                    //         width,
                                    //         height,
                                    //         name: output_info.name.clone(),
                                    //         scale_factor: output_info.scale_factor,
                                    //     },
                                    // );

                                    // 如果这是第一个surface，设置为当前surface
                                    // if state.current_surface_id.is_none() {
                                    //     state.current_surface_id = Some(*id);
                                    // }
                                    state.add_surface(
                                        *id,
                                        SurfaceInfo {
                                            id: *id,
                                            width,
                                            height,
                                            name: output_info.name.clone(),
                                            scale_factor: output_info.scale_factor,
                                        },
                                        wayland_state.surfaces[id].clone(),
                                    );
                                }
                            }
                        }

                        // 确保至少有一个surface被创建
                        if wayland_state.surfaces.is_empty() {
                            println!("没有创建任何surface，请检查显示器配置");
                            return;
                        }

                        // 进入主事件循环
                        println!("进入事件循环...等待configure事件");
                        while wayland_state.running {
                            if let Err(e) = event_queue.blocking_dispatch(&mut wayland_state) {
                                println!("Wayland事件循环错误: {:?}", e);
                                break;
                            }

                            // 给其他任务机会处理
                            // std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                    }
                }
            });

            // 处理overlay命令
            let mut command_rx = command_rx;
            while let Some(cmd) = command_rx.recv().await {
                match cmd {
                    OverlayCommand::GetNextDisplay(resp) => {
                        let next_surface = {
                            let mut state = state.lock().unwrap();

                            // 检查是否有可用的显示器
                            if state.available_surfaces.is_empty() {
                                // 如果没有可用显示器，但有被使用的显示器
                                // 我们等待可用显示器的出现
                                None
                            } else {
                                // 获取下一个可用的显示器ID
                                let next_id = state.available_surfaces.remove(0);

                                // 增加引用计数或添加到使用中映射
                                *state.used_surfaces.entry(next_id).or_insert(0) += 1;

                                // 更新当前显示器ID
                                state.current_surface_id = Some(next_id);

                                // 返回该显示器的信息
                                state.surfaces.get(&next_id).cloned()
                            }
                        };

                        // 发送响应
                        let _ = resp.send(next_surface);
                    }
                    OverlayCommand::GetCurrentDisplay(resp) => {
                        let current = {
                            let state = state.lock().unwrap();
                            if let Some(id) = state.current_surface_id {
                                state.surfaces.get(&id).cloned()
                            } else {
                                None
                            }
                        };

                        let _ = resp.send(current);
                    }
                    OverlayCommand::ReleaseDisplay(id) => {
                        let mut state = state.lock().unwrap();

                        // 减少引用计数
                        if let Some(count) = state.used_surfaces.get_mut(&id) {
                            *count -= 1;

                            // 如果引用计数为0，则将其添加回可用列表
                            if *count == 0 {
                                state.used_surfaces.remove(&id);
                                state.available_surfaces.push(id);
                                println!("显示器 #{} 已释放，现在可用", id);
                            }
                        }
                    }
                }
            }

            // 命令通道关闭，取消Wayland任务
            wayland_task.abort();
        });

        Self {
            command_tx,
            task_handle: Some(task_handle),
        }
    }

    /// 获取下一个显示器
    pub async fn next_display(&self) -> Result<Display, Box<dyn std::error::Error>> {
        let (tx, rx) = oneshot::channel();

        // 发送获取下一个显示器的请求
        self.command_tx
            .send(OverlayCommand::GetNextDisplay(tx))
            .await?;

        // 等待响应
        // 如果当前没有可用显示器，这将阻塞直到有显示器可用
        let surface = rx.await?;

        // 如果没有获取到显示器信息，返回错误
        let surf = surface.ok_or_else(|| unreachable!())?;

        // 创建用于返回的Display实例
        let (channel_tx, mut channel_rx) = mpsc::channel(10);
        let display = Display {
            channel: channel_tx.clone(),
        };

        // 设置监听和处理逻辑
        let display_id = surf.id;
        let tx_clone = self.command_tx.clone();

        // 创建一个协程来处理该Display的请求和生命周期
        tokio::spawn(async move {
            // 保存显示器信息用于后续请求
            let surf_info = surf.clone();

            // 处理用户通过Display发送的命令
            while let Some(cmd) = channel_rx.recv().await {
                match cmd {
                    DisplayCommand::GetInfo(resp) => {
                        let info = DisplayInfo {
                            width: surf_info.width as u32,
                            height: surf_info.height as u32,
                            scale_factor: surf_info.scale_factor,
                            name: surf_info.name.clone().unwrap_or_else(|| "未知".to_string()),
                        };
                        let _ = resp.send(info);
                    }
                    DisplayCommand::GetDmaBuffer(resp) => {
                        // 目前简单返回空结果
                        let _ = resp.send(());
                    }
                }
            }

            // 当channel_rx被关闭时(即Display被丢弃)，发送释放消息
            let _ = tx_clone
                .send(OverlayCommand::ReleaseDisplay(display_id))
                .await;
        });

        // 返回新创建的Display实例
        Ok(display)
    }

    // 获取当前显示器
    // pub async fn current_display(&self) -> Option<SurfaceInfo> {
    //     let (tx, rx) = oneshot::channel();
    //     if let Err(_) = self
    //         .command_tx
    //         .send(OverlayCommand::GetCurrentDisplay(tx))
    //         .await
    //     {
    //         return None;
    //     }

    //     match rx.await {
    //         Ok(surface) => surface,
    //         Err(_) => None,
    //     }
    // }
}

impl Drop for WaylandOverlay {
    fn drop(&mut self) {
        // 取消后台任务
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

// Wayland事件状态
struct WaylandEventState {
    running: bool,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    outputs: HashMap<u32, OutputInfo>,
    surfaces: HashMap<u32, RawSurfaceInfo>,
    registry_done: bool,
}

/// 显示器信息
struct OutputInfo {
    output: wl_output::WlOutput,
    width: Option<i32>,
    height: Option<i32>,
    name: Option<String>,
    scale_factor: i32,
    has_valid_size: bool,
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandEventState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => match &interface[..] {
                "wl_compositor" => {
                    println!("找到wl_compositor");
                    let compositor = registry.bind::<wl_compositor::WlCompositor, _, _>(
                        name,
                        version,
                        qhandle,
                        (),
                    );
                    state.compositor = Some(compositor);
                }
                "wl_shm" => {
                    println!("找到wl_shm");
                    let shm = registry.bind::<wl_shm::WlShm, _, _>(name, version, qhandle, ());
                    state.shm = Some(shm);
                }
                "wl_output" => {
                    println!("找到wl_output #{}", name);
                    let output =
                        registry.bind::<wl_output::WlOutput, _, _>(name, version, qhandle, ());
                    state.outputs.insert(
                        name,
                        OutputInfo {
                            output,
                            width: None,
                            height: None,
                            name: None,
                            scale_factor: 1,
                            has_valid_size: false,
                        },
                    );
                }
                "zwlr_layer_shell_v1" => {
                    println!("找到zwlr_layer_shell_v1");
                    let layer_shell = registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(
                        name,
                        version,
                        qhandle,
                        (),
                    );
                    state.layer_shell = Some(layer_shell);
                }
                _ => {}
            },
            wl_registry::Event::GlobalRemove { name } => {
                if state.outputs.remove(&name).is_some() {
                    println!("显示器 #{} 已移除", name);
                }
                if state.surfaces.remove(&name).is_some() {
                    println!("Surface #{} 已移除", name);
                }
            }
            _ => {}
        }

        // 在获取到基本接口后，标记注册完成
        if state.compositor.is_some() && state.shm.is_some() && state.layer_shell.is_some() {
            state.registry_done = true;
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for WaylandEventState {
    fn event(
        state: &mut Self,
        _output: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // 找到对应的输出设备
        let mut output_id = None;
        for (id, info) in &state.outputs {
            if &info.output == _output {
                output_id = Some(*id);
                break;
            }
        }

        if let Some(id) = output_id {
            if let Some(info) = state.outputs.get_mut(&id) {
                match event {
                    wl_output::Event::Mode { width, height, .. } => {
                        println!("显示器分辨率: {}x{}", width, height);
                        info.width = Some(width);
                        info.height = Some(height);
                        if width > 0 && height > 0 {
                            info.has_valid_size = true;
                            println!("显示器 #{} 已获取到有效尺寸: {}x{}", id, width, height);
                        }
                    }
                    wl_output::Event::Scale { factor } => {
                        println!("显示器缩放因子: {}", factor);
                        info.scale_factor = factor;
                    }
                    wl_output::Event::Name { name } => {
                        println!("显示器名称: {}", name);
                        info.name = Some(name);
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for WaylandEventState {
    fn event(
        state: &mut Self,
        layer_surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                println!(
                    "Layer surface配置: {}x{} (serial: {})",
                    width, height, serial
                );

                // 确认配置
                layer_surface.ack_configure(serial);

                // 查找对应的surface
                for surf_info in state.surfaces.values_mut() {
                    if &surf_info.layer_surface == layer_surface {
                        // 创建缓冲区
                        if width > 0 && height > 0 && state.shm.is_some() {
                            println!("创建{}x{}的缓冲区", width, height);
                            // 创建并绘制缓冲区
                            if let Ok(mut file) = tempfile::tempfile() {
                                draw(&mut file, (width as u32, height as u32));

                                let pool = state.shm.as_ref().unwrap().create_pool(
                                    file.as_fd(),
                                    (width * height * 4) as i32,
                                    qhandle,
                                    (),
                                );

                                let buffer = pool.create_buffer(
                                    0,
                                    width as i32,
                                    height as i32,
                                    (width * 4) as i32,
                                    wl_shm::Format::Argb8888,
                                    qhandle,
                                    (),
                                );

                                println!("附加缓冲区到surface");
                                // 附加缓冲区并提交
                                surf_info.surface.attach(Some(&buffer), 0, 0);
                                surf_info.surface.damage(0, 0, width as i32, height as i32);
                                surf_info.buffer = Some(buffer);
                            }
                        }

                        println!("提交surface");
                        // 提交surface应用更改
                        surf_info.surface.commit();
                        break;
                    }
                }
            }
            zwlr_layer_surface_v1::Event::Closed => {
                println!("Layer surface closed");

                // 查找并移除对应的surface
                let mut id_to_remove = None;
                for (id, surf_info) in &state.surfaces {
                    if &surf_info.layer_surface == layer_surface {
                        id_to_remove = Some(*id);
                        break;
                    }
                }

                if let Some(id) = id_to_remove {
                    state.surfaces.remove(&id);
                    println!("移除surface #{}", id);
                }

                // 如果所有surface都关闭了，退出
                if state.surfaces.is_empty() {
                    println!("所有surface已关闭，退出事件循环");
                    state.running = false;
                }
            }
            _ => {}
        }
    }
}

// 空分发实现
delegate_noop!(WaylandEventState: ignore wl_compositor::WlCompositor);
delegate_noop!(WaylandEventState: ignore wl_surface::WlSurface);
delegate_noop!(WaylandEventState: ignore wl_shm::WlShm);
delegate_noop!(WaylandEventState: ignore wl_shm_pool::WlShmPool);
delegate_noop!(WaylandEventState: ignore wl_buffer::WlBuffer);
delegate_noop!(WaylandEventState: ignore wl_region::WlRegion);
delegate_noop!(WaylandEventState: ignore zwlr_layer_shell_v1::ZwlrLayerShellV1);

/// 测试Wayland overlay的实现
/// 创建一个简单的彩色矩形，显示在屏幕左上角
pub async fn test_overlay() -> Result<(), Box<dyn std::error::Error>> {
    let overlay = WaylandOverlay::new();

    // 等待一段时间，让overlay有时间设置
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // 尝试获取所有显示器
    // if let Some(display) = overlay.current_display().await {
    //     println!(
    //         "当前显示器: {}, 分辨率: {}x{}",
    //         display.name.unwrap_or_else(|| "未知".to_string()),
    //         display.width,
    //         display.height
    //     );
    // } else {
    //     println!("未找到可用显示器");
    // }
    loop {
        let display = overlay.next_display().await?;
        let display_info = display.get_info().await?;
        println!("new display {display_info:?}");
    }
}

/// 在临时文件中绘制示例图像
fn draw(tmp: &mut File, (buf_x, buf_y): (u32, u32)) {
    use std::{cmp::min, io::Write};
    let mut buf = std::io::BufWriter::new(tmp);
    println!("绘制{}x{}的缓冲区", buf_x, buf_y);
    for y in 0..buf_y {
        for x in 0..buf_x {
            // 设置半透明背景
            let a = 0x80; // 50%透明度
            let r = 0x00;
            let g = 0x80;
            let b = 0xFF;
            buf.write_all(&[b as u8, g as u8, r as u8, a as u8])
                .unwrap();
        }
    }
    buf.flush().unwrap();
    println!("缓冲区绘制完成");
}

impl WaylandEventState {
    /// 检查是否所有显示器都已获取到有效尺寸
    fn all_outputs_have_size(&self) -> bool {
        // 如果没有显示器，返回false
        if self.outputs.is_empty() {
            println!("没有检测到显示器");
            return false;
        }

        // 检查是否至少有一个显示器有有效尺寸
        let mut has_any_valid = false;
        for info in self.outputs.values() {
            if info.has_valid_size {
                has_any_valid = true;
                break;
            }
        }

        // 如果至少有一个显示器有有效尺寸，就可以继续
        if !has_any_valid {
            println!("等待至少一个显示器获取有效尺寸...");
            return false;
        }

        println!("至少一个显示器已准备好");
        return true;
    }
}

/* 将来需要实现的功能:
 * 1. 支持多显示器 - 为每个显示器创建独立的overlay
 * 2. 动态光标 - 根据笔的状态(悬空、压力、倾斜等)做出变化
 * 3. 支持不同的缩放比例
 * 4. 与hud_interface模块集成，提供界面渲染接口
 * 5. 支持多数位板，每个光标可以使用不同颜色标注
 */
