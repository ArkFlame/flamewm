#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAction {
    Lock,
    Logout,
    Suspend,
    Reboot,
    Shutdown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SessionCapabilities {
    pub lock: bool,
    pub logout: bool,
    pub suspend: bool,
    pub reboot: bool,
    pub shutdown: bool,
}
