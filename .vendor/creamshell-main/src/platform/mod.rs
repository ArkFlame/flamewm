use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenWindow {
    pub id: String,
    pub app_name: String,
    pub title: String,
    pub icon_path: Option<PathBuf>,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Playback {
    pub app_icon: Option<PathBuf>,
    pub title: String,
    pub artist: String,
    pub status: String,
    pub art_url: Option<PathBuf>,
    pub position: String,
    pub length: String,
}
