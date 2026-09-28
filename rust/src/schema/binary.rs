use std::fmt::Write;

use anyhow::{Result, bail, ensure};
use encoding_rs::WINDOWS_1252;

use super::{MAX_COUNT, MAX_OUTPUT, Primitive};

pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    pub pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
    pub fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    pub fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        ensure!(
            count <= self.remaining(),
            "Truncated data at offset 0x{:X}: need {count} bytes, have {}",
            self.pos,
            self.remaining()
        );
        let start = self.pos;
        self.pos += count;
        Ok(&self.bytes[start..self.pos])
    }

    pub fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn i16(&mut self) -> Result<i16> {
        Ok(i16::from_le_bytes(self.take(2)?.try_into()?))
    }
    pub fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into()?))
    }
    pub fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into()?))
    }

    pub fn compact(&mut self) -> Result<i32> {
        let first = self.byte()?;
        let negative = first & 0x80 != 0;
        let mut value = u32::from(first & 0x3f);
        if first & 0x40 != 0 {
            for index in 1..5 {
                let byte = self.byte()?;
                if index == 4 {
                    ensure!(
                        byte & 0xe0 == 0,
                        "Compact integer overflows at offset 0x{:X}",
                        self.pos - 1
                    );
                    value |= u32::from(byte) << 27;
                } else {
                    value |= u32::from(byte & 0x7f) << (6 + (index - 1) * 7);
                    if byte & 0x80 == 0 {
                        break;
                    }
                }
            }
        }
        ensure!(
            value <= if negative { 0x8000_0000 } else { 0x7fff_ffff },
            "Compact integer out of range at offset 0x{:X}",
            self.pos
        );
        Ok(if negative {
            (value as i32).wrapping_neg()
        } else {
            value as i32
        })
    }

    pub fn unicode(&mut self, raw: bool) -> Result<String> {
        let length = self.i32()?;
        ensure!(
            length >= 0 && length as usize <= MAX_COUNT,
            "Invalid UTF-16 byte length {length}"
        );
        ensure!(length % 2 == 0, "Odd UTF-16 byte length {length}");
        let bytes = self.take(length as usize)?;
        let text = decode_utf16(bytes)?;
        Ok(escape_lines(text, raw))
    }

    pub fn string(&mut self, raw: bool) -> Result<String> {
        let length = self.compact()?;
        if length == 0 {
            return Ok(String::new());
        }
        let bytes_count = if length > 0 {
            length as usize
        } else {
            length.unsigned_abs() as usize * 2
        };
        ensure!(
            bytes_count <= MAX_COUNT,
            "ASCF byte length {bytes_count} exceeds {MAX_COUNT}"
        );
        let bytes = self.take(bytes_count)?;
        let text = if length > 0 {
            ensure!(
                bytes.last() == Some(&0),
                "ASCF string missing NUL terminator"
            );
            WINDOWS_1252
                .decode_without_bom_handling(&bytes[..bytes.len() - 1])
                .0
                .into_owned()
        } else {
            ensure!(
                bytes.ends_with(&[0, 0]),
                "UTF-16 ASCF string missing NUL terminator"
            );
            decode_utf16(&bytes[..bytes.len() - 2])?
        };
        Ok(escape_lines(text, raw))
    }

    pub fn primitive(&mut self, primitive: Primitive, raw: bool) -> Result<String> {
        Ok(match primitive {
            Primitive::Uchar => (self.byte()? as i8).to_string(),
            Primitive::Ubyte => self.byte()?.to_string(),
            Primitive::Counter => self.compact()?.to_string(),
            Primitive::Short => self.i16()?.to_string(),
            Primitive::Ushort => (self.i16()? as u16).to_string(),
            Primitive::Uint | Primitive::Int | Primitive::MapInt => self.i32()?.to_string(),
            Primitive::Long => self.i64()?.to_string(),
            Primitive::Unicode => self.unicode(raw)?,
            Primitive::Ascf | Primitive::String => self.string(raw)?,
            Primitive::Float => {
                let value = f32::from_bits(self.i32()? as u32);
                ensure!(
                    value.is_finite(),
                    "Non-finite FLOAT cannot be losslessly edited as text"
                );
                float_text(value)
            }
            Primitive::Double => {
                let value = f64::from_bits(self.i64()? as u64);
                ensure!(
                    value.is_finite(),
                    "Non-finite DOUBLE cannot be losslessly edited as text"
                );
                let mut text = value.to_string();
                if !text.contains('.') {
                    text.push_str(".0");
                }
                text
            }
            Primitive::Hex | Primitive::Rgb | Primitive::Rgba => {
                let size = match primitive {
                    Primitive::Hex => 1,
                    Primitive::Rgb => 3,
                    _ => 4,
                };
                let mut text = String::with_capacity(size * 2);
                for byte in self.take(size)? {
                    write!(text, "{byte:02X}")?;
                }
                text
            }
        })
    }
}

fn decode_utf16(bytes: &[u8]) -> Result<String> {
    let mut text = String::with_capacity(bytes.len());
    for character in char::decode_utf16(
        bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]])),
    ) {
        text.push(character.map_err(|error| anyhow::anyhow!("Invalid UTF-16: {error}"))?);
    }
    Ok(text)
}

fn escape_lines(text: String, raw: bool) -> String {
    if !raw && text.contains("\r\n") {
        text.replace("\r\n", "\\r\\n")
    } else {
        text
    }
}

fn float_text(value: f32) -> String {
    let absolute = value.abs();
    if absolute != 0.0 && !(0.001..10_000_000.0).contains(&absolute) {
        let text = format!("{value:e}");
        let (mantissa, exponent) = text
            .split_once('e')
            .expect("Scientific formatting has an exponent");
        let fraction = if mantissa.contains('.') {
            mantissa.to_owned()
        } else {
            format!("{mantissa}.0")
        };
        format!("{fraction}E{exponent}")
    } else {
        let mut text = value.to_string();
        if !text.contains('.') {
            text.push_str(".0");
        }
        text
    }
}

pub(super) fn compact(out: &mut Vec<u8>, value: i32) {
    let mut magnitude = value.unsigned_abs();
    let mut first = (magnitude & 0x3f) as u8;
    magnitude >>= 6;
    if value < 0 {
        first |= 0x80;
    }
    if magnitude != 0 {
        first |= 0x40;
    }
    out.push(first);
    while magnitude != 0 {
        let mut byte = (magnitude & 0x7f) as u8;
        magnitude >>= 7;
        if magnitude != 0 {
            byte |= 0x80;
        }
        out.push(byte);
    }
}

pub(super) fn unicode(out: &mut Vec<u8>, text: &str, raw: bool) -> Result<()> {
    let unescaped;
    let text = if !raw && text.contains("\\r\\n") {
        unescaped = text.replace("\\r\\n", "\r\n");
        &unescaped
    } else {
        text
    };
    let count = text.encode_utf16().count();
    ensure!(
        count <= MAX_COUNT / 2,
        "UTF-16 string exceeds {MAX_COUNT} bytes"
    );
    out.extend_from_slice(&((count * 2) as i32).to_le_bytes());
    for unit in text.encode_utf16() {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    Ok(())
}

pub(super) fn string(out: &mut Vec<u8>, text: &str, raw: bool) -> Result<()> {
    let unescaped;
    let text = if !raw && text.contains("\\r\\n") {
        unescaped = text.replace("\\r\\n", "\r\n");
        &unescaped
    } else {
        text
    };
    if text.is_empty() {
        compact(out, 0);
    } else if text.is_ascii() {
        ensure!(
            text.len() < MAX_COUNT,
            "ASCF string exceeds {MAX_COUNT} bytes"
        );
        compact(out, (text.len() + 1) as i32);
        out.extend_from_slice(text.as_bytes());
        out.push(0);
    } else {
        let count = text.encode_utf16().count() + 1;
        ensure!(
            count <= MAX_COUNT / 2,
            "ASCF string exceeds {MAX_COUNT} bytes"
        );
        compact(out, -(count as i32));
        for unit in text.encode_utf16() {
            out.extend_from_slice(&unit.to_le_bytes());
        }
        out.extend_from_slice(&[0, 0]);
    }
    Ok(())
}

pub(super) fn primitive(
    out: &mut Vec<u8>,
    primitive: Primitive,
    text: &str,
    raw: bool,
) -> Result<()> {
    match primitive {
        Primitive::Uchar => out.push(text.parse::<i8>()? as u8),
        Primitive::Ubyte => out.push(text.parse::<u8>()?),
        Primitive::Counter => compact(out, text.parse()?),
        Primitive::Short => out.extend_from_slice(&text.parse::<i16>()?.to_le_bytes()),
        Primitive::Ushort => out.extend_from_slice(&text.parse::<u16>()?.to_le_bytes()),
        Primitive::Int | Primitive::Uint | Primitive::MapInt => {
            out.extend_from_slice(&text.parse::<i32>()?.to_le_bytes())
        }
        Primitive::Long => out.extend_from_slice(&text.parse::<i64>()?.to_le_bytes()),
        Primitive::Float => {
            let value = text.parse::<f32>()?;
            ensure!(value.is_finite(), "Non-finite FLOAT is not supported");
            out.extend_from_slice(&value.to_le_bytes());
        }
        Primitive::Double => {
            let value = text.parse::<f64>()?;
            ensure!(value.is_finite(), "Non-finite DOUBLE is not supported");
            out.extend_from_slice(&value.to_le_bytes());
        }
        Primitive::Unicode => unicode(out, text, raw)?,
        Primitive::Ascf | Primitive::String => string(out, text, raw)?,
        Primitive::Hex | Primitive::Rgb | Primitive::Rgba => {
            let length = match primitive {
                Primitive::Hex => 2,
                Primitive::Rgb => 6,
                _ => 8,
            };
            ensure!(
                text.len() == length && text.is_ascii(),
                "Expected {length} hexadecimal digits, got {text:?}"
            );
            for pair in text.as_bytes().chunks_exact(2) {
                let byte = std::str::from_utf8(pair)?;
                out.push(u8::from_str_radix(byte, 16)?);
            }
        }
    }
    ensure!(
        out.len() <= MAX_OUTPUT,
        "Binary output exceeds {MAX_OUTPUT} bytes"
    );
    Ok(())
}

pub(super) fn unbracket(text: &str) -> Result<&str> {
    match text
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
    {
        Some(text) => Ok(text),
        None => bail!("String value must be enclosed in square brackets: {text:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_integer_signs_and_boundaries() {
        for value in [
            i32::MIN,
            -134217728,
            -8192,
            -64,
            -1,
            0,
            1,
            63,
            64,
            8191,
            8192,
            134217727,
            i32::MAX,
        ] {
            let mut bytes = Vec::new();
            compact(&mut bytes, value);
            let mut reader = Reader::new(&bytes);
            assert_eq!(reader.compact().unwrap(), value);
            assert_eq!(reader.remaining(), 0);
        }
        assert!(Reader::new(&[0x40]).compact().is_err());
        assert!(
            Reader::new(&[0x40, 0x80, 0x80, 0x80, 0xff])
                .compact()
                .is_err()
        );
    }

    #[test]
    fn strings_preserve_unicode_and_line_breaks() {
        for value in ["", "ASCII", "Café", "한글 😀", "one\\r\\ntwo"] {
            let mut bytes = Vec::new();
            string(&mut bytes, value, false).unwrap();
            assert_eq!(Reader::new(&bytes).string(false).unwrap(), value);
            bytes.clear();
            unicode(&mut bytes, value, false).unwrap();
            assert_eq!(Reader::new(&bytes).unicode(false).unwrap(), value);
        }
        assert_eq!(Reader::new(&[2, 0xe9, 0]).string(false).unwrap(), "é");
        assert!(Reader::new(&[2, b'a', b'b']).string(false).is_err());
        assert!(Reader::new(&[3, 0, 0, 0, 0, 0, 0]).unicode(false).is_err());
    }
}
