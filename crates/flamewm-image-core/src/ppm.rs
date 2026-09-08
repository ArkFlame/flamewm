//! Binary PPM (P6) decode. Output is always opaque (alpha=255).

use crate::{RgbaImage, check_size_limits};

/// Decodes binary PPM (P6), max value 255, single whitespace terminator.
pub fn decode(bytes: &[u8]) -> Result<RgbaImage, String> {
    let mut cursor = 0usize;
    let magic = ppm_token(bytes, &mut cursor)?.ok_or("missing PPM magic")?;
    if magic != b"P6" {
        return Err("only binary PPM (P6) is supported".to_string());
    }
    let width = parse_u32(ppm_token(bytes, &mut cursor)?, "width")?;
    let height = parse_u32(ppm_token(bytes, &mut cursor)?, "height")?;
    let max = parse_u32(ppm_token(bytes, &mut cursor)?, "max value")?;
    check_size_limits(width, height)?;
    if max != 255 {
        return Err(format!("PPM max value must be 255, found {max}"));
    }
    if cursor >= bytes.len() || !bytes[cursor].is_ascii_whitespace() {
        return Err("PPM header is not terminated by whitespace".to_string());
    }
    cursor += 1;
    let expected = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(3))
        .and_then(|count| usize::try_from(count).ok())
        .ok_or("PPM dimensions overflow")?;
    if bytes.len().saturating_sub(cursor) != expected {
        return Err(format!(
            "PPM pixel payload has {} bytes; expected {expected}",
            bytes.len().saturating_sub(cursor)
        ));
    }
    RgbaImage::from_rgb8_opaque(width, height, &bytes[cursor..])
        .ok_or_else(|| "PPM dimensions overflow".to_string())
}

fn ppm_token<'a>(bytes: &'a [u8], cursor: &mut usize) -> Result<Option<&'a [u8]>, String> {
    loop {
        while *cursor < bytes.len() && bytes[*cursor].is_ascii_whitespace() {
            *cursor += 1;
        }
        if *cursor >= bytes.len() {
            return Ok(None);
        }
        if bytes[*cursor] == b'#' {
            while *cursor < bytes.len() && bytes[*cursor] != b'\n' {
                *cursor += 1;
            }
            continue;
        }
        break;
    }
    let start = *cursor;
    while *cursor < bytes.len() && !bytes[*cursor].is_ascii_whitespace() && bytes[*cursor] != b'#' {
        *cursor += 1;
    }
    if start == *cursor {
        return Err("invalid empty PPM token".to_string());
    }
    Ok(Some(&bytes[start..*cursor]))
}

fn parse_u32(token: Option<&[u8]>, field: &str) -> Result<u32, String> {
    let token = token.ok_or_else(|| format!("missing PPM {field}"))?;
    let text = std::str::from_utf8(token).map_err(|_| format!("PPM {field} is not ASCII"))?;
    text.parse::<u32>()
        .map_err(|_| format!("invalid PPM {field} '{text}'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_p6_opaque() {
        let mut bytes = b"P6\n1 1\n255\n".to_vec();
        bytes.extend_from_slice(&[0x12, 0x34, 0x56]);
        let image = decode(&bytes).unwrap();
        assert_eq!(image.pixels, vec![0x12, 0x34, 0x56, 255]);
        assert!(image.is_fully_opaque());
    }

    #[test]
    fn rejects_wrong_magic_and_payload() {
        assert!(decode(b"P3\n1 1\n255\n\x00\x00\x00").is_err());
        let mut bytes = b"P6\n1 1\n255\n".to_vec();
        bytes.extend_from_slice(&[0x00, 0x00]);
        assert!(decode(&bytes).is_err());
    }

    #[test]
    fn rejects_zero_and_oversized_dimensions() {
        assert!(decode(b"P6\n0 1\n255\n").is_err());
        assert!(decode(b"P6\n5000 1\n255\n").is_err());
    }
}
