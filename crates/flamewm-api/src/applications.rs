use crate::DesktopAppId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopApplication {
    pub id: DesktopAppId,
    pub name: String,
    pub generic_name: String,
    pub comment: String,
    pub startup_wm_class: String,
    pub argv: Vec<String>,
    pub keywords: Vec<String>,
    pub icon_name: String,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplicationLaunchOptions {
    pub extra_args: Vec<String>,
    pub uris: Vec<String>,
}
