use std::ops::Range;
use std::sync::Arc;

use eframe::egui::{self, FontId, Pos2, Rect, Vec2};

use crate::highlight::Highlighter;
use crate::text_state::{Selection, TextState};
use crate::theme;

mod input;
#[cfg(test)]
mod tests;

const OVERSCAN: usize = 2;
const TEXT_PADDING: f32 = 8.0;

#[derive(Default)]
pub struct TextEditor {
    pub state: TextState,
    highlighter: Highlighter,
    scroll: Vec2,
    viewport_size: Vec2,
    request_focus: bool,
    reveal: bool,
    reset_scroll: bool,
    preferred_column: Option<usize>,
    drag: Option<Drag>,
    composition: Option<Composition>,
    ime_enabled: bool,
    rendered_rows: Range<usize>,
    line_buffer: Vec<VisibleLine>,
}

struct Drag {
    anchor: usize,
    word: Option<Range<usize>>,
}

struct Composition {
    text: String,
    selection: Selection,
}

struct VisibleLine {
    row: usize,
    origin: Pos2,
    galley: Arc<egui::Galley>,
}

impl TextEditor {
    pub fn reset(&mut self, text: &str) {
        *self = Self::default();
        self.state.reset(text);
        self.reset_scroll = true;
    }

    pub fn select_range(&mut self, range: Range<usize>) {
        self.state.selection = Selection {
            anchor: range.start,
            head: range.end,
        };
        self.request_focus = true;
        self.reveal = true;
        self.preferred_column = None;
        self.composition = None;
        self.drag = None;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, text: &mut String, enabled: bool) -> egui::Response {
        let id = egui::Id::new("document_editor");
        let enabled = enabled && ui.is_enabled();
        let available = ui.available_rect_before_wrap();
        let mut response = ui.interact(available, id, egui::Sense::click_and_drag());
        let font = FontId::monospace(14.0);
        let (char_width, row_height) = ui.fonts(|fonts| {
            (
                fonts.glyph_width(&font, ' '),
                fonts.row_height(&font).max(22.0),
            )
        });
        let gutter_width =
            (char_width * (self.state.line_count().ilog10() + 1) as f32 + 24.0).max(44.0);
        if self.viewport_size == Vec2::ZERO {
            self.viewport_size = available.size();
        }
        if self.request_focus && enabled {
            response.request_focus();
            self.request_focus = false;
        }
        let focused = response.has_focus();
        if !focused || !enabled {
            self.composition = None;
            self.ime_enabled = false;
        }
        let mut changed = false;
        if focused && enabled {
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: false,
                    },
                );
            });
            changed = self.process_events(ui, text, row_height);
        }
        let reveal = std::mem::take(&mut self.reveal);
        if reveal {
            let head = self.state.selection.head;
            let row = self.state.line_for_offset(head);
            let x = self.visual_column(text, head) as f32 * char_width;
            let anchor = self.state.selection.anchor;
            let anchor_x = if self.state.line_for_offset(anchor) == row {
                self.visual_column(text, anchor) as f32 * char_width
            } else {
                x
            };
            self.reveal_position(
                x,
                anchor_x,
                row as f32 * row_height,
                row_height,
                gutter_width,
            );
        }
        let scroll_id = ui.make_persistent_id("document_editor_scroll");
        if std::mem::take(&mut self.reset_scroll) {
            egui::scroll_area::State::default().store(ui.ctx(), scroll_id);
        }
        let mut drag_scroll = Vec2::ZERO;
        let output = egui::ScrollArea::both()
            .id_salt("document_editor_scroll")
            .auto_shrink([false, false])
            .drag_to_scroll(false)
            .animated(false)
            .scroll_offset(self.scroll)
            .show_viewport(ui, |ui, viewport| {
                let origin = ui.cursor().min;
                let screen = Rect::from_min_size(origin + viewport.min.to_vec2(), viewport.size())
                    .intersect(ui.clip_rect());
                let text_clip = Rect::from_min_max(
                    egui::pos2(screen.left() + gutter_width, screen.top()),
                    screen.max,
                );
                let width = gutter_width
                    + TEXT_PADDING
                    + self.state.max_line_columns() as f32 * char_width
                    + TEXT_PADDING;
                ui.set_min_size(egui::vec2(
                    width.max(viewport.width()),
                    (self.state.line_count() as f32 * row_height).max(viewport.height()),
                ));
                self.rendered_rows = visible_rows(viewport, row_height, self.state.line_count());
                self.highlighter.retain_lines(self.rendered_rows.clone());
                let mut lines = std::mem::take(&mut self.line_buffer);
                lines.extend(self.rendered_rows.clone().map(|row| {
                    let range = self.state.line_range(text, row);
                    VisibleLine {
                        row,
                        origin: origin
                            + egui::vec2(gutter_width + TEXT_PADDING, row as f32 * row_height),
                        galley: self.highlighter.layout_line(
                            ui,
                            row,
                            &text[range],
                            self.state.string_depth(row),
                        ),
                    }
                }));
                // Monospace fallback glyphs can be wider than the ASCII extent estimate.
                let measured_width = lines
                    .iter()
                    .map(|line| line.galley.size().x)
                    .fold(0.0_f32, f32::max)
                    + gutter_width
                    + 2.0 * TEXT_PADDING;
                ui.set_min_width(width.max(measured_width).max(viewport.width()));
                if enabled {
                    drag_scroll =
                        self.pointer_input(ui, text, &response, &lines, text_clip, row_height);
                } else {
                    self.drag = None;
                }
                if enabled && response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                    ui.output_mut(|output| output.mutable_text_under_cursor = true);
                }
                let painter = ui.painter().with_clip_rect(text_clip);
                let selection = self.state.selection.range();
                for line in &lines {
                    let range = self.state.line_range(text, line.row);
                    if !selection.is_empty()
                        && selection.start <= range.end
                        && selection.end > range.start
                    {
                        let start = selection.start.max(range.start).min(range.end);
                        let end = selection.end.min(range.end);
                        let left = line.cursor_x(text, range.start, start);
                        let right = if selection.end > range.end {
                            (origin.x + width).max(text_clip.right())
                        } else {
                            line.cursor_x(text, range.start, end)
                        };
                        painter.rect_filled(
                            Rect::from_min_max(
                                egui::pos2(left, line.origin.y),
                                egui::pos2(right.max(left + 2.0), line.origin.y + row_height),
                            ),
                            0.0,
                            ui.visuals().selection.bg_fill,
                        );
                    }
                    painter.galley(line.origin, line.galley.clone(), theme::TEXT);
                }
                let caret_byte = self
                    .composition
                    .as_ref()
                    .map_or(self.state.selection.head, |composition| {
                        composition.selection.range().start
                    });
                let caret_row = self.state.line_for_offset(caret_byte);
                if let Some(line) = lines.iter().find(|line| line.row == caret_row) {
                    let x = line.cursor_x(text, self.state.line_start(caret_row), caret_byte);
                    let mut caret = Rect::from_min_size(
                        egui::pos2(x, line.origin.y),
                        egui::vec2(1.5, row_height),
                    );
                    if enabled && response.has_focus() {
                        if let Some(composition) = &self.composition {
                            let preedit = painter.layout_no_wrap(
                                composition.text.clone(),
                                font.clone(),
                                theme::TEXT,
                            );
                            let rect = Rect::from_min_size(caret.min, preedit.size());
                            painter.rect_filled(rect, 0.0, theme::BG);
                            painter.galley(rect.min, preedit, theme::TEXT);
                            painter.hline(
                                rect.x_range(),
                                rect.bottom(),
                                egui::Stroke::new(1.0_f32, theme::ACCENT),
                            );
                            caret = caret.translate(egui::vec2(rect.width(), 0.0));
                        }
                        painter.line_segment(
                            [caret.left_top(), caret.left_bottom()],
                            egui::Stroke::new(1.5_f32, theme::ACCENT),
                        );
                        let transform = ui
                            .ctx()
                            .layer_transform_to_global(ui.layer_id())
                            .unwrap_or_default();
                        ui.output_mut(|output| {
                            output.ime = Some(egui::output::IMEOutput {
                                rect: transform * text_clip,
                                cursor_rect: transform * caret,
                            })
                        });
                    }
                    if reveal {
                        let before = self.scroll;
                        let anchor = self.state.selection.anchor;
                        let anchor_x = if self.state.line_for_offset(anchor) == caret_row {
                            line.cursor_x(text, self.state.line_start(caret_row), anchor)
                        } else {
                            x
                        };
                        self.reveal_position(
                            x - origin.x - gutter_width - TEXT_PADDING,
                            anchor_x - origin.x - gutter_width - TEXT_PADDING,
                            caret_row as f32 * row_height,
                            row_height,
                            gutter_width,
                        );
                        // Refine the horizontal estimate with the actual visible galley.
                        drag_scroll += self.scroll - before;
                    }
                }
                let gutter =
                    Rect::from_min_max(screen.min, egui::pos2(text_clip.left(), screen.bottom()));
                let gutter_painter = ui.painter().with_clip_rect(gutter);
                gutter_painter.rect_filled(gutter, 0.0, theme::BG);
                gutter_painter.vline(
                    gutter.right() - 4.0,
                    gutter.y_range(),
                    egui::Stroke::new(1.0_f32, theme::BORDER),
                );
                for line in &lines {
                    gutter_painter.text(
                        egui::pos2(gutter.right() - 12.0, line.origin.y),
                        egui::Align2::RIGHT_TOP,
                        (line.row + 1).to_string(),
                        font.clone(),
                        theme::MUTED,
                    );
                }
                lines.clear();
                self.line_buffer = lines;
            });
        self.viewport_size = output.inner_rect.size();
        self.scroll = (output.state.offset + drag_scroll)
            .max(Vec2::ZERO)
            .min((output.content_size - self.viewport_size).max(Vec2::ZERO));
        response.rect = output.inner_rect;
        response.interact_rect = output.inner_rect;
        if drag_scroll != Vec2::ZERO {
            ui.ctx().request_repaint();
        }
        if changed {
            response.mark_changed();
        }
        response
    }

    fn reveal_position(
        &mut self,
        x: f32,
        anchor_x: f32,
        y: f32,
        row_height: f32,
        gutter_width: f32,
    ) {
        let width = (self.viewport_size.x - gutter_width - 2.0 * TEXT_PADDING).max(1.0);
        let height = self.viewport_size.y.max(row_height);
        let (left, right) = if (x - anchor_x).abs() + 2.0 <= width {
            (x.min(anchor_x), x.max(anchor_x) + 2.0)
        } else {
            (x, x + 2.0)
        };
        if left < self.scroll.x {
            self.scroll.x = left;
        } else if right > self.scroll.x + width {
            self.scroll.x = right - width;
        }
        if y < self.scroll.y {
            self.scroll.y = y;
        } else if y + row_height > self.scroll.y + height {
            self.scroll.y = y + row_height - height;
        }
        self.scroll = self.scroll.max(Vec2::ZERO);
    }
}

impl VisibleLine {
    fn cursor_x(&self, text: &str, line_start: usize, byte: usize) -> f32 {
        let column = text[line_start..byte].chars().count();
        self.origin.x
            + self
                .galley
                .pos_from_ccursor(egui::text::CCursor::new(column))
                .left()
    }
}

fn visible_rows(viewport: Rect, row_height: f32, line_count: usize) -> Range<usize> {
    let start = (viewport.top().max(0.0) / row_height).floor() as usize;
    let end = (viewport.bottom().max(0.0) / row_height).ceil() as usize;
    start.saturating_sub(OVERSCAN).min(line_count)..end.saturating_add(OVERSCAN).min(line_count)
}
