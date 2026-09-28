use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail, ensure};

/// Converts descriptor text to the editable, joined representation.
pub fn decode(format: &str, text: &str) -> Result<String> {
    decode_with_enums(format, text, false)
}

/// The legacy item formatter uses a different absent-enchant default in enum mode.
pub fn decode_with_enums(format: &str, text: &str, enums: bool) -> Result<String> {
    transform(format, text, false, enums).with_context(|| format!("decoding {format}"))
}

/// Converts editable text back to the descriptor's separate record groups.
pub fn encode(format: &str, text: &str) -> Result<String> {
    transform(format, text, true, false).with_context(|| format!("encoding {format}"))
}

#[derive(Clone, Debug)]
struct Record {
    kind: String,
    fields: Vec<(String, String)>,
    original: String,
}

impl Record {
    fn new(kind: &str, fields: &[(&str, String)]) -> Self {
        Self {
            kind: kind.to_owned(),
            fields: fields
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
            original: String::new(),
        }
    }

    fn get(&self, key: &str) -> Result<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .with_context(|| format!("{} record: missing {key}", self.kind))
    }

    fn number(&self, key: &str) -> Result<i32> {
        number(self.get(key)?, key).with_context(|| format!("{} record", self.kind))
    }

    fn put(&mut self, key: &str, value: String) {
        if let Some((_, v)) = self.fields.iter_mut().find(|(k, _)| k == key) {
            *v = value;
        } else {
            self.fields.push((key.to_owned(), value));
        }
    }

    fn remove(&mut self, key: &str) -> Option<String> {
        let index = self.fields.iter().position(|(k, _)| k == key)?;
        Some(self.fields.remove(index).1)
    }

    fn only(&self, allowed: &[&str]) -> Result<()> {
        for (key, _) in &self.fields {
            ensure!(
                allowed.contains(&key.as_str()),
                "{}: unsupported auxiliary field {key}",
                self.kind
            );
        }
        Ok(())
    }

    fn append(&self, output: &mut String) {
        output.push_str(&self.kind);
        output.push_str("_begin\t");
        for (key, value) in &self.fields {
            output.push_str(key);
            output.push('=');
            output.push_str(value);
            output.push('\t');
        }
        output.push_str(&self.kind);
        output.push_str("_end\r\n");
    }
}

fn number(value: &str, key: &str) -> Result<i32> {
    value
        .parse()
        .with_context(|| format!("{key}: expected signed 32-bit integer, got {value:?}"))
}

// Unlike the Java regex/sentinel parser, this does not mutate literal sentinel
// strings, split tabs inside groups, or silently discard unrecognized input.
fn records(text: &str) -> Result<Vec<Record>> {
    let mut result = Vec::new();
    let mut rest = text.trim_start_matches('\u{feff}');
    while !rest.trim().is_empty() {
        rest = rest.trim_start();
        let token_end = rest
            .find(char::is_whitespace)
            .context("record begin marker must be followed by whitespace")?;
        let token = &rest[..token_end];
        let kind = token
            .strip_suffix("_begin")
            .with_context(|| format!("expected record begin marker, got {token:?}"))?;
        ensure!(!kind.is_empty(), "empty record name");
        let end_marker = format!("{kind}_end");
        let body_start = token_end;
        let mut square = 0usize;
        let mut curly = 0usize;
        let mut end = None;
        for (offset, ch) in rest[body_start..].char_indices() {
            let index = body_start + offset;
            if square == 0
                && curly == 0
                && rest[index..].starts_with(&end_marker)
                && rest[..index].ends_with(char::is_whitespace)
                && rest[index + end_marker.len()..]
                    .chars()
                    .next()
                    .is_none_or(char::is_whitespace)
            {
                end = Some(index);
                break;
            }
            match ch {
                '[' => square += 1,
                ']' if square > 0 => square -= 1,
                '{' if square == 0 => curly += 1,
                '}' if square == 0 && curly > 0 => curly -= 1,
                _ => {}
            }
        }
        let end =
            end.with_context(|| format!("{kind}: missing {end_marker} or unclosed grouped value"))?;
        let fields = fields(&rest[body_start..end])
            .with_context(|| format!("{kind} record {}", result.len() + 1))?;
        let consumed = end + end_marker.len();
        result.push(Record {
            kind: kind.to_owned(),
            fields,
            original: rest[..consumed].to_owned(),
        });
        rest = &rest[consumed..];
    }
    Ok(result)
}

fn fields(body: &str) -> Result<Vec<(String, String)>> {
    let mut fields = Vec::new();
    let mut start = 0;
    let mut square = 0usize;
    let mut curly = 0usize;
    for (index, ch) in body
        .char_indices()
        .chain(std::iter::once((body.len(), '\t')))
    {
        match ch {
            '[' => square += 1,
            ']' if square > 0 => square -= 1,
            '{' if square == 0 => curly += 1,
            '}' if square == 0 && curly > 0 => curly -= 1,
            '\t' if square == 0 && curly == 0 => {
                let field = body[start..index].trim_start();
                if !field.is_empty() {
                    let (key, value) = field
                        .split_once('=')
                        .with_context(|| format!("expected key=value, got {field:?}"))?;
                    let key = key.trim();
                    ensure!(
                        !key.is_empty() && !key.contains(char::is_whitespace),
                        "invalid field name {key:?}"
                    );
                    ensure!(
                        !fields.iter().any(|(k, _)| k == key),
                        "duplicate field {key}"
                    );
                    fields.push((key.to_owned(), value.trim_start().to_owned()));
                }
                start = index + 1;
            }
            _ => {}
        }
    }
    ensure!(square == 0 && curly == 0, "unclosed grouped field");
    Ok(fields)
}

struct Join {
    main: &'static str,
    id: &'static str,
    side: &'static str,
    side_id: &'static str,
    field: &'static str,
    default: i32,
}

fn transform(format: &str, text: &str, encode: bool, enums: bool) -> Result<String> {
    let spec = match format {
        "SkillNameFormat" => return skill_names(records(text)?, encode),
        "ItemNameFormat" | "ItemNameFormat245" => Join {
            main: "item_name",
            id: "id",
            side: "item_autouse",
            side_id: "item_id",
            field: "autouse_type",
            default: 0,
        },
        "SkillGrpFormat" => Join {
            main: "skill",
            id: "id",
            side: "skill_autouse",
            side_id: "skill_id",
            field: "auto_use",
            default: 0,
        },
        "SkillGrpFormat245" => Join {
            main: "skill",
            id: "id",
            side: "skill_autouse",
            side_id: "skill_id",
            field: "auto_use_type",
            default: 0,
        },
        "CollectionFormat" => Join {
            main: "collection",
            id: "collection_ID",
            side: "collection_info",
            side_id: "collection_ID",
            field: "server_group_id",
            default: -1,
        },
        _ => bail!("unknown formatter {format:?}"),
    };
    joined(records(text)?, &spec, format, encode, enums)
}

// The vector retains first occurrence ordering while the map makes large tables
// linear-time. Updating a repeated ID retains its original position, as in Java
// LinkedHashMap; conflicting values are rejected rather than silently lost.
#[derive(Default)]
struct OrderedValues<K> {
    positions: HashMap<K, usize>,
    entries: Vec<(K, String)>,
}

impl<K: Eq + std::hash::Hash + Clone> OrderedValues<K> {
    fn insert(&mut self, key: K, value: String) -> Result<()> {
        if let Some(&index) = self.positions.get(&key) {
            ensure!(
                self.entries[index].1 == value,
                "conflicting auxiliary values for the same identity"
            );
        } else {
            self.positions.insert(key.clone(), self.entries.len());
            self.entries.push((key, value));
        }
        Ok(())
    }

    fn get(&self, key: &K) -> Option<&str> {
        self.positions
            .get(key)
            .map(|&index| self.entries[index].1.as_str())
    }
}

fn level_key(record: &Record, id: &str, level: &str, sublevel: &str) -> Result<(i32, i32)> {
    let level = record.number(level)?;
    let sublevel = record.number(sublevel)?;
    ensure!(
        (0..=65535).contains(&level) && (0..=65535).contains(&sublevel),
        "{}: level/sublevel exceeds 16-bit mask",
        record.kind
    );
    Ok((record.number(id)?, level | (sublevel << 16)))
}

fn joined(
    input: Vec<Record>,
    spec: &Join,
    format: &str,
    encode: bool,
    enums: bool,
) -> Result<String> {
    let item245 = format == "ItemNameFormat245";
    let skill245 = format == "SkillGrpFormat245";
    let collection = format == "CollectionFormat";
    let mut auto = OrderedValues::<i32>::default();
    let mut enchant = HashMap::<i32, (String, String)>::new();
    let mut enchant_order = Vec::new();
    let mut icons = OrderedValues::<(i32, i32)>::default();
    let mut main = Vec::new();
    let mut unknown = Vec::new();
    let mut groups = Vec::new();
    for record in input {
        if record.kind == spec.main {
            main.push(record);
        } else if collection && record.kind == "unk_507_data" {
            unknown.push(record);
        } else if collection && record.kind == "collection_server_group" {
            groups.push(record);
        } else if !encode && record.kind == spec.side {
            record.only(&[spec.side_id, spec.field])?;
            auto.insert(
                record.number(spec.side_id)?,
                record.number(spec.field)?.to_string(),
            )?;
        } else if !encode && item245 && record.kind == "item_enchant" {
            record.only(&["item_ex_id", "keep_type_selection", "keep_type_enchant"])?;
            let id = record.number("item_ex_id")?;
            let value = (
                record.get("keep_type_selection")?.to_owned(),
                record.get("keep_type_enchant")?.to_owned(),
            );
            if let Some(previous) = enchant.insert(id, value.clone()) {
                ensure!(
                    previous == value,
                    "item_enchant: conflicting entries for item {id}"
                );
            }
        } else if !encode && skill245 && record.kind == "icon_panel_2" {
            record.only(&[
                "skill_id2",
                "skill_level2",
                "skill_sublevel2",
                "icon_panel2",
            ])?;
            icons.insert(
                level_key(&record, "skill_id2", "skill_level2", "skill_sublevel2")?,
                record.get("icon_panel2")?.to_owned(),
            )?;
        } else {
            bail!(
                "unexpected {} record in {format} {} input",
                record.kind,
                if encode { "encode" } else { "decode" }
            );
        }
    }
    let mut body = String::new();
    for record in unknown.iter().chain(groups.iter()) {
        body.push_str(&record.original);
        body.push_str("\r\n");
    }
    let mut ids = HashSet::new();
    let mut levels = HashSet::new();
    for mut record in main {
        let id = record.number(spec.id)?;
        ids.insert(id);
        if encode {
            if let Some(value) = record.remove(spec.field) {
                let value = number(&value, spec.field)?;
                if value != spec.default {
                    auto.insert(id, value.to_string())?;
                }
            }
        } else {
            record.put(
                spec.field,
                auto.get(&id)
                    .map(str::to_owned)
                    .unwrap_or_else(|| spec.default.to_string()),
            );
        }
        if item245 {
            if encode {
                let selection = record.remove("keep_type_selection").unwrap_or_default();
                let value = record.remove("keep_type_enchant").unwrap_or_default();
                if !value.is_empty() {
                    ensure!(
                        !selection.is_empty(),
                        "item {id}: keep_type_enchant requires keep_type_selection"
                    );
                    ensure!(
                        !selection.parse::<i32>().is_ok_and(|v| v < 0),
                        "item {id}: negative enchant selection would discard data in Java"
                    );
                    let entry = (selection, value);
                    if let Some(previous) = enchant.get(&id) {
                        ensure!(previous == &entry, "item {id}: conflicting enchant data");
                    } else {
                        enchant_order.push(id);
                        enchant.insert(id, entry);
                    }
                }
            } else {
                let (selection, value) = enchant
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| (if enums { "" } else { "0" }.to_owned(), String::new()));
                record.put("keep_type_selection", selection);
                record.put("keep_type_enchant", value);
            }
        }
        if skill245 {
            let key = level_key(&record, "id", "level", "sublevel")?;
            levels.insert(key);
            if encode {
                if let Some(icon) = record.remove("icon_panel_2") {
                    if icon != "[]" {
                        icons.insert(key, icon)?;
                    }
                }
            } else {
                record.put("icon_panel_2", icons.get(&key).unwrap_or("[]").to_owned());
            }
        }
        record.append(&mut body);
    }
    if !encode {
        ensure!(
            auto.entries.iter().all(|(id, _)| ids.contains(id)),
            "orphan {} record has no {} record",
            spec.side,
            spec.main
        );
        ensure!(
            enchant.keys().all(|id| ids.contains(id)),
            "orphan item_enchant record"
        );
        ensure!(
            icons.entries.iter().all(|(key, _)| levels.contains(key)),
            "orphan icon_panel_2 record"
        );
        return Ok(body);
    }
    let mut side = String::new();
    for (id, value) in auto.entries {
        Record::new(
            spec.side,
            &[(spec.side_id, id.to_string()), (spec.field, value)],
        )
        .append(&mut side);
    }
    for id in enchant_order {
        let (selection, value) = enchant
            .remove(&id)
            .context("missing collected enchant entry")?;
        Record::new(
            "item_enchant",
            &[
                ("item_ex_id", id.to_string()),
                ("keep_type_selection", selection),
                ("keep_type_enchant", value),
            ],
        )
        .append(&mut side);
    }
    // Java emits all levels for the first skill before moving to the next skill.
    let mut icon_groups: HashMap<i32, Vec<(i32, String)>> = HashMap::new();
    let mut icon_ids = Vec::new();
    for ((id, mask), icon) in icons.entries {
        if !icon_groups.contains_key(&id) {
            icon_ids.push(id);
        }
        icon_groups.entry(id).or_default().push((mask, icon));
    }
    for id in icon_ids {
        for (mask, icon) in icon_groups
            .remove(&id)
            .context("missing collected icon group")?
        {
            Record::new(
                "icon_panel_2",
                &[
                    ("skill_id2", id.to_string()),
                    ("skill_level2", (mask & 65535).to_string()),
                    ("skill_sublevel2", ((mask as u32) >> 16).to_string()),
                    ("icon_panel2", icon),
                ],
            )
            .append(&mut side);
        }
    }
    if collection {
        side.push_str(&body);
        Ok(side)
    } else {
        body.push_str(&side);
        Ok(body)
    }
}

const TEXT_FIELDS: [&str; 7] = [
    "name",
    "desc",
    "desc_param",
    "enchant_name",
    "enchant_name_param",
    "enchant_desc",
    "enchant_desc_param",
];

fn skill_names(input: Vec<Record>, encode: bool) -> Result<String> {
    let mut texts = OrderedValues::<i32>::default();
    let mut interned = HashMap::<String, usize>::new();
    let mut text_order = Vec::new();
    let mut skills = Vec::new();
    for record in input {
        match record.kind.as_str() {
            "skill" => skills.push(record),
            "skill_txt" if !encode => {
                record.only(&["text", "index"])?;
                let text = record.get("text")?;
                ensure!(
                    text.starts_with('[') && text.ends_with(']'),
                    "skill_txt text must be bracketed"
                );
                texts.insert(record.number("index")?, text.to_owned())?;
            }
            _ => bail!("unexpected {} record in SkillNameFormat", record.kind),
        }
    }
    let mut output = String::new();
    let mut sorted = Vec::new();
    for mut record in skills {
        for field in TEXT_FIELDS {
            let value = record.get(field)?;
            let replacement = if encode {
                ensure!(
                    value.starts_with('[') && value.ends_with(']'),
                    "skill field {field} must be bracketed"
                );
                let index = if let Some(&index) = interned.get(value) {
                    index
                } else {
                    let index = text_order.len();
                    text_order.push(value.to_owned());
                    interned.insert(value.to_owned(), index);
                    index
                };
                index.to_string()
            } else {
                let index = number(value, field)?;
                texts
                    .get(&index)
                    .with_context(|| format!("skill {field}: missing skill_txt index {index}"))?
                    .to_owned()
            };
            record.put(field, replacement);
        }
        if encode {
            let key = (
                record.number("skill_id")?,
                record.number("skill_level")?,
                record.number("skill_sublevel")?,
            );
            sorted.push((key, record));
        } else {
            record.append(&mut output);
        }
    }
    if encode {
        for (index, text) in text_order.into_iter().enumerate() {
            Record::new("skill_txt", &[("text", text), ("index", index.to_string())])
                .append(&mut output);
        }
        sorted.sort_by_key(|(key, _)| *key);
        for (_, record) in sorted {
            record.append(&mut output);
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{decode, decode_with_enums, encode};

    #[test]
    fn item_and_skill_autouse_join_and_split() {
        for (format, main, side, side_id, field) in [
            (
                "ItemNameFormat",
                "item_name",
                "item_autouse",
                "item_id",
                "autouse_type",
            ),
            (
                "SkillGrpFormat",
                "skill",
                "skill_autouse",
                "skill_id",
                "auto_use",
            ),
        ] {
            let raw = format!(
                "{main}_begin\tid=7\tname=[A\tB=C]\t{main}_end\r\n{side}_begin\t{side_id}=7\t{field}=2\t{side}_end\r\n"
            );
            let editable = decode(format, &raw).unwrap();
            assert!(editable.contains(&format!("name=[A\tB=C]\t{field}=2\t")));
            assert_eq!(encode(format, &editable).unwrap(), raw);
        }
    }

    #[test]
    fn enchant_symbolic_and_numeric_values_round_trip() {
        for selection in ["0", "KEEP_ALL"] {
            let raw = format!(
                "item_name_begin\tid=9\titem_name_end\r\nitem_enchant_begin\titem_ex_id=9\tkeep_type_selection={selection}\tkeep_type_enchant={{1;{{2;3}}}}\titem_enchant_end\r\n"
            );
            assert_eq!(
                encode(
                    "ItemNameFormat245",
                    &decode("ItemNameFormat245", &raw).unwrap()
                )
                .unwrap(),
                raw
            );
        }
        let raw = "item_name_begin\tid=9\titem_name_end\r\n";
        assert!(
            decode_with_enums("ItemNameFormat245", raw, true)
                .unwrap()
                .contains("keep_type_selection=\t")
        );
        assert!(
            decode("ItemNameFormat245", raw)
                .unwrap()
                .contains("keep_type_selection=0\t")
        );
    }

    #[test]
    fn skill_icons_distinguish_sublevels_and_group_by_skill() {
        let editable = "skill_begin\tid=8\tlevel=1\tsublevel=2\tauto_use_type=0\ticon_panel_2=[a]\tskill_end\r\nskill_begin\tid=9\tlevel=1\tsublevel=0\tauto_use_type=3\ticon_panel_2=[]\tskill_end\r\nskill_begin\tid=8\tlevel=1\tsublevel=65535\tauto_use_type=0\ticon_panel_2=[b]\tskill_end\r\n";
        let raw = encode("SkillGrpFormat245", editable).unwrap();
        assert!(raw.contains("skill_sublevel2=65535\ticon_panel2=[b]"));
        assert_eq!(decode("SkillGrpFormat245", &raw).unwrap(), editable);
    }

    #[test]
    fn collection_preserves_passthrough_groups_and_missing_default() {
        let raw = "collection_info_begin\tcollection_ID=4\tserver_group_id=2\tcollection_info_end\r\nunk_507_data_begin\tdata={1;[a=b\tc]}\tunk_507_data_end\r\ncollection_server_group_begin\tid=2\tcollection_server_group_end\r\ncollection_begin\tcollection_ID=4\tcollection_end\r\ncollection_begin\tcollection_ID=5\tcollection_end\r\n";
        let editable = decode("CollectionFormat", raw).unwrap();
        assert!(editable.contains("collection_ID=5\tserver_group_id=-1\t"));
        assert_eq!(encode("CollectionFormat", &editable).unwrap(), raw);
    }

    #[test]
    fn skill_texts_intern_before_stable_skill_sorting() {
        let text = "skill_begin\tskill_id=9\tskill_level=1\tskill_sublevel=0\tname=[Z]\tdesc=[]\tdesc_param=[]\tenchant_name=[]\tenchant_name_param=[]\tenchant_desc=[]\tenchant_desc_param=[]\tskill_end\r\nskill_begin\tskill_id=2\tskill_level=1\tskill_sublevel=0\tname=[A]\tdesc=[Z]\tdesc_param=[]\tenchant_name=[]\tenchant_name_param=[]\tenchant_desc=[]\tenchant_desc_param=[]\tskill_end\r\n";
        let raw = encode("SkillNameFormat", text).unwrap();
        assert!(raw.starts_with("skill_txt_begin\ttext=[Z]\tindex=0\t"));
        assert_eq!(raw.matches("skill_txt_begin").count(), 3);
        assert!(raw.find("skill_id=2").unwrap() < raw.find("skill_id=9").unwrap());
        let decoded = decode("SkillNameFormat", &raw).unwrap();
        assert!(decoded.contains("name=[A]\tdesc=[Z]"));
        assert_eq!(
            encode(
                "SkillNameFormat",
                &decode(
                    "SkillNameFormat",
                    &encode("SkillNameFormat", &decoded).unwrap()
                )
                .unwrap()
            )
            .unwrap(),
            encode("SkillNameFormat", &decoded).unwrap()
        );
    }

    #[test]
    fn malformed_or_unknown_data_is_not_silently_discarded() {
        assert!(decode("ItemNameFormat", "garbage").is_err());
        assert!(
            decode(
                "ItemNameFormat",
                "item_name_begin\tid=1\tid=2\titem_name_end"
            )
            .is_err()
        );
        assert!(
            decode(
                "ItemNameFormat",
                "item_name_begin\tid=1\tname=[unclosed\titem_name_end"
            )
            .is_err()
        );
        assert!(
            decode(
                "ItemNameFormat",
                "item_autouse_begin\titem_id=1\tautouse_type=2\titem_autouse_end"
            )
            .is_err()
        );
        assert!(decode("not_registered", "").is_err());
    }
}
