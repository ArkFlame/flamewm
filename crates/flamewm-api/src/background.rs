#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallpaperFit {
    Fill,
    Fit,
    Stretch,
    Center,
    Tile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundState {
    pub wallpaper_path: String,
    pub fit: WallpaperFit,
    pub show_flamewm_mark: bool,
}
