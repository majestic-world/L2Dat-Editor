use std::ops::Range;

use regex::{NoExpand, Regex, RegexBuilder};

#[derive(Default)]
pub struct Search {
    query: String,
    pattern: Option<Regex>,
    matches: Vec<Range<usize>>,
    current: Option<usize>,
}

impl Search {
    pub fn refresh(&mut self, text: &str, query: &str) -> Result<(), regex::Error> {
        self.matches.clear();
        self.current = None;
        if query != self.query {
            self.pattern = if query.is_empty() {
                None
            } else {
                Some(
                    RegexBuilder::new(&regex::escape(query))
                        .case_insensitive(true)
                        .build()?,
                )
            };
            self.query = query.to_owned();
        }
        if let Some(pattern) = &self.pattern {
            self.matches
                .extend(pattern.find_iter(text).map(|found| found.range()));
        }
        Ok(())
    }

    pub fn navigate(&mut self, forward: bool, selection: Range<usize>) -> Option<Range<usize>> {
        if self.matches.is_empty() {
            return None;
        }
        let current = self
            .current
            .filter(|index| self.matches[*index] == selection);
        let index = match (current, forward) {
            (Some(index), true) => (index + 1) % self.matches.len(),
            (Some(index), false) => (index + self.matches.len() - 1) % self.matches.len(),
            (None, true) => self
                .matches
                .iter()
                .position(|found| found.start >= selection.end)
                .unwrap_or(0),
            (None, false) => self
                .matches
                .iter()
                .rposition(|found| found.start < selection.start)
                .unwrap_or(self.matches.len() - 1),
        };
        self.current = Some(index);
        Some(self.matches[index].clone())
    }

    pub fn summary(&self) -> String {
        match self.current {
            Some(index) => format!("{} de {}", index + 1, self.matches.len()),
            None => format!("{} ocorrência(s)", self.matches.len()),
        }
    }

    pub fn replace_all(&self, text: &str, replacement: &str) -> Option<(String, usize)> {
        if self.matches.is_empty() {
            return None;
        }
        self.pattern.as_ref().map(|pattern| {
            (
                pattern
                    .replace_all(text, NoExpand(replacement))
                    .into_owned(),
                self.matches.len(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Search;

    #[test]
    fn unicode_literal_search_wraps_in_both_directions_from_selection() {
        let text = "[Á] x [á]";
        let mut search = Search::default();
        search.refresh(text, "[á]").unwrap();
        assert_eq!(search.navigate(true, 0..0), Some(0..4));
        assert_eq!(search.navigate(true, 0..4), Some(7..11));
        assert_eq!(search.navigate(true, 7..11), Some(0..4));
        assert_eq!(search.navigate(false, 0..4), Some(7..11));
        assert_eq!(search.navigate(false, 6..6), Some(0..4));
    }

    #[test]
    fn replacement_is_literal_and_edits_invalidate_old_matches() {
        let mut search = Search::default();
        search.refresh("Name=TEST;name=test", "test").unwrap();
        assert_eq!(
            search.replace_all("Name=TEST;name=test", "$1"),
            Some(("Name=$1;name=$1".into(), 2))
        );
        search.refresh("Name=changed", "test").unwrap();
        assert_eq!(search.navigate(true, 0..0), None);
        assert_eq!(search.replace_all("Name=changed", "x"), None);
    }
}
