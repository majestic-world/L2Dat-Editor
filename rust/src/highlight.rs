use std::collections::HashMap;
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

struct CachedLine {
    galley: Arc<Galley>,
    string_depth: usize,
}

#[derive(Default)]
pub struct Highlighter {
    lines: HashMap<usize, CachedLine>,
    pixels_per_point: f32,
    atlas: Weak<Mutex<TextureAtlas>>,
}

impl Highlighter {
    pub fn retain_lines(&mut self, visible: Range<usize>) {
        self.lines.retain(|row, _| visible.contains(row));
    }

    pub fn layout_line(
        &mut self,
        ui: &Ui,
        row: usize,
        text: &str,
        initial_string_depth: usize,
    ) -> Arc<Galley> {
        let pixels_per_point = ui.pixels_per_point();
        ui.fonts(|fonts| {
            let atlas = fonts.texture_atlas();
            if self.pixels_per_point != pixels_per_point
                || self.atlas.as_ptr() != Arc::as_ptr(&atlas)
            {
                self.lines.clear();
                self.pixels_per_point = pixels_per_point;
                self.atlas = Arc::downgrade(&atlas);
            }
            if let Some(cached) = self.lines.get(&row) {
                if cached.string_depth == initial_string_depth && cached.galley.job.text == text {
                    return Arc::clone(&cached.galley);
                }
            }
            let mut job = highlighted_job(text, initial_string_depth);
            job.wrap.max_width = f32::INFINITY;
            let galley = fonts.layout_job(job);
            self.lines.insert(
                row,
                CachedLine {
                    galley: Arc::clone(&galley),
                    string_depth: initial_string_depth,
                },
            );
            galley
        })
    }
}

fn highlighted_job(text: &str, initial_string_depth: usize) -> LayoutJob {
    let mut job = LayoutJob {
        text: text.to_owned(),
        ..Default::default()
    };
    let bytes = text.as_bytes();
    let mut position = 0;
    let mut depth = initial_string_depth;
    while position < bytes.len() {
        let start = position;
        let color = match bytes[position] {
            b'[' if depth == 0 => {
                depth = 1;
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
            _ if depth > 0 => {
                // A viewport may begin inside a nested, multiline DAT string.
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
    use eframe::egui::Color32;
    use eframe::egui::text::LayoutJob;

    fn assert_color(job: &LayoutJob, fragment: &str, expected: Color32) {
        let start = job.text.find(fragment).expect("Fragment must exist");
        let end = start + fragment.len();
        for section in &job.sections {
            if section.byte_range.start < end && section.byte_range.end > start {
                assert_eq!(section.format.color, expected, "{fragment}");
            }
        }
        assert!(
            job.sections
                .iter()
                .any(|section| section.byte_range.contains(&start))
        );
    }

    #[test]
    fn highlights_nested_unicode_strings_and_dat_fields() {
        let text = "record_begin\tnome={[Olá [勇者]];{-1;.5;2e+3;12abc}}\trecord_end";
        let job = highlighted_job(text, 0);
        assert_eq!(job.text, text);
        assert_color(&job, "record_begin", MARKER);
        assert_color(&job, "record_end", MARKER);
        assert_color(&job, "nome", KEY);
        assert_color(&job, "[Olá [勇者]]", STRING);
        assert_color(&job, "-1", NUMBER);
        assert_color(&job, ".5", NUMBER);
        assert_color(&job, "2e+3", NUMBER);
        assert_color(&job, "12abc", TEXT);
    }

    #[test]
    fn viewport_start_inside_string_preserves_color_until_matching_close() {
        let text = "\t{42}; [nested] x_end]\tlevel=2e+3\trecord_end";
        let job = highlighted_job(text, 1);
        assert_color(&job, "\t{42}; [nested] x_end]", STRING);
        assert_color(&job, "level", KEY);
        assert_color(&job, "2e+3", NUMBER);
        assert_color(&job, "record_end", MARKER);
        let deeper = highlighted_job("inner]\tkey=12]\tkey=7", 2);
        assert_color(&deeper, "inner]\tkey=12]", STRING);
        assert_color(&deeper, "7", NUMBER);
    }

    #[test]
    fn tolerates_unmatched_delimiters_while_editing() {
        let job = highlighted_job("]} 名称 = [未完 [nested] key=12\nthing_end", 0);
        assert_color(&job, "]} ", TEXT);
        assert_color(&job, "名称", KEY);
        assert_color(&job, "[未完 [nested] key=12\nthing_end", STRING);
    }
}
