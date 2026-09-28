use std::collections::HashMap;

use anyhow::{Context, Result, anyhow, ensure};

use super::binary::{self, Reader};
use super::{MAX_COUNT, MAX_DEPTH, MAX_OUTPUT};

#[derive(Default, Clone, Debug)]
pub struct NameTable {
    names: Vec<String>,
    indices: HashMap<String, i32>,
    loaded: bool,
    dirty: bool,
}

impl NameTable {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut reader = Reader::new(bytes);
        let count = reader
            .i32()
            .context("L2GameDataName.dat count at offset 0x0")?;
        ensure!(
            count >= 0 && count as usize <= MAX_COUNT,
            "L2GameDataName.dat invalid name count {count}"
        );
        ensure!(
            count as usize <= reader.remaining() / 4,
            "L2GameDataName.dat truncated name list"
        );
        let mut table = Self {
            loaded: true,
            ..Self::default()
        };
        for index in 0..count {
            let offset = reader.pos;
            let name = reader.unicode(false).with_context(|| {
                format!("L2GameDataName.dat name[{index}] at offset 0x{offset:X}")
            })?;
            table.indices.entry(name.to_lowercase()).or_insert(index);
            table.names.push(name);
        }
        let offset = reader.pos;
        ensure!(
            reader.string(true).with_context(|| format!(
                "L2GameDataName.dat SafePackage at offset 0x{offset:X}"
            ))? == "SafePackage",
            "L2GameDataName.dat missing SafePackage marker at offset 0x{offset:X}"
        );
        ensure!(
            reader.remaining() == 0,
            "L2GameDataName.dat has {} unrecognized trailing bytes at offset 0x{:X}",
            reader.remaining(),
            reader.pos
        );
        Ok(table)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        ensure!(
            self.loaded,
            "Cannot serialize an unloaded L2GameDataName.dat dictionary"
        );
        ensure!(
            self.names.len() <= MAX_COUNT,
            "Name table exceeds {MAX_COUNT} entries"
        );
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(self.names.len() as i32).to_le_bytes());
        for (index, name) in self.names.iter().enumerate() {
            binary::unicode(&mut bytes, name, false)
                .with_context(|| format!("L2GameDataName.dat name[{index}]"))?;
            ensure!(
                bytes.len() <= MAX_OUTPUT,
                "Name table exceeds {MAX_OUTPUT} bytes"
            );
        }
        binary::string(&mut bytes, "SafePackage", true)?;
        Ok(bytes)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(super) fn checkpoint(&self) -> (usize, bool) {
        (self.names.len(), self.dirty)
    }

    pub(super) fn rollback(&mut self, checkpoint: (usize, bool)) {
        for name in self.names.drain(checkpoint.0..) {
            self.indices.remove(&name.to_lowercase());
        }
        self.dirty = checkpoint.1;
    }

    pub(super) fn name(&self, index: i32) -> String {
        if let Some(name) = usize::try_from(index)
            .ok()
            .and_then(|index| self.names.get(index))
        {
            // Preserve IDs when spelling is duplicated or cannot be represented as a text value.
            if self.indices.get(&name.to_lowercase()) == Some(&index)
                && !name.is_empty()
                && can_display_name(name)
            {
                return format!("[{name}]");
            }
        }
        format!("[<StrID:{index}>]")
    }

    pub(super) fn index(&mut self, value: &str) -> Result<i32> {
        let name = binary::unbracket(value)?;
        if name.is_empty() {
            return Ok(-1);
        }
        if let Some(index) = name
            .strip_prefix("<StrID:")
            .and_then(|value| value.strip_suffix('>'))
        {
            return index.parse::<i32>().context("Invalid name-table string ID");
        }
        let key = name.to_lowercase();
        if let Some(index) = self.indices.get(&key) {
            return Ok(*index);
        }
        ensure!(
            self.loaded,
            "Adding a name requires the original L2GameDataName.dat; an unloaded dictionary cannot be safely reconstructed"
        );
        ensure!(
            self.names.len() < MAX_COUNT,
            "Name table exceeds {MAX_COUNT} entries"
        );
        let index = i32::try_from(self.names.len()).map_err(|error| anyhow!(error))?;
        self.names.push(name.to_owned());
        self.indices.insert(key, index);
        self.dirty = true;
        Ok(index)
    }
}

fn can_display_name(name: &str) -> bool {
    if name.starts_with("<StrID:") && name.ends_with('>') {
        return false;
    }
    // The surrounding brackets must stay open throughout the literal name.
    let mut depth = 1usize;
    for byte in name.bytes() {
        match byte {
            b'[' => {
                depth += 1;
                if depth > MAX_DEPTH {
                    return false;
                }
            }
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(names: &[&str]) -> NameTable {
        let mut bytes = (names.len() as i32).to_le_bytes().to_vec();
        for name in names {
            binary::unicode(&mut bytes, name, false).unwrap();
        }
        binary::string(&mut bytes, "SafePackage", true).unwrap();
        NameTable::from_bytes(&bytes).unwrap()
    }

    #[test]
    fn names_reuse_case_insensitive_ids_and_append_safely() {
        let mut names = table(&["Texture.One", "Texture.Two"]);
        assert_eq!(names.index("[texture.ONE]").unwrap(), 0);
        assert!(!names.is_dirty());
        assert_eq!(names.index("[New.Mesh]").unwrap(), 2);
        assert!(names.is_dirty());
        assert_eq!(
            NameTable::from_bytes(&names.to_bytes().unwrap())
                .unwrap()
                .name(2),
            "[New.Mesh]"
        );
        assert_eq!(names.index("[<StrID:-1>]").unwrap(), -1);
        assert_eq!(names.index("[<StrID:450>]").unwrap(), 450);
        assert!(NameTable::default().index("[New.Mesh]").is_err());
    }

    #[test]
    fn duplicate_names_and_unknown_ids_keep_identity() {
        let mut names = table(&["one", "ONE", ""]);
        for index in [-1, 0, 1, 2, 900] {
            assert_eq!(names.index(&names.name(index)).unwrap(), index);
        }
    }
}
