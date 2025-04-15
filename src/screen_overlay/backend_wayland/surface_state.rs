use std::collections::HashMap;

use super::surface_info::{RawSurfaceInfo, SurfaceInfo};

pub struct SurfaceItem {
    
}

/// 内部状态对象，用于在异步任务内维护
pub struct SurfaceState {
    surfaces: HashMap<u32, SurfaceInfo>,
    current_surface_id: Option<u32>,
    raw_surfaces: HashMap<u32, RawSurfaceInfo>,
    available_surfaces: Vec<u32>,     // 可用的显示器ID列表
    used_surfaces: HashMap<u32, u32>, // 显示器ID到引用计数的映射
}

impl SurfaceState {
    /// 初始化状态
    pub fn new() -> Self {
        Self {
            surfaces: HashMap::new(),
            current_surface_id: None,
            raw_surfaces: HashMap::new(),
            available_surfaces: Vec::new(),
            used_surfaces: HashMap::new(),
        }
    }

    /// 添加新的surface
    pub fn add_surface(&mut self, id: u32, surface_info: SurfaceInfo, raw_info: RawSurfaceInfo) {
        self.surfaces.insert(id, surface_info);
        self.raw_surfaces.insert(id, raw_info);
        self.available_surfaces.push(id);

        // 如果这是第一个surface，设置为当前surface
        // if self.current_surface_id.is_none() {
        //     self.current_surface_id = Some(id);
        // }
    }
}
