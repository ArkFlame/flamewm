//! Global font registry: register font files, then resolve a CSS-style
//! family stack (`"Inter, sans-serif"`) to a loaded face.

use fontdue::{Font as FontFace, FontSettings};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

/// Family name the bundled DejaVu Sans font is registered under.
pub const DEFAULT_FAMILY: &str = "sans-serif";

const DEFAULT_REGULAR_BYTES: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
const DEFAULT_BOLD_BYTES: &[u8] = include_bytes!("../assets/DejaVuSans-Bold.ttf");

/// Embeds a font file's bytes at compile time.
#[macro_export]
macro_rules! include_font {
    ($path:literal) => {
        include_bytes!($path)
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontWeight {
    Regular,
    Bold,
}

#[derive(Debug)]
pub struct FontError(String);

impl std::fmt::Display for FontError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "creamui-fonts: {}", self.0)
    }
}
impl std::error::Error for FontError {}

struct Registry {
    faces: HashMap<(String, FontWeight), Rc<FontFace>>,
}

impl Registry {
    fn with_defaults() -> Self {
        let mut faces = HashMap::new();
        faces.insert(
            (DEFAULT_FAMILY.to_string(), FontWeight::Regular),
            Rc::new(parse(DEFAULT_REGULAR_BYTES).expect("bundled font is a valid, fixed asset")),
        );
        faces.insert(
            (DEFAULT_FAMILY.to_string(), FontWeight::Bold),
            Rc::new(parse(DEFAULT_BOLD_BYTES).expect("bundled font is a valid, fixed asset")),
        );
        Registry { faces }
    }
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::with_defaults());
}

fn parse(bytes: &[u8]) -> Result<FontFace, FontError> {
    FontFace::from_bytes(bytes, FontSettings::default()).map_err(|e| FontError(e.to_string()))
}

/// Registers `bytes` as `family`'s face for `weight`, replacing any face
/// previously registered for that (family, weight) pair.
pub fn register_bytes(
    family: impl Into<String>,
    weight: FontWeight,
    bytes: impl AsRef<[u8]>,
) -> Result<(), FontError> {
    let face = parse(bytes.as_ref())?;
    REGISTRY.with(|registry| {
        registry
            .borrow_mut()
            .faces
            .insert((family.into(), weight), Rc::new(face));
    });
    Ok(())
}

/// Reads `path` from disk and registers it.
pub fn register_file(
    family: impl Into<String>,
    weight: FontWeight,
    path: impl AsRef<Path>,
) -> std::io::Result<()> {
    let bytes = std::fs::read(path)?;
    register_bytes(family, weight, bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Resolves a CSS-style comma-separated family stack against the registry:
/// the first family with a face registered for `weight` wins. A family
/// registered only at `Regular` still matches a `Bold` request. Falls back
/// to [`DEFAULT_FAMILY`] if nothing in the stack matches.
pub fn resolve(spec: &str, weight: FontWeight) -> Rc<FontFace> {
    REGISTRY.with(|registry| {
        let registry = registry.borrow();
        for family in spec.split(',').map(str::trim).filter(|f| !f.is_empty()) {
            if let Some(face) = registry.faces.get(&(family.to_string(), weight)) {
                return face.clone();
            }
            if weight == FontWeight::Bold {
                if let Some(face) = registry
                    .faces
                    .get(&(family.to_string(), FontWeight::Regular))
                {
                    return face.clone();
                }
            }
        }
        registry
            .faces
            .get(&(DEFAULT_FAMILY.to_string(), weight))
            .or_else(|| {
                registry
                    .faces
                    .get(&(DEFAULT_FAMILY.to_string(), FontWeight::Regular))
            })
            .expect("DEFAULT_FAMILY is always registered")
            .clone()
    })
}

/// Both weights of a resolved family stack.
#[derive(Clone)]
pub struct FontHandle {
    pub regular: Rc<FontFace>,
    pub bold: Rc<FontFace>,
}

impl FontHandle {
    pub fn weight(&self, weight: FontWeight) -> &Rc<FontFace> {
        match weight {
            FontWeight::Regular => &self.regular,
            FontWeight::Bold => &self.bold,
        }
    }
}

/// Resolves `spec` against the registry. Only callable while a
/// `with_context_scope` is active (e.g. during a window's `build_ui`);
/// panics otherwise.
pub fn use_font(spec: impl AsRef<str>) -> FontHandle {
    creamui_reactive::require_context_scope("use_font");
    FontHandle {
        regular: resolve(spec.as_ref(), FontWeight::Regular),
        bold: resolve(spec.as_ref(), FontWeight::Bold),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_the_bundled_default_family() {
        let face = resolve(DEFAULT_FAMILY, FontWeight::Regular);
        assert!(face.glyph_count() > 0);
    }

    #[test]
    fn unknown_family_falls_back_to_the_default() {
        let default = resolve(DEFAULT_FAMILY, FontWeight::Regular);
        let fallback = resolve("Nonexistent Family", FontWeight::Regular);
        assert!(Rc::ptr_eq(&default, &fallback));
    }

    #[test]
    fn css_style_stack_resolves_to_the_first_registered_family() {
        register_bytes("Test Family A", FontWeight::Regular, DEFAULT_REGULAR_BYTES).unwrap();
        let resolved = resolve(
            "Nonexistent, Test Family A, sans-serif",
            FontWeight::Regular,
        );
        let expected = resolve("Test Family A", FontWeight::Regular);
        assert!(Rc::ptr_eq(&resolved, &expected));
    }

    #[test]
    fn bold_falls_back_to_the_family_s_own_regular_before_the_default() {
        register_bytes("Test Family B", FontWeight::Regular, DEFAULT_REGULAR_BYTES).unwrap();
        let own_regular = resolve("Test Family B", FontWeight::Regular);
        let bold_request = resolve("Test Family B", FontWeight::Bold);
        assert!(Rc::ptr_eq(&own_regular, &bold_request));
    }

    #[test]
    fn register_bytes_rejects_invalid_font_data() {
        assert!(register_bytes("Broken", FontWeight::Regular, b"not a font").is_err());
    }

    #[test]
    #[should_panic(expected = "use_font")]
    fn use_font_outside_a_context_scope_panics() {
        use_font("sans-serif");
    }

    #[test]
    fn use_font_inside_a_context_scope_resolves() {
        creamui_reactive::with_context_scope(|| {
            let handle = use_font(DEFAULT_FAMILY);
            assert!(handle.regular.glyph_count() > 0);
            assert!(handle.bold.glyph_count() > 0);
        });
    }
}
