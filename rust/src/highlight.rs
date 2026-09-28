use std::ops::Range;
use std::sync::{Arc, Weak};

use eframe::egui::epaint::{TextureAtlas, mutex::Mutex};
use eframe::egui::text::{LayoutJob, LayoutSection};
use eframe::egui::{Color32, FontId, Galley, TextFormat, Ui};

const TEXT: Color32 = Color32::from_rgb(0xdd, 0xdf, 0xe5);
const MARKER: Color32 = Color32::from_rgb(0xde, 0xa7, 0x7b);
const KEY: Color32 = Color32::from_rgb(0xd3, 0xc5, 0xaf);
const STRING: Color32 = Color32::from_rgb(0xa5, 0xc8, 0x90);
const NUMBER: Color32 = Color32::from_rgb(0x91, 0xaf, 0xca);

#[derive(Default)]
pub struct Highlighter {
    galley: Option<Arc<Galley>>,
    pixels_per_point: f32,
    atlas: Weak<Mutex<TextureAtlas>>,
}

impl Highlighter {
    pub fn layout(&mut self, ui: &Ui, text: &str, wrap_width: f32) -> Arc<Galley> {
        let pixels_per_point = ui.pixels_per_point();
        ui.fonts(|fonts| {
            // Atlas identity also invalidates cached glyphs after font replacement
            // or atlas recreation, even when the requested font size is unchanged.
            let atlas = fonts.texture_atlas();
            let cached = self
                .galley
                .as_ref()
                .filter(|galley| galley.job.text == text);
            if let Some(galley) = cached {
                if galley.job.wrap.max_width == wrap_width
                    && self.pixels_per_point == pixels_per_point
                    && self.atlas.as_ptr() == Arc::as_ptr(&atlas)
                {
                    return Arc::clone(galley);
                }
            }

            let mut job = match cached {
                Some(galley) => (*galley.job).clone(),
                _ => highlighted_job(text),
            };
            job.wrap.max_width = wrap_width;
            let galley = fonts.layout_job(job);
            self.pixels_per_point = pixels_per_point;
            self.atlas = Arc::downgrade(&atlas);
            self.galley = Some(Arc::clone(&galley));
            galley
        })
    }
}

fn highlighted_job(text: &str) -> LayoutJob {
    let mut job = LayoutJob {
        text: text.to_owned(),
        ..Default::default()
    };
    let bytes = text.as_bytes();
    let mut position = 0;
    while position < bytes.len() {
        let start = position;
        let color = match bytes[position] {
            b'[' => {
                // DAT strings may contain nested brackets, braces, tabs and newlines.
                // An unfinished string remains highlighted while the user edits it.
                let mut depth = 1usize;
                position += 1;
                while position < bytes.len() && depth != 0 {
                    match bytes[position] {
                        b'[' => depth += 1,
                        b']' => depth -= 1,
                        _ => {}
                    }
                    position += 1;
                }
                STRING
            }
            byte if is_separator(byte) => {
                position += 1;
                TEXT
            }
            _ => {
                position += 1;
                while position < bytes.len() && !is_separator(bytes[position]) {
                    position += 1;
                }
                // All separators are ASCII, so these offsets are UTF-8 boundaries.
                let word = &text[start..position];
                let mut next = position;
                while next < bytes.len() && bytes[next].is_ascii_whitespace() {
                    next += 1;
                }
                if bytes.get(next) == Some(&b'=') {
                    KEY
                } else if word.ends_with("_begin") || word.ends_with("_end") {
                    MARKER
                } else if matches!(bytes[start], b'0'..=b'9' | b'+' | b'-' | b'.')
                    && word.parse::<f64>().is_ok()
                {
                    NUMBER
                } else {
                    TEXT
                }
            }
        };
        push_section(&mut job, start..position, color);
    }
    if job.sections.is_empty() {
        push_section(&mut job, 0..0, TEXT);
    }
    job
}

fn is_separator(byte: u8) -> bool {
    byte.is_ascii_whitespace() || matches!(byte, b'=' | b'[' | b']' | b'{' | b'}' | b';' | b',')
}

fn push_section(job: &mut LayoutJob, byte_range: Range<usize>, color: Color32) {
    if let Some(previous) = job.sections.last_mut() {
        if previous.format.color == color {
            previous.byte_range.end = byte_range.end;
            return;
        }
    }
    job.sections.push(LayoutSection {
        leading_space: 0.0,
        byte_range,
        format: TextFormat::simple(FontId::monospace(14.0), color),
    });
}

#[cfg(test)]
mod tests {
    use super::{KEY, MARKER, NUMBER, STRING, TEXT, highlighted_job};

    #[test]
    fn preserves_unicode_and_nested_multiline_strings_inside_lists() {
        let text =
            "record_begin\r\n\tnome={[Olá [勇者]\n\t{42}; x_end];{-1;.5;2e+3;12abc}}\trecord_end";
        let job = highlighted_job(text);
        assert_eq!(job.text, text);
        let sections: Vec<_> = job
            .sections
            .iter()
            .map(|section| (&job.text[section.byte_range.clone()], section.format.color))
            .collect();
        assert_eq!(
            sections,
            vec![
                ("record_begin", MARKER),
                ("\r\n\t", TEXT),
                ("nome", KEY),
                ("={", TEXT),
                ("[Olá [勇者]\n\t{42}; x_end]", STRING),
                (";{", TEXT),
                ("-1", NUMBER),
                (";", TEXT),
                (".5", NUMBER),
                (";", TEXT),
                ("2e+3", NUMBER),
                (";12abc}}\t", TEXT),
                ("record_end", MARKER),
            ]
        );
        let mut end = 0;
        for section in &job.sections {
            assert_eq!(section.byte_range.start, end);
            end = section.byte_range.end;
        }
        assert_eq!(end, text.len());
    }

    #[test]
    fn tolerates_unmatched_delimiters_without_reclassifying_string_contents() {
        let text = "]} 名称 = [未完 [nested] key=12\nthing_end";
        let job = highlighted_job(text);
        assert_eq!(job.text, text);
        let sections: Vec<_> = job
            .sections
            .iter()
            .map(|section| (&job.text[section.byte_range.clone()], section.format.color))
            .collect();
        assert_eq!(
            sections,
            vec![
                ("]} ", TEXT),
                ("名称", KEY),
                (" = ", TEXT),
                ("[未完 [nested] key=12\nthing_end", STRING),
            ]
        );
    }
}
