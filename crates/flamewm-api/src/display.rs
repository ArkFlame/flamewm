use crate::{ModeId, OutputId, Rect, Size, TransactionId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayMode {
    pub id: ModeId,
    pub resolution: Size,
    pub refresh_millihz: i32,
    pub preferred: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputSnapshot {
    pub id: OutputId,
    pub connector: String,
    pub edid_identity: String,
    pub connected: bool,
    pub primary: bool,
    pub geometry: Rect,
    pub current_mode: ModeId,
    pub modes: Vec<DisplayMode>,
    pub shell_scale_percent: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingModeChange {
    pub transaction: TransactionId,
    pub output: OutputId,
    pub mode: ModeId,
    pub deadline_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplaySnapshot {
    pub generation: u64,
    pub outputs: Vec<OutputSnapshot>,
    pub pending: Option<PendingModeChange>,
}
