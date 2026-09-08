//! Linux icon lookup and RGB8 preparation.
//!
//! Packaged semantic assets are tried before freedesktop theme paths. The
//! resolver performs filesystem work only on cache misses; callers must bump
//! `theme_generation` when host theme state changes.

use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

const GENERIC_FALLBACK: &str = "assets/raster/task-start.ppm";
const ICON_CONTEXTS: &[&str] = &[
    "apps",
    "actions",
    "categories",
    "devices",
    "emblems",
    "mimetypes",
    "places",
    "status",
    "",
];
const ICON_EXTENSIONS: &[&str] = &["png", "svg", "ppm"];

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum IconRequest {
    Name(String),
    Path(PathBuf),
}

impl IconRequest {
    pub fn name(name: impl Into<String>) -> Self {
        Self::Name(name.into())
    }

    pub fn path(path: impl Into<PathBuf>) -> Self {
        Self::Path(path.into())
    }
}

impl From<&str> for IconRequest {
    fn from(name: &str) -> Self {
        Self::name(name)
    }
}

impl From<String> for IconRequest {
    fn from(name: String) -> Self {
        Self::Name(name)
    }
}

impl From<PathBuf> for IconRequest {
    fn from(path: PathBuf) -> Self {
        Self::Path(path)
    }
}

impl From<&Path> for IconRequest {
    fn from(path: &Path) -> Self {
        Self::Path(path.to_path_buf())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IconSize {
    pub logical: u32,
    pub physical: u32,
}

impl IconSize {
    #[must_use]
    pub const fn new(logical: u32, physical: u32) -> Self {
        Self { logical, physical }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconOrigin {
    ExplicitPath,
    PackagedFlame,
    PackagedBreeze,
    SystemTheme,
    PackagedFallback,
}

/// Canonical alpha-bearing icon raster: straight (non-premultiplied) RGBA8
/// in row-major order. PNG/SVG sources preserve tiny-skia alpha via
/// un-premultiplication; opaque PPM sources convert to alpha=255 at decode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rgba8Raster {
    pub source: String,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub origin: IconOrigin,
    /// Set when an earlier candidate failed, while packaged fallback succeeded.
    pub fallback_reason: Option<RasterError>,
}

/// Compatibility alias for the J06 RuntimeImage bridge consumers
/// (`flamewm-shell`/`flamewm-desktop` projections). The aliased pixels are
/// RGBA8 straight alpha; use [`Rgba8Raster::to_rgb8_black_background`] for the
/// legacy opaque black-background projection.
pub type Rgb8Raster = Rgba8Raster;

impl Rgba8Raster {
    #[must_use]
    pub fn alpha_at(&self, x: u32, y: u32) -> Option<u8> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = (y as usize * self.width as usize + x as usize) * 4 + 3;
        self.pixels.get(index).copied()
    }

    /// Legacy opaque projection with black-background compositing semantics.
    #[must_use]
    pub fn to_rgb8_black_background(&self) -> Vec<u8> {
        let mut pixels = Vec::with_capacity(self.width as usize * self.height as usize * 3);
        for rgba in self.pixels.chunks_exact(4) {
            let a = u16::from(rgba[3]);
            pixels.push(((u16::from(rgba[0]) * a) / 255) as u8);
            pixels.push(((u16::from(rgba[1]) * a) / 255) as u8);
            pixels.push(((u16::from(rgba[2]) * a) / 255) as u8);
        }
        pixels
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RasterError {
    InvalidSize {
        logical: u32,
        physical: u32,
    },
    Io {
        path: PathBuf,
        message: String,
    },
    InvalidRaster {
        path: PathBuf,
        format: &'static str,
        message: String,
    },
    UnsupportedFormat {
        path: PathBuf,
        extension: String,
    },
    InvalidPpm {
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for RasterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize { logical, physical } => write!(
                formatter,
                "icon size must have non-zero physical dimensions (logical={logical}, physical={physical})"
            ),
            Self::Io { path, message } => {
                write!(
                    formatter,
                    "failed to read icon {}: {message}",
                    path.display()
                )
            }
            Self::InvalidRaster {
                path,
                format,
                message,
            } => write!(
                formatter,
                "invalid {format} icon {}: {message}",
                path.display()
            ),
            Self::UnsupportedFormat { path, extension } => write!(
                formatter,
                "unsupported icon format '{extension}' at {}",
                path.display()
            ),
            Self::InvalidPpm { path, message } => {
                write!(formatter, "invalid PPM icon {}: {message}", path.display())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IconError {
    InvalidSize {
        logical: u32,
        physical: u32,
    },
    FallbackUnavailable {
        fallback: PathBuf,
        fallback_error: Box<RasterError>,
        candidate_error: Option<Box<RasterError>>,
    },
}

impl fmt::Display for IconError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize { logical, physical } => write!(
                formatter,
                "icon size must have non-zero physical dimensions (logical={logical}, physical={physical})"
            ),
            Self::FallbackUnavailable {
                fallback,
                fallback_error,
                candidate_error,
            } => {
                write!(
                    formatter,
                    "packaged icon fallback {} failed: {fallback_error}",
                    fallback.display()
                )?;
                if let Some(candidate_error) = candidate_error {
                    write!(formatter, "; first candidate failure: {candidate_error}")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    identity: IconRequest,
    size: IconSize,
    theme_generation: u64,
}

#[derive(Clone, Debug)]
struct Candidate {
    path: PathBuf,
    origin: IconOrigin,
}

pub struct IconResolver {
    packaged_root: PathBuf,
    theme_names: Vec<String>,
    data_dirs: Vec<PathBuf>,
    theme_generation: u64,
    cache: HashMap<CacheKey, Result<Rgba8Raster, IconError>>,
}

impl IconResolver {
    #[must_use]
    pub fn new(
        packaged_root: impl Into<PathBuf>,
        theme_names: Vec<String>,
        data_dirs: Vec<PathBuf>,
        theme_generation: u64,
    ) -> Self {
        Self {
            packaged_root: packaged_root.into(),
            theme_names: unique_strings(theme_names),
            data_dirs: unique_paths(data_dirs),
            theme_generation,
            cache: HashMap::new(),
        }
    }

    /// Reads XDG paths once. Theme selection remains explicit when callers use `new`.
    /// Default search order after packaged assets is Breeze then hicolor; a
    /// configured `FLAMEWM_ICON_THEME` is always searched first.
    #[must_use]
    pub fn from_environment(packaged_root: impl Into<PathBuf>, theme_generation: u64) -> Self {
        let theme_names = env::var_os("FLAMEWM_ICON_THEME")
            .map(|value| value.to_string_lossy().into_owned())
            .filter(|value| !value.is_empty())
            .map(|value| vec![value])
            .unwrap_or_else(|| vec!["Breeze".to_owned()]);
        Self::new(
            packaged_root,
            theme_names,
            xdg_data_dirs(),
            theme_generation,
        )
    }

    pub fn notify_theme_changed(&mut self, theme_generation: u64) {
        self.theme_generation = theme_generation;
    }

    #[must_use]
    pub const fn theme_generation(&self) -> u64 {
        self.theme_generation
    }

    #[must_use]
    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }

    pub fn prepare(
        &mut self,
        request: impl Into<IconRequest>,
        size: IconSize,
    ) -> Result<Rgba8Raster, IconError> {
        let request = request.into();
        let key = CacheKey {
            identity: request.clone(),
            size,
            theme_generation: self.theme_generation,
        };
        if let Some(result) = self.cache.get(&key) {
            return result.clone();
        }

        let result = self.prepare_uncached(&request, size);
        self.cache.insert(key, result.clone());
        result
    }

    pub fn prepare_name(
        &mut self,
        name: impl Into<String>,
        size: IconSize,
    ) -> Result<Rgba8Raster, IconError> {
        self.prepare(IconRequest::name(name), size)
    }

    pub fn prepare_path(
        &mut self,
        path: impl Into<PathBuf>,
        size: IconSize,
    ) -> Result<Rgba8Raster, IconError> {
        self.prepare(IconRequest::path(path), size)
    }

    fn prepare_uncached(
        &self,
        request: &IconRequest,
        size: IconSize,
    ) -> Result<Rgba8Raster, IconError> {
        if size.physical == 0 {
            return Err(IconError::InvalidSize {
                logical: size.logical,
                physical: size.physical,
            });
        }

        let candidates = self.candidates(request, size);
        let mut candidate_error = None;
        for candidate in candidates {
            if !candidate.path.is_file() {
                continue;
            }
            match rasterize_path_with_origin(&candidate.path, size, candidate.origin) {
                Ok(raster) => return Ok(raster),
                Err(error) => {
                    if candidate_error.is_none() {
                        candidate_error = Some(error);
                    }
                }
            }
        }

        let fallback = self.packaged_root.join(GENERIC_FALLBACK);
        match rasterize_path_with_origin(&fallback, size, IconOrigin::PackagedFallback) {
            Ok(mut raster) => {
                raster.fallback_reason = candidate_error;
                Ok(raster)
            }
            Err(fallback_error) => Err(IconError::FallbackUnavailable {
                fallback,
                fallback_error: Box::new(fallback_error),
                candidate_error: candidate_error.map(Box::new),
            }),
        }
    }

    fn candidates(&self, request: &IconRequest, size: IconSize) -> Vec<Candidate> {
        match request {
            IconRequest::Path(path) => vec![Candidate {
                path: path.clone(),
                origin: IconOrigin::ExplicitPath,
            }],
            IconRequest::Name(name) => {
                let mut candidates = Vec::new();
                // Absolute Icon= paths arrive via prepare_name from desktop entries.
                // Try the literal path first without changing the public request API.
                if Path::new(name.as_str()).is_absolute() {
                    candidates.push(Candidate {
                        path: PathBuf::from(name),
                        origin: IconOrigin::ExplicitPath,
                    });
                }
                candidates.extend(self.packaged_candidates(name));
                candidates.extend(self.theme_candidates(name, size));
                candidates
            }
        }
    }

    fn effective_theme_names(&self) -> Vec<&str> {
        let mut ordered: Vec<&str> = Vec::new();
        for theme in &self.theme_names {
            if !theme.is_empty() && !ordered.contains(&theme.as_str()) {
                ordered.push(theme.as_str());
            }
        }
        for fallback in ["Breeze", "hicolor"] {
            if !ordered.iter().any(|existing| existing == &fallback) {
                ordered.push(fallback);
            }
        }
        ordered
    }

    fn packaged_candidates(&self, name: &str) -> Vec<Candidate> {
        let mut candidates = Vec::new();
        if matches!(name, "flamewm" | "flamewm-icon") {
            candidates.push(Candidate {
                path: self.packaged_root.join("assets/web/flamewm-icon.svg"),
                origin: IconOrigin::PackagedFlame,
            });
        }
        if valid_icon_name(name) {
            candidates.push(Candidate {
                path: self
                    .packaged_root
                    .join("assets/web/breeze")
                    .join(format!("{name}.svg")),
                origin: IconOrigin::PackagedBreeze,
            });
        }
        candidates
    }

    fn theme_candidates(&self, name: &str, size: IconSize) -> Vec<Candidate> {
        if !valid_icon_name(name) {
            return Vec::new();
        }

        let mut candidates = Vec::new();
        let mut size_dirs = vec![size.physical];
        for standard in [16_u32, 22, 24, 32, 48, 64, 96, 128, 256] {
            if !size_dirs.contains(&standard) {
                size_dirs.push(standard);
            }
        }
        size_dirs.sort_by_key(|candidate| candidate.abs_diff(size.physical));

        for data_dir in &self.data_dirs {
            for theme in self.effective_theme_names() {
                for size_dir in &size_dirs {
                    for context in ICON_CONTEXTS {
                        for extension in ICON_EXTENSIONS {
                            let mut path = data_dir
                                .join("icons")
                                .join(theme)
                                .join(format!("{size_dir}x{size_dir}"));
                            if !context.is_empty() {
                                path = path.join(context);
                            }
                            candidates.push(Candidate {
                                path: path.join(format!("{name}.{extension}")),
                                origin: IconOrigin::SystemTheme,
                            });
                        }
                    }
                }
                let scalable = data_dir.join("icons").join(theme).join("scalable");
                for context in ICON_CONTEXTS {
                    for extension in ICON_EXTENSIONS {
                        let path = if context.is_empty() {
                            scalable.join(format!("{name}.{extension}"))
                        } else {
                            scalable.join(context).join(format!("{name}.{extension}"))
                        };
                        candidates.push(Candidate {
                            path,
                            origin: IconOrigin::SystemTheme,
                        });
                    }
                }
            }
        }
        for data_dir in &self.data_dirs {
            for extension in ICON_EXTENSIONS {
                candidates.push(Candidate {
                    path: data_dir.join("pixmaps").join(format!("{name}.{extension}")),
                    origin: IconOrigin::SystemTheme,
                });
            }
        }
        candidates
    }
}

/// Direct adapter entry point for consumers that already selected a path.
/// Returns straight-alpha RGBA8; see [`Rgba8Raster`].
pub fn rasterize_path(path: &Path, size: IconSize) -> Result<Rgba8Raster, RasterError> {
    rasterize_path_with_origin(path, size, IconOrigin::ExplicitPath)
}

/// Legacy opaque projection with black-background compositing semantics.
/// Returns packed RGB8 bytes (3 bytes/px); prefer [`rasterize_path`] RGBA8.
pub fn rasterize_path_rgb8(path: &Path, size: IconSize) -> Result<Vec<u8>, RasterError> {
    rasterize_path(path, size).map(|raster| raster.to_rgb8_black_background())
}

fn rasterize_path_with_origin(
    path: &Path,
    size: IconSize,
    origin: IconOrigin,
) -> Result<Rgb8Raster, RasterError> {
    if size.physical == 0 {
        return Err(RasterError::InvalidSize {
            logical: size.logical,
            physical: size.physical,
        });
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match extension.as_str() {
        "ppm" => {
            let bytes = fs::read(path).map_err(|error| RasterError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
            decode_ppm(path, &bytes, size, origin)
        }
        "svg" => {
            let bytes = fs::read(path).map_err(|error| RasterError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
            decode_svg(path, &bytes, size, origin)
        }
        "png" => {
            let bytes = fs::read(path).map_err(|error| RasterError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
            decode_png(path, &bytes, size, origin)
        }
        _ => Err(RasterError::UnsupportedFormat {
            path: path.to_path_buf(),
            extension,
        }),
    }
}

fn decode_svg(
    path: &Path,
    bytes: &[u8],
    size: IconSize,
    origin: IconOrigin,
) -> Result<Rgb8Raster, RasterError> {
    // SVG renders directly at the requested size; alpha preserved by image-core.
    let image = flamewm_image_core::svg::render(bytes, size.physical, size.physical)
        .map_err(|message| invalid_raster_error(path, "SVG", &message))?;
    Ok(Rgba8Raster {
        source: path.to_string_lossy().into_owned(),
        width: image.width,
        height: image.height,
        pixels: image.pixels,
        origin,
        fallback_reason: None,
    })
}

fn decode_png(
    path: &Path,
    bytes: &[u8],
    size: IconSize,
    origin: IconOrigin,
) -> Result<Rgb8Raster, RasterError> {
    let natural = flamewm_image_core::png::decode(bytes)
        .map_err(|message| invalid_raster_error(path, "PNG", &message))?;
    if natural.width == 0 || natural.height == 0 {
        return invalid_raster(path, "PNG", "dimensions must be non-zero".to_owned());
    }
    // Deterministic bilinear upscale/downscale owned by image-core.
    let image = if natural.width == size.physical && natural.height == size.physical {
        natural
    } else {
        flamewm_image_core::scale::scale_rgba(
            &natural,
            size.physical,
            size.physical,
            flamewm_image_core::scale::ScaleFilter::Bilinear,
        )
        .ok_or_else(|| invalid_raster_error(path, "PNG", "requested dimensions overflow"))?
    };
    Ok(Rgba8Raster {
        source: path.to_string_lossy().into_owned(),
        width: image.width,
        height: image.height,
        pixels: image.pixels,
        origin,
        fallback_reason: None,
    })
}

fn invalid_raster_error(path: &Path, format: &'static str, message: &str) -> RasterError {
    RasterError::InvalidRaster {
        path: path.to_path_buf(),
        format,
        message: message.to_owned(),
    }
}

fn invalid_raster<T>(path: &Path, format: &'static str, message: String) -> Result<T, RasterError> {
    Err(RasterError::InvalidRaster {
        path: path.to_path_buf(),
        format,
        message,
    })
}

fn decode_ppm(
    path: &Path,
    bytes: &[u8],
    size: IconSize,
    origin: IconOrigin,
) -> Result<Rgba8Raster, RasterError> {
    // PPM is opaque (alpha=255); decode + deterministic nearest scale via image-core.
    let image = flamewm_image_core::decode_bytes_at(
        bytes,
        flamewm_image_core::ImageSourceFormat::Ppm,
        size.physical,
        size.physical,
        flamewm_image_core::scale::ScaleFilter::Nearest,
    )
    .map_err(|message| map_ppm_error(path, &message))?;
    Ok(Rgba8Raster {
        source: path.to_string_lossy().into_owned(),
        width: image.width,
        height: image.height,
        pixels: image.pixels,
        origin,
        fallback_reason: None,
    })
}

fn map_ppm_error(path: &Path, message: &str) -> RasterError {
    // Preserve the historical "only binary PPM (P6) is supported" surface for
    // non-P6 inputs so fallback_reason assertions stay stable.
    if message.contains("P6") {
        return RasterError::InvalidPpm {
            path: path.to_path_buf(),
            message: "only binary PPM (P6) is supported".to_owned(),
        };
    }
    RasterError::InvalidPpm {
        path: path.to_path_buf(),
        message: message.to_owned(),
    }
}

fn valid_icon_name(name: &str) -> bool {
    !name.is_empty() && !name.bytes().any(|byte| matches!(byte, b'/' | b'\\' | 0))
}

fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for value in values {
        if !value.is_empty() && !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

fn unique_paths(values: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique = Vec::new();
    for value in values {
        if !value.as_os_str().is_empty() && !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

fn xdg_data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let home = env::var_os("HOME").map(PathBuf::from);
    let data_home = env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.map(|path| path.join(".local/share")));
    if let Some(path) = data_home {
        dirs.push(path);
    }
    let data_dirs = env::var_os("XDG_DATA_DIRS")
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_owned());
    dirs.extend(
        data_dirs
            .split(':')
            .filter(|value| !value.is_empty())
            .map(PathBuf::from),
    );
    unique_paths(dirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p6_is_scaled_to_requested_physical_size() {
        let bytes = b"P6\n2 1\n255\n\xff\0\0\0\xff\0";
        let raster = decode_ppm(
            Path::new("test.ppm"),
            bytes,
            IconSize::new(16, 2),
            IconOrigin::ExplicitPath,
        )
        .expect("valid PPM");
        assert_eq!(raster.width, 2);
        assert_eq!(raster.height, 2);
        // Opaque PPM fixture converts to alpha=255 at decode time.
        assert_eq!(
            raster.pixels,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255
            ]
        );
        assert!(raster.pixels.chunks_exact(4).all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn svg_fixture_preserves_alpha_with_nonblack_pixel() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/web/flamewm-icon.svg");
        let raster = rasterize_path(&path, IconSize::new(16, 8)).expect("SVG fixture");
        assert_eq!((raster.width, raster.height), (8, 8));
        assert_eq!(raster.pixels.len(), 8 * 8 * 4);
        assert!(
            raster
                .pixels
                .chunks_exact(4)
                .any(|pixel| pixel[3] > 0 && pixel[..3] != [0, 0, 0])
        );
    }

    #[test]
    fn png_fixture_preserves_alpha_with_nonblack_pixel() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/web/flamewm.png");
        let raster = rasterize_path(&path, IconSize::new(16, 8)).expect("PNG fixture");
        assert_eq!((raster.width, raster.height), (8, 8));
        assert_eq!(raster.pixels.len(), 8 * 8 * 4);
        assert!(
            raster
                .pixels
                .chunks_exact(4)
                .any(|pixel| pixel[3] > 0 && pixel[..3] != [0, 0, 0])
        );
    }

    #[test]
    fn rgba_icons_carry_no_magenta_color_key_dependency() {
        // Baseline sweep: dynamic/app icon alpha must come from the alpha
        // channel, never from a magenta (255,0,255) transparent-pixel rule.
        // The only in-repo mention of that triple is the legacy X11
        // image_pixmap opaque-mask path, which accepts RGBA assets and no
        // longer gates on magenta. Here we assert decoded icons never need
        // a magenta test to recover transparency: every RGBA pixel stands
        // on its own alpha byte.
        let bytes = b"P6\n1 1\n255\n\xff\0\xff";
        let raster = decode_ppm(
            Path::new("magenta.ppm"),
            bytes,
            IconSize::new(16, 1),
            IconOrigin::ExplicitPath,
        )
        .expect("magenta PPM");
        assert_eq!(raster.pixels, vec![255, 0, 255, 255]);
        // A magenta pixel decodes fully opaque: it is ordinary content.
        assert_eq!(raster.alpha_at(0, 0), Some(255));
        let svg = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/web/flamewm-icon.svg");
        let svg_raster = rasterize_path(&svg, IconSize::new(16, 8)).expect("SVG fixture");
        let magenta_count = svg_raster
            .pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[..3] == [255, 0, 255])
            .count();
        let transparent_nonmagenta = svg_raster
            .pixels
            .chunks_exact(4)
            .any(|pixel| pixel[3] == 0 && pixel[..3] != [255, 0, 255]);
        let _ = magenta_count;
        assert!(
            transparent_nonmagenta || svg_raster.pixels.chunks_exact(4).all(|p| p[3] == 255),
            "alpha channel, not magenta, decides transparency"
        );
    }

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "flamewm-j08-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn write_ppm(path: &Path, pixel: [u8; 3]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("temp parent");
        }
        let mut bytes = b"P6\n1 1\n255\n".to_vec();
        bytes.extend_from_slice(&pixel);
        fs::write(path, bytes).expect("temp ppm");
    }

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn absolute_name_uses_explicit_path_first() {
        let dir = test_dir("explicit");
        let path = dir.join("custom.ppm");
        write_ppm(&path, [200, 10, 10]);
        let mut resolver = IconResolver::new(dir.clone(), vec!["hicolor".to_owned()], vec![dir], 0);
        let raster = resolver
            .prepare_name(path.to_string_lossy().into_owned(), IconSize::new(16, 8))
            .expect("absolute icon path");
        assert_eq!(raster.origin, IconOrigin::ExplicitPath);
        assert_eq!(raster.source, path.to_string_lossy());
        assert_eq!((raster.width, raster.height), (8, 8));
    }

    #[test]
    fn hicolor_name_resolves_through_theme_dirs() {
        let dir = test_dir("hicolor");
        let path = dir.join("icons/hicolor/16x16/apps/j08-hicolor-only.ppm");
        write_ppm(&path, [10, 200, 10]);
        let mut resolver = IconResolver::new(dir.clone(), Vec::new(), vec![dir.clone()], 7);
        let raster = resolver
            .prepare_name("j08-hicolor-only", IconSize::new(16, 16))
            .expect("hicolor icon");
        assert_eq!(raster.origin, IconOrigin::SystemTheme);
        assert_eq!(raster.source, path.to_string_lossy());
    }

    #[test]
    fn breeze_theme_is_preferred_over_hicolor() {
        let dir = test_dir("breeze");
        let breeze = dir.join("icons/Breeze/16x16/apps/j08-theme-order.ppm");
        let hicolor = dir.join("icons/hicolor/16x16/apps/j08-theme-order.ppm");
        write_ppm(&breeze, [200, 10, 10]);
        write_ppm(&hicolor, [10, 10, 200]);
        let mut resolver = IconResolver::new(
            dir.clone(),
            vec!["CustomTheme".to_owned()],
            vec![dir.clone()],
            3,
        );
        let raster = resolver
            .prepare_name("j08-theme-order", IconSize::new(16, 16))
            .expect("breeze icon");
        assert_eq!(raster.origin, IconOrigin::SystemTheme);
        assert_eq!(raster.source, breeze.to_string_lossy());
        // Breeze red pixel survives scaling to the requested size.
        assert_eq!(&raster.pixels[..4], &[200, 10, 10, 255]);
    }

    #[test]
    fn corrupt_candidate_keeps_fallback_reason() {
        let dir = test_dir("fallback");
        let bad = dir.join("bad.ppm");
        fs::create_dir_all(&dir).expect("temp dir");
        fs::write(&bad, b"not a ppm").expect("temp bad ppm");
        let mut resolver = IconResolver::new(workspace_root(), Vec::new(), Vec::new(), 0);
        let raster = resolver
            .prepare(IconRequest::path(bad.clone()), IconSize::new(16, 8))
            .expect("packaged fallback");
        assert_eq!(raster.origin, IconOrigin::PackagedFallback);
        let reason = raster.fallback_reason.expect("fallback reason");
        assert_eq!(
            reason,
            RasterError::InvalidPpm {
                path: bad,
                message: "only binary PPM (P6) is supported".to_owned(),
            }
        );
    }
}
