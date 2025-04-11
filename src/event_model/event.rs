#[derive(Debug, Clone, Copy)]
pub struct Tilt {
    pub x: i16,
    pub y: i16,
}

#[derive(Debug, Clone, Copy)]
pub enum PenLocation {
    Leaved,
    Floating,
    Pressed,
}

#[derive(Debug, Clone, Copy)]
pub enum ToolType {
    Pen,
    Eraser,
}

#[derive(Debug, Clone, Copy)]
pub struct PenButton {
    pub upper: bool,
    pub lower: bool,
}

#[derive(Debug, Clone)]
pub struct PenState {
    pub x: u32,
    pub y: u32,
    pub pressure: u32,
    pub tilt: Tilt,
    pub tool: ToolType,
    pub location: PenLocation,
}

#[derive(Debug, Clone)]
pub struct AuxButtonEvent {
    pub button_id: u8,
    pub pressed: bool,
}

#[derive(Debug, Clone)]
pub enum WheelDirection {
    Clockwise,
    CounterClockwise,
}

#[derive(Debug, Clone)]
pub enum TabletEvent {
    PenEvent(PenState),
    AuxButton(AuxButtonEvent),
    Wheel(WheelDirection),
    Unknown,
}

impl Default for TabletEvent {
    fn default() -> Self {
        Self::Unknown
    }
}
