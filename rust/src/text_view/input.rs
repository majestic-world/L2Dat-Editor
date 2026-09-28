use std::borrow::Cow;
use std::ops::Range;

use eframe::egui::{self, Event, ImeEvent, Key, Modifiers, PointerButton, Rect, Vec2};

use super::{Composition, Drag, TextEditor, VisibleLine};
use crate::text_state::Selection;

fn is_editor_event(event: &Event) -> bool {
    match event {
        Event::Copy | Event::Cut | Event::Paste(_) | Event::Text(_) | Event::Ime(_) => true,
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => match key {
            Key::ArrowLeft
            | Key::ArrowRight
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
            | Key::Backspace
            | Key::Delete
            | Key::Enter
            | Key::Tab
            | Key::Escape => true,
            Key::A | Key::D | Key::Z | Key::Y => modifiers.command || modifiers.ctrl,
            _ => false,
        },
        _ => false,
    }
}

impl TextEditor {
    pub(super) fn process_events(
        &mut self,
        ui: &egui::Ui,
        text: &mut String,
        row_height: f32,
    ) -> bool {
        let mut changed = false;
        let page_rows = ((self.viewport_size.y / row_height).floor() as usize)
            .saturating_sub(1)
            .max(1);
        // Move paste/IME payloads instead of cloning potentially large strings.
        // Leave unrelated events available to egui's global shortcuts/widgets.
        let events = ui.input_mut(|input| {
            input
                .events
                .extract_if(.., |event| is_editor_event(event))
                .collect::<Vec<_>>()
        });
        for event in events {
            let before = self.state.selection;
            let edited = match event {
                Event::Copy => {
                    self.copy(ui, text);
                    false
                }
                Event::Cut => {
                    self.copy(ui, text);
                    self.cancel_composition();
                    !self.state.selection.is_empty() && self.state.replace_selection(text, "")
                }
                Event::Paste(inserted) => {
                    self.cancel_composition();
                    !inserted.is_empty()
                        && self
                            .state
                            .replace_selection(text, &normalize_newlines(&inserted))
                }
                Event::Text(inserted) if !self.ime_enabled => {
                    !inserted.is_empty()
                        && inserted != "\n"
                        && inserted != "\r"
                        && self.state.replace_selection(text, &inserted)
                }
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    repeat,
                    ..
                } => {
                    if self.ime_enabled
                        && (repeat
                            || matches!(
                                key,
                                Key::ArrowLeft
                                    | Key::ArrowRight
                                    | Key::ArrowUp
                                    | Key::ArrowDown
                                    | Key::Enter
                                    | Key::Backspace
                                    | Key::Delete
                            ))
                    {
                        false
                    } else {
                        self.key(text, key, modifiers, page_rows)
                    }
                }
                Event::Ime(event) => self.ime(text, event),
                _ => false,
            };
            if edited || self.state.selection != before {
                self.reveal = true;
                self.drag = None;
                if edited {
                    self.preferred_column = None;
                }
            }
            changed |= edited;
        }
        changed
    }

    fn copy(&self, ui: &egui::Ui, text: &str) {
        if !self.state.selection.is_empty() {
            ui.ctx()
                .copy_text(text[self.state.selection.range()].to_owned());
        }
    }

    fn cancel_composition(&mut self) {
        self.composition = None;
        self.ime_enabled = false;
    }

    fn ime(&mut self, text: &mut String, event: ImeEvent) -> bool {
        match event {
            ImeEvent::Enabled => {
                self.ime_enabled = true;
            }
            ImeEvent::Preedit(preedit) => {
                self.ime_enabled = true;
                if preedit != "\n" && preedit != "\r" {
                    let composition = self.composition.get_or_insert_with(|| Composition {
                        text: String::new(),
                        selection: self.state.selection,
                    });
                    composition.text = preedit;
                    self.reveal = true;
                }
            }
            ImeEvent::Commit(committed) => {
                let selection = self
                    .composition
                    .take()
                    .map_or(self.state.selection, |c| c.selection);
                self.ime_enabled = false;
                if !committed.is_empty() && committed != "\n" && committed != "\r" {
                    self.state.selection = selection;
                    return self.state.replace_selection(text, &committed);
                }
            }
            ImeEvent::Disabled => self.cancel_composition(),
        }
        false
    }

    fn key(&mut self, text: &mut String, key: Key, modifiers: Modifiers, page_rows: usize) -> bool {
        let command = modifiers.command || modifiers.ctrl;
        if command && key == Key::A {
            self.cancel_composition();
            self.state.selection = Selection {
                anchor: 0,
                head: text.len(),
            };
            self.preferred_column = None;
            return false;
        }
        if command && (key == Key::Z || key == Key::Y) {
            self.cancel_composition();
            return if key == Key::Y || modifiers.shift {
                self.state.redo(text)
            } else {
                self.state.undo(text)
            };
        }
        if command && key == Key::D {
            self.cancel_composition();
            return self.state.duplicate_lines(text);
        }
        let head = self.state.selection.head;
        let row = self.state.line_for_offset(head);
        let mut vertical = false;
        let target = match key {
            Key::ArrowLeft => Some(if !modifiers.shift && !self.state.selection.is_empty() {
                self.state.selection.range().start
            } else if command || modifiers.alt {
                previous_word(text, head)
            } else {
                previous_char(text, head)
            }),
            Key::ArrowRight => Some(if !modifiers.shift && !self.state.selection.is_empty() {
                self.state.selection.range().end
            } else if command || modifiers.alt {
                next_word(text, head)
            } else {
                next_char(text, head)
            }),
            Key::Home => Some(if command {
                0
            } else {
                self.state.line_start(row)
            }),
            Key::End => Some(if command {
                text.len()
            } else {
                self.state.line_range(text, row).end
            }),
            Key::ArrowUp | Key::ArrowDown | Key::PageUp | Key::PageDown => {
                vertical = true;
                if command && matches!(key, Key::ArrowUp | Key::ArrowDown) {
                    Some(if key == Key::ArrowUp { 0 } else { text.len() })
                } else {
                    let amount = if matches!(key, Key::PageUp | Key::PageDown) {
                        page_rows
                    } else {
                        1
                    };
                    let target_row = if matches!(key, Key::ArrowUp | Key::PageUp) {
                        row.saturating_sub(amount)
                    } else {
                        row.saturating_add(amount).min(self.state.line_count() - 1)
                    };
                    let column = self
                        .preferred_column
                        .unwrap_or_else(|| self.visual_column(text, head));
                    self.preferred_column = Some(column);
                    Some(self.byte_at_visual_column(text, target_row, column))
                }
            }
            _ => None,
        };
        if let Some(target) = target {
            self.cancel_composition();
            self.state.selection.head = target;
            if !modifiers.shift {
                self.state.selection.anchor = target;
            }
            if !vertical {
                self.preferred_column = None;
            }
            return false;
        }
        match key {
            Key::Backspace | Key::Delete => {
                self.cancel_composition();
                let mut range = self.state.selection.range();
                if range.is_empty() {
                    if key == Key::Backspace {
                        range.start = if command || modifiers.alt {
                            previous_word(text, head)
                        } else {
                            previous_char(text, head)
                        };
                    } else {
                        range.end = if command || modifiers.alt {
                            next_word(text, head)
                        } else {
                            next_char(text, head)
                        };
                    }
                }
                !range.is_empty() && self.state.replace(text, range, "")
            }
            Key::Enter if !command && !modifiers.alt => self.state.replace_selection(text, "\n"),
            Key::Tab if !command && !modifiers.alt => self.state.replace_selection(text, "\t"),
            Key::Escape => {
                self.cancel_composition();
                false
            }
            _ => false,
        }
    }

    pub(super) fn visual_column(&self, text: &str, byte: usize) -> usize {
        let row = self.state.line_for_offset(byte);
        text[self.state.line_start(row)..byte]
            .chars()
            .map(|c| if c == '\t' { 4 } else { 1 })
            .sum()
    }

    fn byte_at_visual_column(&self, text: &str, row: usize, target: usize) -> usize {
        let range = self.state.line_range(text, row);
        let mut column = 0;
        for (offset, ch) in text[range.clone()].char_indices() {
            let width = if ch == '\t' { 4 } else { 1 };
            if column + width > target {
                return range.start + offset;
            }
            column += width;
        }
        range.end
    }

    pub(super) fn pointer_input(
        &mut self,
        ui: &egui::Ui,
        text: &str,
        response: &egui::Response,
        lines: &[VisibleLine],
        clip: Rect,
        row_height: f32,
    ) -> Vec2 {
        let (pointer, pressed, down, shift, dt) = ui.input(|input| {
            (
                input.pointer.interact_pos(),
                input.pointer.button_pressed(PointerButton::Primary),
                input.pointer.button_down(PointerButton::Primary),
                input.modifiers.shift,
                input.stable_dt.min(0.05),
            )
        });
        let Some(pointer) = pointer else {
            self.drag = None;
            return Vec2::ZERO;
        };
        if lines.is_empty() {
            return Vec2::ZERO;
        }
        let clamped = egui::pos2(
            pointer.x.max(clip.left()).min(clip.right()),
            pointer.y.max(clip.top()).min(clip.bottom() - 0.1),
        );
        let first = &lines[0];
        let index = (((clamped.y - first.origin.y) / row_height).floor().max(0.0) as usize)
            .min(lines.len() - 1);
        let line = &lines[index];
        let char_index = line
            .galley
            .cursor_from_pos(egui::vec2(
                clamped.x - line.origin.x,
                line.galley.size().y * 0.5,
            ))
            .ccursor
            .index;
        let byte = self.state.byte_at_column(text, line.row, char_index);
        if (pressed && response.hovered()) || response.clicked() {
            response.request_focus();
            self.cancel_composition();
            self.preferred_column = None;
            if !shift {
                self.state.selection.anchor = byte;
            }
            self.state.selection.head = byte;
            self.drag = Some(Drag {
                anchor: self.state.selection.anchor,
                word: None,
            });
        }
        if response.double_clicked() {
            let range = word_at(text, byte);
            self.state.selection = Selection {
                anchor: range.start,
                head: range.end,
            };
            self.drag = Some(Drag {
                anchor: range.start,
                word: Some(range),
            });
        }
        if response.triple_clicked() {
            let range = self.state.line_range(text, line.row);
            let end = if range.end < text.len() {
                range.end + 1
            } else {
                range.end
            };
            self.state.selection = Selection {
                anchor: range.start,
                head: end,
            };
            self.drag = Some(Drag {
                anchor: range.start,
                word: Some(range.start..end),
            });
        }
        let mut scroll = Vec2::ZERO;
        if down {
            if let Some(drag) = &self.drag {
                if let Some(word) = &drag.word {
                    let target = word_at(text, byte);
                    self.state.selection = if byte < word.start {
                        Selection {
                            anchor: word.end,
                            head: target.start,
                        }
                    } else {
                        Selection {
                            anchor: word.start,
                            head: target.end.max(word.end),
                        }
                    };
                } else {
                    self.state.selection = Selection {
                        anchor: drag.anchor,
                        head: byte,
                    };
                }
                scroll.x = autoscroll(pointer.x, clip.left(), clip.right()) * dt;
                scroll.y = autoscroll(pointer.y, clip.top(), clip.bottom()) * dt;
                if scroll != Vec2::ZERO {
                    ui.ctx().request_repaint();
                }
            }
        } else {
            self.drag = None;
        }
        scroll
    }
}

fn normalize_newlines(text: &str) -> Cow<'_, str> {
    if text.contains('\r') {
        Cow::Owned(text.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

fn previous_char(text: &str, byte: usize) -> usize {
    text[..byte]
        .char_indices()
        .next_back()
        .map_or(0, |(offset, _)| offset)
}

fn next_char(text: &str, byte: usize) -> usize {
    byte + text[byte..].chars().next().map_or(0, char::len_utf8)
}

fn word_kind(ch: char) -> u8 {
    if ch.is_whitespace() {
        0
    } else if ch.is_alphanumeric() || ch == '_' {
        1
    } else {
        2
    }
}

fn previous_word(text: &str, mut byte: usize) -> usize {
    while byte > 0 {
        let previous = previous_char(text, byte);
        if !text[previous..byte].chars().next().unwrap().is_whitespace() {
            break;
        }
        byte = previous;
    }
    if byte > 0 {
        let kind = word_kind(text[..byte].chars().next_back().unwrap());
        while byte > 0 {
            let previous = previous_char(text, byte);
            if word_kind(text[previous..byte].chars().next().unwrap()) != kind {
                break;
            }
            byte = previous;
        }
    }
    byte
}

fn next_word(text: &str, mut byte: usize) -> usize {
    if let Some(ch) = text[byte..].chars().next() {
        let kind = word_kind(ch);
        while byte < text.len() {
            let ch = text[byte..].chars().next().unwrap();
            if word_kind(ch) != kind {
                break;
            }
            byte += ch.len_utf8();
        }
        while byte < text.len() {
            let ch = text[byte..].chars().next().unwrap();
            if !ch.is_whitespace() {
                break;
            }
            byte += ch.len_utf8();
        }
    }
    byte
}

fn word_at(text: &str, byte: usize) -> Range<usize> {
    if text.is_empty() {
        return 0..0;
    }
    let byte = if byte == text.len() {
        previous_char(text, byte)
    } else {
        byte
    };
    let ch = text[byte..].chars().next().unwrap();
    if ch == '\n' || ch == '\r' {
        return byte..byte;
    }
    let kind = word_kind(ch);
    let mut start = byte;
    let mut end = next_char(text, byte);
    while start > 0 {
        let previous = previous_char(text, start);
        let ch = text[previous..start].chars().next().unwrap();
        if ch == '\n' || ch == '\r' || word_kind(ch) != kind {
            break;
        }
        start = previous;
    }
    while end < text.len() {
        let ch = text[end..].chars().next().unwrap();
        if ch == '\n' || ch == '\r' || word_kind(ch) != kind {
            break;
        }
        end += ch.len_utf8();
    }
    start..end
}

fn autoscroll(position: f32, min: f32, max: f32) -> f32 {
    if position < min {
        -(80.0 + (min - position) * 12.0).min(1600.0)
    } else if position > max {
        (80.0 + (position - max) * 12.0).min(1600.0)
    } else {
        0.0
    }
}
