//! PNG textures: decoding to RGBA, re-encoding without ancillary chunks.
use base64::Engine;

/// A decoded texture, 8-bit RGBA, rows top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum TextureError {
    #[error("not base64: {0}")]
    Base64(String),
    #[error("not a PNG: {0}")]
    Png(String),
    #[error("{0}x{1} is empty")]
    Empty(u32, u32),
}

impl Texture {
    /// Decodes PNG bytes (any colour type and depth) to RGBA8.
    pub fn decode_png(bytes: &[u8]) -> Result<Texture, TextureError> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder
            .read_info()
            .map_err(|e| TextureError::Png(e.to_string()))?;
        let (width, height) = (reader.info().width, reader.info().height);
        if width == 0 || height == 0 {
            return Err(TextureError::Empty(width, height));
        }
        let size = reader
            .output_buffer_size()
            .ok_or_else(|| TextureError::Png("image too large".into()))?;
        let mut buf = vec![0; size];
        let frame = reader
            .next_frame(&mut buf)
            .map_err(|e| TextureError::Png(e.to_string()))?;
        let pixels = &buf[..frame.buffer_size()];
        let rgba = match frame.color_type {
            png::ColorType::Rgba => pixels.to_vec(),
            png::ColorType::Rgb => pixels
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2], 255])
                .collect(),
            png::ColorType::GrayscaleAlpha => pixels
                .as_chunks::<2>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[0], p[0], p[1]])
                .collect(),
            png::ColorType::Grayscale => pixels.iter().flat_map(|&g| [g, g, g, 255]).collect(),
            png::ColorType::Indexed => {
                return Err(TextureError::Png("palette not expanded".into()));
            }
        };
        Ok(Texture {
            width,
            height,
            rgba,
        })
    }

    /// Decodes base64 PNG text.
    pub fn decode_base64(text: &str) -> Result<Texture, TextureError> {
        Self::decode_png(&decode_base64(text)?)
    }

    /// Encodes as an RGBA8 PNG with no ancillary chunk (no text, time or colour profile): the
    /// same pixels always give the same bytes.
    pub fn encode_png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_compression(png::Compression::High);
            // Writing into a Vec cannot fail for a well-formed image.
            if let Ok(mut writer) = encoder.write_header() {
                let _ = writer.write_image_data(&self.rgba);
            }
        }
        out
    }

    /// Texels with any alpha.
    pub fn used(&self) -> usize {
        self.rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] != 0)
            .count()
    }

    /// The texture inside a 64x64 RGBA slot, top-left aligned (what a skin-atlas slot takes).
    pub fn slot_64(&self) -> Option<Vec<u8>> {
        if self.width > 64 || self.height > 64 {
            return None;
        }
        let mut out = vec![0; 64 * 64 * 4];
        let row = self.width as usize * 4;
        for y in 0..self.height as usize {
            out[y * 256..y * 256 + row].copy_from_slice(&self.rgba[y * row..(y + 1) * row]);
        }
        Some(out)
    }
}

pub fn decode_base64(text: &str) -> Result<Vec<u8>, TextureError> {
    base64::engine::general_purpose::STANDARD
        .decode(text.trim())
        .map_err(|e| TextureError::Base64(e.to_string()))
}

pub fn encode_base64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_round_trip_is_stable() {
        let t = Texture {
            width: 2,
            height: 2,
            rgba: vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 0, 0, 0, 0],
        };
        let png = t.encode_png();
        let back = Texture::decode_png(&png).unwrap();
        assert_eq!(back, t);
        assert_eq!(back.encode_png(), png, "deterministic bytes");
        assert_eq!(t.used(), 3);
        let slot = t.slot_64().unwrap();
        assert_eq!(&slot[0..8], &t.rgba[0..8]);
        assert_eq!(&slot[256..264], &t.rgba[8..16]);
    }

    #[test]
    fn garbage_is_refused() {
        assert!(Texture::decode_png(b"nope").is_err());
        assert!(Texture::decode_base64("!!!").is_err());
    }
}
