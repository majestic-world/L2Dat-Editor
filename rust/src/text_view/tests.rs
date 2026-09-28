use eframe::egui::{self, Event, Key, Modifiers};

use super::TextEditor;
use crate::text_state::Selection;

fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

fn frame(
    ctx: &egui::Context,
    editor: &mut TextEditor,
    text: &mut String,
    events: Vec<Event>,
) -> (bool, egui::FullOutput) {
    let mut changed = false;
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(640.0, 300.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                changed = editor.show(ui, text, true).changed();
            });
        },
    );
    (changed, output)
}

fn paints_text(output: &egui::FullOutput, needle: &str) -> bool {
    output.shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::Text(shape) => {
            shape.galley.text().contains(needle)
                && clipped
                    .clip_rect
                    .intersects(shape.galley.rect.translate(shape.pos.to_vec2()))
        }
        _ => false,
    })
}

fn vertex_count(ctx: &egui::Context, output: egui::FullOutput) -> usize {
    ctx.tessellate(output.shapes, output.pixels_per_point)
        .into_iter()
        .map(|primitive| match primitive.primitive {
            egui::epaint::Primitive::Mesh(mesh) => mesh.vertices.len(),
            egui::epaint::Primitive::Callback(_) => 0,
        })
        .sum()
}

#[test]
fn offscreen_selection_paints_target_and_edits_global_document_with_undo() {
    let ctx = egui::Context::default();
    let row = format!("npc_begin id=42 name=[{}] npc_end\n", "abcdefgh".repeat(12));
    let mut text = row.repeat(20_000);
    let target = 15_000 * row.len();
    text.insert_str(target, "far_target_한글_猫 ");
    let original = text.clone();
    let mut editor = TextEditor::default();
    editor.reset(&text);
    frame(&ctx, &mut editor, &mut text, vec![]);
    editor.select_range(target..target + "far_target_한글_猫".len());
    frame(&ctx, &mut editor, &mut text, vec![]);
    let (_, output) = frame(&ctx, &mut editor, &mut text, vec![]);
    assert!(paints_text(&output, "far_target_한글_猫"));
    assert!(editor.rendered_rows.contains(&15_000));
    assert!(editor.rendered_rows.len() < 24);
    assert!(vertex_count(&ctx, output) < 20_000);
    let (changed, _) = frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![Event::Text("替換".into())],
    );
    assert!(changed);
    assert_eq!(&text[target..target + "替換".len()], "替換");
    assert_eq!(&text[..target], &original[..target]);
    assert_eq!(
        &text[target + "替換".len()..],
        &original[target + "far_target_한글_猫".len()..]
    );
    let (changed, _) = frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![key(Key::Z, Modifiers::CTRL)],
    );
    assert!(changed);
    assert_eq!(text, original);
    assert_eq!(
        editor.state.selection.range(),
        target..target + "far_target_한글_猫".len()
    );
    let (changed, _) = frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![key(Key::Y, Modifiers::CTRL)],
    );
    assert!(changed);
    assert_eq!(&text[target..target + "替換".len()], "替換");
}

#[test]
fn search_selection_is_fully_visible_after_horizontal_scrolling() {
    let ctx = egui::Context::default();
    let mut text = format!("name=[ação 猫 한글] {}", "abcdef ".repeat(100));
    let mut editor = TextEditor::default();
    editor.reset(&text);
    editor.select_range(text.len()..text.len());
    frame(&ctx, &mut editor, &mut text, vec![]);
    frame(&ctx, &mut editor, &mut text, vec![]);
    let start = text.find("ação").unwrap();
    let end = start + "ação 猫 한글".len();
    editor.select_range(start..end);
    frame(&ctx, &mut editor, &mut text, vec![]);
    let (_, output) = frame(&ctx, &mut editor, &mut text, vec![]);
    let (clip, shape) = output
        .shapes
        .iter()
        .find_map(|clipped| {
            if let egui::Shape::Text(shape) = &clipped.shape {
                if shape.galley.text() == text {
                    return Some((clipped.clip_rect, shape));
                }
            }
            None
        })
        .expect("The selected line must be painted");
    for byte in [start, end] {
        let cursor = egui::text::CCursor::new(text[..byte].chars().count());
        let position = shape.pos + shape.galley.pos_from_ccursor(cursor).center().to_vec2();
        assert!(
            clip.contains(position),
            "The entire search match must be visible"
        );
    }
}

#[test]
fn geometry_stays_bounded_when_scrolling_between_distant_lines() {
    let ctx = egui::Context::default();
    let row = format!("id=1 value=[{}]\n", "x".repeat(128));
    let mut text = row.repeat(20_000);
    let mut editor = TextEditor::default();
    editor.reset(&text);
    frame(&ctx, &mut editor, &mut text, vec![]);
    for line in [100, 19_500, 4_000, 19_999, 0] {
        editor.select_range(line * row.len()..line * row.len());
        frame(&ctx, &mut editor, &mut text, vec![]);
        let (_, output) = frame(&ctx, &mut editor, &mut text, vec![]);
        assert!(editor.rendered_rows.contains(&line));
        assert!(editor.rendered_rows.len() < 24);
        assert!(vertex_count(&ctx, output) < 20_000);
    }
}

#[test]
fn clipboard_and_focus_loss_preserve_selection_across_thousands_of_lines() {
    let ctx = egui::Context::default();
    let mut text = "αβ_猫 field=[value]\n".repeat(10_000);
    let original = text.clone();
    let mut editor = TextEditor::default();
    editor.reset(&text);
    let start = editor.state.line_start(11);
    let end = editor.state.line_start(9_000);
    editor.select_range(start..end);
    frame(&ctx, &mut editor, &mut text, vec![]);
    ctx.memory_mut(|memory| memory.surrender_focus(egui::Id::new("document_editor")));
    frame(&ctx, &mut editor, &mut text, vec![]);
    assert_eq!(editor.state.selection.range(), start..end);
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("document_editor")));
    let (changed, output) = frame(&ctx, &mut editor, &mut text, vec![Event::Copy]);
    assert!(!changed);
    assert!(output.platform_output.commands.iter().any(|command| {
        matches!(command, egui::OutputCommand::CopyText(copied) if copied == &original[start..end])
    }));
    let (changed, _) = frame(&ctx, &mut editor, &mut text, vec![Event::Cut]);
    assert!(changed);
    assert_eq!(text, format!("{}{}", &original[..start], &original[end..]));
    frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![key(Key::Z, Modifiers::CTRL)],
    );
    assert_eq!(text, original);
    assert_eq!(editor.state.selection.range(), start..end);
}

#[test]
fn keyboard_navigation_and_selection_cross_viewports_at_utf8_boundaries() {
    let ctx = egui::Context::default();
    let mut text = "αβ\t猫  word\n".repeat(500);
    let mut editor = TextEditor::default();
    editor.reset(&text);
    let start = editor.state.line_start(300) + "αβ\t".len();
    editor.select_range(start..start);
    frame(&ctx, &mut editor, &mut text, vec![]);
    frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![key(Key::ArrowRight, Modifiers::SHIFT)],
    );
    assert_eq!(editor.state.selection.range(), start..start + "猫".len());
    frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![key(Key::PageDown, Modifiers::SHIFT)],
    );
    assert!(editor.state.line_for_offset(editor.state.selection.head) > 300);
    assert_eq!(editor.state.selection.anchor, start);
    frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![key(Key::Home, Modifiers::CTRL | Modifiers::SHIFT)],
    );
    assert_eq!(
        editor.state.selection,
        Selection {
            anchor: start,
            head: 0
        }
    );
    frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![key(Key::End, Modifiers::CTRL)],
    );
    assert_eq!(
        editor.state.selection,
        Selection {
            anchor: text.len(),
            head: text.len()
        }
    );
    let (_, output) = frame(&ctx, &mut editor, &mut text, vec![]);
    assert!(editor.rendered_rows.contains(&500));
    assert!(vertex_count(&ctx, output) < 3_000);
}

#[test]
fn horizontal_navigation_reveals_the_end_of_an_unwrapped_line() {
    let ctx = egui::Context::default();
    let mut text = format!("{}far_right_target", "\tα".repeat(400));
    let mut editor = TextEditor::default();
    editor.reset(&text);
    editor.select_range(text.len()..text.len());
    frame(&ctx, &mut editor, &mut text, vec![]);
    let (_, output) = frame(&ctx, &mut editor, &mut text, vec![]);
    assert!(editor.scroll.x > 1_000.0);
    assert_eq!(editor.rendered_rows, 0..1);
    assert!(paints_text(&output, "far_right_target"));
    frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![key(Key::Home, Modifiers::NONE)],
    );
    assert_eq!(editor.state.selection.head, 0);
    assert_eq!(editor.scroll.x, 0.0);
}

#[test]
fn ime_preedit_is_transient_and_commit_is_one_undoable_edit() {
    let ctx = egui::Context::default();
    let mut text = "first old last".to_owned();
    let mut editor = TextEditor::default();
    editor.reset(&text);
    editor.select_range(6..9);
    frame(&ctx, &mut editor, &mut text, vec![]);
    let (changed, output) = frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![
            Event::Ime(egui::ImeEvent::Enabled),
            Event::Ime(egui::ImeEvent::Preedit("候補".into())),
            key(Key::Backspace, Modifiers::NONE),
        ],
    );
    assert!(!changed);
    assert_eq!(text, "first old last");
    assert!(paints_text(&output, "候補"));
    assert!(output.platform_output.ime.is_some());
    let (changed, _) = frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![Event::Ime(egui::ImeEvent::Commit("確定".into()))],
    );
    assert!(changed);
    assert_eq!(text, "first 確定 last");
    frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![key(Key::Z, Modifiers::CTRL)],
    );
    assert_eq!(text, "first old last");
    assert_eq!(editor.state.selection.range(), 6..9);
    frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![
            Event::Ime(egui::ImeEvent::Preedit("cancel".into())),
            Event::Ime(egui::ImeEvent::Disabled),
        ],
    );
    assert_eq!(text, "first old last");
    assert!(editor.composition.is_none());
}

#[test]
fn mouse_drag_autoscroll_keeps_the_original_global_anchor() {
    let ctx = egui::Context::default();
    let mut text = "αβ 猫 value=42\n".repeat(500);
    let mut editor = TextEditor::default();
    editor.reset(&text);
    frame(&ctx, &mut editor, &mut text, vec![]);
    let press = egui::pos2(100.0, 40.0);
    frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![
            Event::PointerMoved(press),
            Event::PointerButton {
                pos: press,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    let anchor = editor.state.selection.anchor;
    assert!(anchor > 0);
    let outside = egui::pos2(180.0, 450.0);
    frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![Event::PointerMoved(outside)],
    );
    for _ in 0..40 {
        frame(&ctx, &mut editor, &mut text, vec![]);
    }
    assert_eq!(editor.state.selection.anchor, anchor);
    assert!(editor.state.line_for_offset(editor.state.selection.head) > 20);
    assert!(editor.scroll.y > 300.0);
    frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![Event::PointerButton {
            pos: outside,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
    let range = editor.state.selection.range();
    assert!(text.is_char_boundary(range.start) && text.is_char_boundary(range.end));
    assert!(editor.drag.is_none());
}

#[test]
fn reset_discards_old_history_scroll_and_composition() {
    let ctx = egui::Context::default();
    let mut text = "line\n".repeat(1_000);
    let mut editor = TextEditor::default();
    editor.reset(&text);
    editor.select_range(text.len()..text.len());
    frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![Event::Text("edit".into())],
    );
    assert!(editor.scroll.y > 0.0);
    text = "new document".into();
    editor.reset(&text);
    editor.select_range(0..0);
    frame(
        &ctx,
        &mut editor,
        &mut text,
        vec![key(Key::Z, Modifiers::CTRL)],
    );
    assert_eq!(text, "new document");
    assert_eq!(editor.scroll, egui::Vec2::ZERO);
    assert_eq!(editor.state.selection, Selection::default());
}
