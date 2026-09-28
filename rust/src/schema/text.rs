use anyhow::{Result, bail, ensure};

use super::{MAX_COUNT, MAX_DEPTH, Node};

#[derive(Debug)]
enum Item<'a> {
    Field {
        name: &'a str,
        value: &'a str,
        used: bool,
    },
    Block {
        name: &'a str,
        record: Record<'a>,
        used: bool,
    },
    Literal {
        value: &'a str,
        used: bool,
    },
}

#[derive(Debug, Default)]
pub(super) struct Record<'a> {
    items: Vec<Item<'a>>,
}

impl<'a> Record<'a> {
    pub fn parse(text: &'a str) -> Result<Self> {
        let mut position = 0;
        parse_record(text, &mut position, None, 0)
    }

    pub fn field(&mut self, node: &Node) -> Option<&'a str> {
        for item in &mut self.items {
            if let Item::Field { name, value, used } = item {
                if !*used && node.matches(name) {
                    *used = true;
                    return Some(value);
                }
            }
        }
        None
    }

    pub fn blocks(&mut self, node: &Node) -> Vec<&mut Record<'a>> {
        self.items
            .iter_mut()
            .filter_map(|item| match item {
                Item::Block { name, record, used } if !*used && node.matches(name) => {
                    *used = true;
                    Some(record)
                }
                _ => None,
            })
            .collect()
    }

    pub fn constant(&mut self, constant: &str) -> Result<()> {
        for word in constant.split_whitespace() {
            let item = self.items.iter_mut().find(
                |item| matches!(item, Item::Literal { value, used: false } if *value == word),
            );
            match item {
                Some(Item::Literal { used, .. }) => *used = true,
                _ => bail!("Missing literal {word:?}"),
            }
        }
        Ok(())
    }

    pub fn finish(&self) -> Result<()> {
        for item in &self.items {
            match item {
                Item::Field {
                    name, used: false, ..
                } => bail!("Unrecognized or duplicate text field {name:?}"),
                Item::Block {
                    name, used: false, ..
                } => bail!("Unrecognized text record {name:?}"),
                Item::Literal { value, used: false } => bail!("Unrecognized text token {value:?}"),
                _ => {}
            }
        }
        Ok(())
    }
}

fn parse_record<'a>(
    text: &'a str,
    position: &mut usize,
    end: Option<&str>,
    depth: usize,
) -> Result<Record<'a>> {
    ensure!(depth < MAX_DEPTH, "Text record nesting exceeds {MAX_DEPTH}");
    let bytes = text.as_bytes();
    let mut record = Record::default();
    while *position < bytes.len() {
        while *position < bytes.len() && bytes[*position].is_ascii_whitespace() {
            *position += 1;
        }
        if *position == bytes.len() {
            break;
        }
        let start = *position;
        while *position < bytes.len()
            && !bytes[*position].is_ascii_whitespace()
            && bytes[*position] != b'='
        {
            *position += 1;
        }
        let word = &text[start..*position];
        ensure!(
            !word.is_empty(),
            "Expected field or record name at text offset {start}"
        );
        if bytes.get(*position) == Some(&b'=') {
            *position += 1;
            let start = *position;
            *position = value_end(text, start, false)?;
            ensure!(
                *position > start,
                "Empty value for field {word:?} at text offset {start}"
            );
            record.items.push(Item::Field {
                name: word,
                value: &text[start..*position],
                used: false,
            });
        } else if let Some(name) = word.strip_suffix("_begin") {
            let child = parse_record(text, position, Some(name), depth + 1)?;
            record.items.push(Item::Block {
                name,
                record: child,
                used: false,
            });
        } else if let Some(name) = word.strip_suffix("_end") {
            ensure!(
                end == Some(name),
                "Mismatched record end {word:?} at text offset {start}; expected {end:?}"
            );
            return Ok(record);
        } else {
            record.items.push(Item::Literal {
                value: word,
                used: false,
            });
        }
        ensure!(
            record.items.len() <= MAX_COUNT,
            "Text record exceeds {MAX_COUNT} items"
        );
    }
    ensure!(end.is_none(), "Unclosed record {end:?}");
    Ok(record)
}

fn value_end(text: &str, start: usize, list: bool) -> Result<usize> {
    let mut braces = 0usize;
    let mut brackets = 0usize;
    for (offset, byte) in text.as_bytes()[start..].iter().copied().enumerate() {
        let index = start + offset;
        if brackets == 0
            && braces == 0
            && (if list {
                byte == b';'
            } else {
                byte.is_ascii_whitespace()
            })
        {
            return Ok(index);
        }
        match byte {
            b'[' => brackets += 1,
            b']' => {
                ensure!(brackets > 0, "Unmatched ] at text offset {index}");
                brackets -= 1;
            }
            b'{' if brackets == 0 => braces += 1,
            b'}' if brackets == 0 => {
                ensure!(braces > 0, "Unmatched }} at text offset {index}");
                braces -= 1;
            }
            _ => {}
        }
        ensure!(
            braces <= MAX_DEPTH && brackets <= MAX_DEPTH,
            "Text nesting exceeds {MAX_DEPTH} at offset {index}"
        );
    }
    ensure!(
        braces == 0 && brackets == 0,
        "Unclosed value at text offset {start}"
    );
    Ok(text.len())
}

pub(super) fn list(text: &str) -> Result<Vec<&str>> {
    let text = text.trim();
    let text = if let Some(text) = text.strip_prefix('{') {
        text.strip_suffix('}')
            .ok_or_else(|| anyhow::anyhow!("Unclosed list {text:?}"))?
    } else {
        text
    };
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let mut values = Vec::new();
    let mut start = 0;
    loop {
        let end = value_end(text, start, true)?;
        values.push(text[start..end].trim());
        ensure!(
            values.len() <= MAX_COUNT,
            "List exceeds {MAX_COUNT} elements"
        );
        if end == text.len() {
            break;
        }
        start = end + 1;
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_keep_nested_rows_and_string_delimiters() {
        assert_eq!(
            list("{{1;[a;b=c\ttext]};{2;[nested [label]]}}").unwrap(),
            vec!["{1;[a;b=c\ttext]}", "{2;[nested [label]]}"]
        );
        assert!(list("{{1;2};{3;4}").is_err());
        assert!(Record::parse("row_begin\tid=1\tother_end").is_err());
        assert!(Record::parse("row_begin\tid=1").is_err());
    }
}
