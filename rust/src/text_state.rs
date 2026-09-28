use std::collections::{BTreeMap, VecDeque};
use std::ops::Range;

const HISTORY_LIMIT: usize = 100;
const HISTORY_BYTE_LIMIT: usize = 128 * 1024 * 1024;
const TAB_WIDTH: usize = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}

impl Selection {
    pub fn range(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
}

#[derive(Clone, Copy, Default)]
struct LineMetrics {
    columns: usize,
    opens: usize,
    closes: usize,
}

impl LineMetrics {
    fn measure(text: &str) -> Self {
        let mut metrics = Self::default();
        for ch in text.chars() {
            metrics.columns += if ch == '\t' { TAB_WIDTH } else { 1 };
            match ch {
                '[' => metrics.opens += 1,
                ']' if metrics.opens > 0 => metrics.opens -= 1,
                ']' => metrics.closes += 1,
                _ => {}
            }
        }
        metrics
    }

    fn end_depth(self, initial: usize) -> usize {
        initial.saturating_sub(self.closes) + self.opens
    }
}

#[derive(Clone, Copy, Default)]
struct Line {
    start: usize,
    depth: usize,
    metrics: LineMetrics,
}

struct Edit {
    start: usize,
    removed: String,
    inserted: String,
    before: Selection,
    after: Selection,
}

impl Edit {
    fn bytes(&self) -> usize {
        self.removed.len() + self.inserted.len()
    }
}

/// Indexes a caller-owned document. Call `reset` whenever that document is replaced
/// externally; subsequent content changes must go through this state.
#[derive(Default)]
pub struct TextState {
    pub selection: Selection,
    lines: Vec<Line>,
    text_len: usize,
    column_counts: BTreeMap<usize, usize>,
    undo: VecDeque<Edit>,
    redo: Vec<Edit>,
    history_bytes: usize,
}

impl TextState {
    pub fn reset(&mut self, text: &str) {
        self.selection = Selection::default();
        self.lines.clear();
        self.column_counts.clear();
        self.undo.clear();
        self.redo.clear();
        self.history_bytes = 0;
        self.text_len = text.len();

        let mut start = 0;
        let mut depth = 0;
        for content in text.split('\n') {
            let metrics = LineMetrics::measure(content);
            self.lines.push(Line {
                start,
                depth,
                metrics,
            });
            add_count(&mut self.column_counts, metrics.columns);
            start += content.len() + 1;
            depth = metrics.end_depth(depth);
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len().max(1)
    }

    pub fn line_start(&self, row: usize) -> usize {
        assert!(row < self.line_count(), "line is outside the document");
        self.lines.get(row).map_or(0, |line| line.start)
    }

    pub fn line_range(&self, text: &str, row: usize) -> Range<usize> {
        self.assert_document(text);
        let start = self.line_start(row);
        let end = self
            .lines
            .get(row + 1)
            .map_or(text.len(), |next| next.start - 1);
        start..end
    }

    pub fn line_for_offset(&self, byte: usize) -> usize {
        assert!(byte <= self.text_len, "offset is outside the document");
        self.lines
            .partition_point(|line| line.start <= byte)
            .saturating_sub(1)
    }

    pub fn string_depth(&self, row: usize) -> usize {
        assert!(row < self.line_count(), "line is outside the document");
        self.lines.get(row).map_or(0, |line| line.depth)
    }

    /// Maximum scalar width with each tab occupying four columns, as in egui.
    pub fn max_line_columns(&self) -> usize {
        self.column_counts
            .last_key_value()
            .map_or(0, |(&size, _)| size)
    }

    pub fn byte_at_column(&self, text: &str, row: usize, column: usize) -> usize {
        let range = self.line_range(text, row);
        text[range.clone()]
            .char_indices()
            .nth(column)
            .map_or(range.end, |(offset, _)| range.start + offset)
    }

    pub fn replace(&mut self, text: &mut String, range: Range<usize>, inserted: &str) -> bool {
        self.assert_document(text);
        let removed = text
            .get(range.clone())
            .expect("edit range must be ordered and on UTF-8 boundaries");
        let after = Selection {
            anchor: range.start + inserted.len(),
            head: range.start + inserted.len(),
        };
        if removed == inserted {
            self.selection = after;
            return false;
        }

        let edit = Edit {
            start: range.start,
            removed: removed.to_owned(),
            inserted: inserted.to_owned(),
            before: self.selection,
            after,
        };
        self.apply_edit(text, range, inserted);
        self.selection = after;
        self.history_bytes -= self.redo.iter().map(Edit::bytes).sum::<usize>();
        self.redo.clear();
        self.history_bytes += edit.bytes();
        self.undo.push_back(edit);
        self.trim_history(HISTORY_BYTE_LIMIT);
        true
    }

    pub fn replace_selection(&mut self, text: &mut String, inserted: &str) -> bool {
        self.replace(text, self.selection.range(), inserted)
    }

    pub fn undo(&mut self, text: &mut String) -> bool {
        self.assert_document(text);
        let Some(edit) = self.undo.pop_back() else {
            return false;
        };
        self.apply_edit(
            text,
            edit.start..edit.start + edit.inserted.len(),
            &edit.removed,
        );
        self.selection = edit.before;
        self.redo.push(edit);
        true
    }

    pub fn redo(&mut self, text: &mut String) -> bool {
        self.assert_document(text);
        let Some(edit) = self.redo.pop() else {
            return false;
        };
        self.apply_edit(
            text,
            edit.start..edit.start + edit.removed.len(),
            &edit.inserted,
        );
        self.selection = edit.after;
        self.undo.push_back(edit);
        true
    }

    fn trim_history(&mut self, byte_limit: usize) {
        // Keep the latest edit undoable even when one large paste exceeds the
        // budget. Older bulk replacements must not accumulate without bound.
        while self.undo.len() > 1
            && (self.undo.len() > HISTORY_LIMIT || self.history_bytes > byte_limit)
        {
            if let Some(edit) = self.undo.pop_front() {
                self.history_bytes -= edit.bytes();
            }
        }
    }

    fn assert_document(&self, text: &str) {
        assert_eq!(
            self.text_len,
            text.len(),
            "document changed outside TextState; call reset first"
        );
    }

    fn apply_edit(&mut self, text: &mut String, range: Range<usize>, inserted: &str) {
        // A default state represents an empty document without allocating an index.
        if self.lines.is_empty() {
            self.lines.push(Line::default());
            add_count(&mut self.column_counts, 0);
        }
        let first = self.line_for_offset(range.start);
        let last = self.line_for_offset(range.end);
        let region_start = self.lines[first].start;
        let has_suffix = last + 1 < self.lines.len();
        let old_region_end = self
            .lines
            .get(last + 1)
            .map_or(text.len(), |line| line.start);
        let removed_len = range.len();
        let region_end = old_region_end - removed_len + inserted.len();
        let initial_depth = self.lines[first].depth;
        let single_line = first == last && !inserted.contains('\n');

        let inline_metrics = if single_line {
            let previous = self.lines[first].metrics;
            let removed = LineMetrics::measure(&text[range.clone()]);
            let added = LineMetrics::measure(inserted);
            // Ordinary typing only inspects the edit, even if that physical line
            // spans the entire document. Equal bracket summaries leave the
            // containing line's bracket transform unchanged.
            if removed.opens == added.opens && removed.closes == added.closes {
                Some(LineMetrics {
                    columns: previous.columns - removed.columns + added.columns,
                    ..previous
                })
            } else {
                None
            }
        } else {
            None
        };

        text.replace_range(range, inserted);
        self.text_len = text.len();
        let content_end = region_end - usize::from(has_suffix);
        let mut depth = initial_depth;
        let suffix_row;
        if single_line {
            let metrics = inline_metrics
                .unwrap_or_else(|| LineMetrics::measure(&text[region_start..content_end]));
            let previous = self.lines[first].metrics;
            change_count(&mut self.column_counts, previous.columns, metrics.columns);
            self.lines[first].metrics = metrics;
            depth = metrics.end_depth(depth);
            suffix_row = first + 1;
        } else {
            for line in &self.lines[first..=last] {
                remove_count(&mut self.column_counts, line.metrics.columns);
            }
            let mut start = region_start;
            let replacements = text[region_start..content_end].split('\n').map(|content| {
                let metrics = LineMetrics::measure(content);
                let line = Line {
                    start,
                    depth,
                    metrics,
                };
                start += content.len() + 1;
                depth = metrics.end_depth(depth);
                add_count(&mut self.column_counts, metrics.columns);
                line
            });
            let old_count = self.lines.len();
            // Consuming the splice indexes only touched physical lines. Retained
            // lines keep their summaries, so propagation never rereads their text.
            self.lines.splice(first..=last, replacements).for_each(drop);
            suffix_row = last + 1 + self.lines.len() - old_count;
        }

        let shifted = removed_len != inserted.len();
        let mut propagate = true;
        for line in &mut self.lines[suffix_row..] {
            if shifted {
                line.start = line.start - removed_len + inserted.len();
            }
            if propagate {
                if line.depth == depth {
                    propagate = false;
                    if !shifted {
                        break;
                    }
                } else {
                    line.depth = depth;
                    depth = line.metrics.end_depth(depth);
                }
            }
        }
    }
}

fn add_count(counts: &mut BTreeMap<usize, usize>, size: usize) {
    *counts.entry(size).or_default() += 1;
}

fn remove_count(counts: &mut BTreeMap<usize, usize>, size: usize) {
    let count = counts
        .get_mut(&size)
        .expect("indexed line width is present");
    *count -= 1;
    if *count == 0 {
        counts.remove(&size);
    }
}

fn change_count(counts: &mut BTreeMap<usize, usize>, before: usize, after: usize) {
    if before != after {
        remove_count(counts, before);
        add_count(counts, after);
    }
}

#[cfg(test)]
mod tests {
    use super::{HISTORY_LIMIT, Selection, TextState};

    fn assert_index(state: &TextState, text: &str) {
        let contents: Vec<_> = text.split('\n').collect();
        assert_eq!(state.line_count(), contents.len());
        let mut offset = 0;
        let mut depth = 0;
        let mut max_columns = 0;
        for (row, content) in contents.iter().enumerate() {
            assert_eq!(state.line_start(row), offset);
            assert_eq!(state.line_range(text, row), offset..offset + content.len());
            assert_eq!(state.string_depth(row), depth, "line {row} of {text:?}");
            let mut columns = 0;
            for (column, (byte, ch)) in content.char_indices().enumerate() {
                assert_eq!(state.line_for_offset(offset + byte), row);
                assert_eq!(state.byte_at_column(text, row, column), offset + byte);
                columns += if ch == '\t' { 4 } else { 1 };
                match ch {
                    '[' => depth += 1,
                    ']' => depth = depth.saturating_sub(1),
                    _ => {}
                }
            }
            assert_eq!(state.line_for_offset(offset + content.len()), row);
            assert_eq!(
                state.byte_at_column(text, row, usize::MAX),
                offset + content.len()
            );
            max_columns = max_columns.max(columns);
            offset += content.len() + 1;
        }
        assert_eq!(state.max_line_columns(), max_columns);
        assert_eq!(state.line_for_offset(text.len()), contents.len() - 1);
    }

    #[test]
    fn cross_line_unicode_replacement_preserves_byte_mappings() {
        let mut text = "αβ\n猫x\n🙂!\n".to_owned();
        let mut state = TextState::default();
        state.reset(&text);
        assert_index(&state, &text);
        state.selection = Selection { anchor: 8, head: 2 };
        assert!(state.replace_selection(&mut text, "é\n界"));
        assert_eq!(text, "αé\n界x\n🙂!\n");
        assert_eq!(state.selection, Selection { anchor: 8, head: 8 });
        assert_index(&state, &text);
        assert!(state.undo(&mut text));
        assert_eq!(text, "αβ\n猫x\n🙂!\n");
        assert_eq!(state.selection, Selection { anchor: 8, head: 2 });
        assert_index(&state, &text);
        assert!(state.redo(&mut text));
        assert_eq!(text, "αé\n界x\n🙂!\n");
        assert_index(&state, &text);
    }

    #[test]
    fn inserted_deleted_and_trailing_lines_include_empty_eof() {
        let mut text = String::new();
        let mut state = TextState::default();
        assert_index(&state, &text);
        assert!(state.replace(&mut text, 0..0, "a\n\nβ\n"));
        assert_index(&state, &text);
        assert_eq!(state.line_count(), 4);
        assert!(state.replace(&mut text, 1..3, ""));
        assert_eq!(text, "aβ\n");
        assert_index(&state, &text);
        assert!(state.replace(&mut text, 3..4, ""));
        assert_eq!(text, "aβ");
        assert_index(&state, &text);
        assert!(state.replace(&mut text, 0..0, "\n"));
        assert_eq!(text, "\naβ");
        assert_index(&state, &text);
        let end = text.len();
        assert!(state.replace(&mut text, end..end, "\n\n"));
        assert_index(&state, &text);
        assert_eq!(state.line_count(), 4);
        let end = text.len();
        assert!(state.replace(&mut text, 0..end, ""));
        assert_eq!(text, "");
        assert_eq!(state.line_count(), 1);
        assert_index(&state, &text);
        assert!(state.undo(&mut text));
        assert_eq!(text, "\naβ\n\n");
        assert_index(&state, &text);
    }

    #[test]
    fn bracket_depth_propagates_and_recovers_after_unmatched_closers() {
        let mut text = "]outer[\n[inner\n]]tail\nend\n".to_owned();
        let mut state = TextState::default();
        state.reset(&text);
        assert_index(&state, &text);
        assert_eq!(
            (
                state.string_depth(1),
                state.string_depth(2),
                state.string_depth(3)
            ),
            (1, 2, 0)
        );
        assert!(state.replace(&mut text, 0..1, "[["));
        assert_index(&state, &text);
        assert_eq!(
            (
                state.string_depth(1),
                state.string_depth(2),
                state.string_depth(3)
            ),
            (3, 4, 2)
        );
        let row = state.line_start(2);
        assert!(state.replace(&mut text, row..row + 2, "]]]]]"));
        assert_index(&state, &text);
        assert_eq!(state.string_depth(3), 0);
        assert!(state.replace(&mut text, 0..1, "x"));
        assert_index(&state, &text);
        assert_eq!(state.string_depth(1), 2);
        assert_eq!(state.string_depth(3), 0);
        assert!(state.undo(&mut text));
        assert_index(&state, &text);
    }

    #[test]
    fn balanced_inline_brackets_preserve_depth_and_tab_widths_update() {
        let mut text = "[a[]\n\t猫\tx\n]end".to_owned();
        let mut state = TextState::default();
        state.reset(&text);
        assert_eq!(state.max_line_columns(), 10);
        assert!(state.replace(&mut text, 2..4, "β[]γ"));
        assert_index(&state, &text);
        assert_eq!(state.string_depth(1), 1);
        let row = state.line_start(1);
        assert!(state.replace(&mut text, row..row, "abcd"));
        assert_index(&state, &text);
        assert_eq!(state.max_line_columns(), 14);
        let end = state.line_range(&text, 1).end;
        assert!(state.replace(&mut text, row..end, "z"));
        assert_index(&state, &text);
        assert_eq!(state.max_line_columns(), 6);
        assert!(state.undo(&mut text));
        assert_index(&state, &text);
        assert_eq!(state.max_line_columns(), 14);
    }

    #[test]
    fn undo_redo_restore_directional_selection_and_new_edits_clear_redo() {
        let mut text = "zero\none\ntwo".to_owned();
        let mut state = TextState::default();
        state.reset(&text);
        let original = Selection { anchor: 8, head: 5 };
        state.selection = original;
        assert!(state.replace_selection(&mut text, "猫\n🙂"));
        let edited = "zero\n猫\n🙂\ntwo";
        assert_eq!(text, edited);
        let after = Selection {
            anchor: 13,
            head: 13,
        };
        assert_eq!(state.selection, after);
        state.selection = Selection { anchor: 0, head: 4 };
        assert!(state.undo(&mut text));
        assert_eq!(text, "zero\none\ntwo");
        assert_eq!(state.selection, original);
        assert!(state.redo(&mut text));
        assert_eq!(text, edited);
        assert_eq!(state.selection, after);
        assert!(state.undo(&mut text));
        assert!(state.replace_selection(&mut text, "new"));
        assert_eq!(text, "zero\nnew\ntwo");
        assert!(!state.redo(&mut text));
        assert!(state.undo(&mut text));
        assert_eq!(text, "zero\none\ntwo");
        assert_eq!(state.selection, original);
        assert_index(&state, &text);
    }

    #[test]
    fn identical_replacement_collapses_selection_without_consuming_history() {
        let mut text = "猫".to_owned();
        let mut state = TextState::default();
        state.reset(&text);
        assert!(state.replace(&mut text, 3..3, "x"));
        assert!(state.undo(&mut text));
        state.selection = Selection { anchor: 3, head: 0 };
        assert!(!state.replace_selection(&mut text, "猫"));
        assert_eq!(state.selection, Selection { anchor: 3, head: 3 });
        assert!(state.redo(&mut text));
        assert_eq!(text, "猫x");
        assert!(state.undo(&mut text));
        assert!(!state.undo(&mut text));
    }

    #[test]
    fn history_is_bounded_and_reset_isolates_documents() {
        let mut text = String::new();
        let mut state = TextState::default();
        for _ in 0..HISTORY_LIMIT + 3 {
            let end = text.len();
            assert!(state.replace(&mut text, end..end, "é"));
        }
        for _ in 0..HISTORY_LIMIT {
            assert!(state.undo(&mut text));
            assert_index(&state, &text);
        }
        assert_eq!(text, "ééé");
        assert!(!state.undo(&mut text));
        assert!(state.redo(&mut text));
        text = "other[\n文\n".to_owned();
        state.reset(&text);
        assert_eq!(state.selection, Selection::default());
        assert!(!state.undo(&mut text));
        assert!(!state.redo(&mut text));
        assert_index(&state, &text);
        assert!(state.replace(&mut text, 0..5, "new"));
        assert!(state.undo(&mut text));
        assert_eq!(text, "other[\n文\n");
        assert_index(&state, &text);
    }

    #[test]
    fn independent_states_do_not_share_history_or_indexes() {
        let mut first_text = "first\n[one]".to_owned();
        let mut second_text = "[second\n猫]".to_owned();
        let mut first = TextState::default();
        let mut second = TextState::default();
        first.reset(&first_text);
        second.reset(&second_text);
        assert!(first.replace(&mut first_text, 0..5, "changed\nfirst"));
        assert!(second.replace(&mut second_text, 0..1, ""));
        assert!(first.undo(&mut first_text));
        assert_eq!(first_text, "first\n[one]");
        assert_eq!(second_text, "second\n猫]");
        assert_index(&first, &first_text);
        assert_index(&second, &second_text);
        assert!(second.undo(&mut second_text));
        assert_eq!(second_text, "[second\n猫]");
        assert_index(&second, &second_text);
    }

    #[test]
    fn deterministic_edit_sequences_match_physical_text() {
        let mut text = "[α\n\t猫]\n\n🙂tail".to_owned();
        let mut state = TextState::default();
        state.reset(&text);
        let replacements = ["x", "\n", "猫\n[", "", "]", "\t🙂", "[]", "\n\n"];
        let mut seed = 0x91e1_0da5_u32;
        for step in 0..160 {
            let mut boundaries: Vec<_> = text.char_indices().map(|(byte, _)| byte).collect();
            boundaries.push(text.len());
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let a = boundaries[seed as usize % boundaries.len()];
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let b = boundaries[seed as usize % boundaries.len()];
            state.selection = Selection { anchor: b, head: a };
            let before_selection = state.selection;
            let before = text.clone();
            let replacement = replacements[step % replacements.len()];
            let changed = state.replace_selection(&mut text, replacement);
            assert_index(&state, &text);
            if changed {
                let after = text.clone();
                let after_selection = state.selection;
                assert!(state.undo(&mut text));
                assert_eq!(text, before);
                assert_eq!(state.selection, before_selection);
                assert_index(&state, &text);
                assert!(state.redo(&mut text));
                assert_eq!(text, after);
                assert_eq!(state.selection, after_selection);
                assert_index(&state, &text);
            }
        }
    }

    #[test]
    fn bulk_history_budget_keeps_latest_edit_and_tracks_redo() {
        let mut text = "first".to_owned();
        let mut state = TextState::default();
        state.reset(&text);
        state.replace(&mut text, 0..5, "second");
        state.replace(&mut text, 0..6, "third");
        state.replace(&mut text, 0..5, "fourth");
        state.trim_history(22);
        assert!(state.undo(&mut text));
        assert_eq!(text, "third");
        assert!(state.undo(&mut text));
        assert_eq!(text, "second");
        assert!(!state.undo(&mut text));
        assert!(state.redo(&mut text));
        assert_eq!(text, "third");
        state.replace(&mut text, 0..5, "replacement");
        assert!(!state.redo(&mut text));
        state.trim_history(1);
        assert!(state.undo(&mut text));
        assert_eq!(text, "third");
        assert!(!state.undo(&mut text));
        assert!(state.redo(&mut text));
        assert_eq!(text, "replacement");
    }

    #[test]
    #[should_panic(expected = "UTF-8 boundaries")]
    fn edits_reject_offsets_inside_utf8_characters() {
        let mut text = "猫".to_owned();
        let mut state = TextState::default();
        state.reset(&text);
        state.replace(&mut text, 1..2, "x");
    }
}
