use std::collections::HashMap;

use anyhow::{Context, Result, anyhow, bail, ensure};

use super::binary::{self, Reader};
use super::text::{self, Record};
use super::{
    CodecOptions, Descriptor, Kind, MAX_COUNT, MAX_DEPTH, MAX_OPERATIONS, MAX_OUTPUT, NameTable,
    Node, Primitive,
};

pub fn decode(
    desc: &Descriptor,
    bytes: &[u8],
    options: &CodecOptions,
    names: &mut NameTable,
) -> Result<String> {
    let mut decoder = Decoder {
        desc,
        reader: Reader::new(bytes),
        options,
        names,
        variables: HashMap::new(),
        output: String::new(),
        operations: 0,
    };
    if desc.raw {
        decoder.variable(&desc.nodes[0], true).with_context(|| {
            format!(
                "{}: raw field {} at offset 0x0",
                desc.source, desc.nodes[0].name
            )
        })?;
    } else {
        decoder.nodes(&desc.nodes, None, 1, false, 0, 0)?;
        decoder.output.truncate(decoder.output.trim_end().len());
    }
    if desc.safe_package {
        let offset = decoder.reader.pos;
        let marker = decoder
            .reader
            .string(true)
            .with_context(|| format!("{}: SafePackage at offset 0x{offset:X}", desc.source))?;
        ensure!(
            marker == "SafePackage",
            "{}: wrong SafePackage marker {marker:?} at offset 0x{offset:X}",
            desc.source
        );
    }
    ensure!(
        decoder.reader.remaining() == 0,
        "{}: {} unrecognized trailing bytes at offset 0x{:X}; refusing a lossy decode",
        desc.source,
        decoder.reader.remaining(),
        decoder.reader.pos
    );
    Ok(decoder.output)
}

struct Decoder<'a, 'n> {
    desc: &'a Descriptor,
    reader: Reader<'a>,
    options: &'a CodecOptions,
    names: &'n mut NameTable,
    variables: HashMap<String, String>,
    output: String,
    operations: usize,
}

impl Decoder<'_, '_> {
    fn nodes(
        &mut self,
        nodes: &[Node],
        cycle: Option<&Node>,
        count: usize,
        hidden: bool,
        level: usize,
        depth: usize,
    ) -> Result<()> {
        ensure!(
            depth < MAX_DEPTH,
            "{}: nesting exceeds {MAX_DEPTH}",
            self.desc.source
        );
        ensure!(
            count <= MAX_COUNT,
            "{}: cycle count {count} exceeds {MAX_COUNT}",
            self.desc.source
        );
        let visible = nodes.iter().filter(|node| !node.iterator).count();
        for row in 0..count {
            self.operations += 1;
            ensure!(
                self.operations <= MAX_OPERATIONS,
                "{}: decoding exceeds {MAX_OPERATIONS} operations",
                self.desc.source
            );
            let named = cycle.filter(|_| !hidden);
            if let Some(cycle) = named {
                for _ in 0..level {
                    self.output.push('\t');
                }
                self.output.push_str(&cycle.name);
                self.output.push_str("_begin");
            }
            if hidden && visible > 1 {
                self.output.push('{');
            }
            for (index, node) in nodes.iter().enumerate() {
                self.operations += 1;
                ensure!(
                    self.operations <= MAX_OPERATIONS,
                    "{}: decoding exceeds {MAX_OPERATIONS} operations",
                    self.desc.source
                );
                let offset = self.reader.pos;
                self.node(
                    node,
                    index,
                    nodes.len(),
                    hidden,
                    level + usize::from(named.is_some()),
                    depth,
                )
                .with_context(|| {
                    format!(
                        "{}: field {} row {row} at offset 0x{offset:X}",
                        self.desc.source, node.name
                    )
                })?;
                ensure!(
                    self.output.len() <= MAX_OUTPUT,
                    "{}: decoded text exceeds {MAX_OUTPUT} bytes",
                    self.desc.source
                );
            }
            if hidden {
                if visible > 1 {
                    self.output.push('}');
                }
                if row + 1 < count {
                    self.output.push(';');
                }
            }
            if let Some(cycle) = named {
                if !self.output.ends_with('\n') {
                    self.output.push('\t');
                }
                self.output.push_str(&cycle.name);
                self.output.push_str("_end\r\n");
            }
        }
        Ok(())
    }

    fn node(
        &mut self,
        node: &Node,
        index: usize,
        node_count: usize,
        hidden: bool,
        level: usize,
        depth: usize,
    ) -> Result<()> {
        if is_condition(node) {
            if condition(node, &self.variables)? {
                self.nodes(&node.children, None, 1, hidden, level, depth + 1)?;
            }
            return Ok(());
        }
        if !node.iterator
            && !matches!(node.kind, Kind::Constant)
            && !hidden
            && (matches!(node.kind, Kind::Wrapper) || node.hidden)
        {
            self.output.push('\t');
            self.output.push_str(&node.name);
            self.output.push('=');
        }
        match &node.kind {
            Kind::Variable(_) => self.variable(node, false)?,
            Kind::Constant => self
                .output
                .push_str(&node.name.replace("\\t", "\t").replace("\\r\\n", "\r\n")),
            Kind::Wrapper => self.nodes(&node.children, None, 1, true, level, depth + 1)?,
            Kind::Cycle { size, counter, .. } => {
                let count = match size {
                    Some(count) => *count,
                    None => {
                        let value = self
                            .variables
                            .get(counter)
                            .ok_or_else(|| anyhow!("Missing cycle counter {counter:?}"))?;
                        let count = value
                            .parse::<i64>()
                            .with_context(|| format!("Invalid cycle counter {counter}={value}"))?;
                        ensure!(
                            count >= 0 && count <= MAX_COUNT as i64,
                            "Invalid cycle counter {counter}={count}"
                        );
                        count as usize
                    }
                };
                if node.hidden {
                    self.output.push('{');
                }
                self.nodes(
                    &node.children,
                    Some(node),
                    count,
                    node.hidden,
                    level,
                    depth + 1,
                )?;
                if node.hidden {
                    self.output.push('}');
                }
            }
            Kind::If { .. } | Kind::Mask { .. } => unreachable!("Conditions were handled above"),
        }
        if !node.iterator
            && !matches!(node.kind, Kind::Constant)
            && hidden
            && index + 1 < node_count
        {
            self.output.push(';');
        }
        Ok(())
    }

    fn variable(&mut self, node: &Node, raw: bool) -> Result<()> {
        let Kind::Variable(primitive) = node.kind else {
            bail!("Expected variable {}", node.name);
        };
        let mut value = self.reader.primitive(primitive, raw)?;
        let string = matches!(
            primitive,
            Primitive::Unicode | Primitive::Ascf | Primitive::String
        );
        if primitive == Primitive::MapInt {
            value = self.names.name(value.parse()?);
        } else if self.options.enums {
            if let Some(enumeration) = node
                .enumeration
                .as_ref()
                .and_then(|name| self.desc.enums.get(name))
            {
                if let Ok(index) = value.parse::<i32>() {
                    if let Some(name) = enumeration.names.get(&index) {
                        value.clone_from(name);
                    }
                }
            }
        }
        if !node.iterator {
            if string && !raw {
                self.output.push('[');
            }
            self.output.push_str(&value);
            if string && !raw {
                self.output.push(']');
            }
        }
        if !raw {
            set_variable(&mut self.variables, node, value);
        }
        Ok(())
    }
}

fn set_variable(variables: &mut HashMap<String, String>, node: &Node, value: String) {
    if let Some(old) = &node.old_name {
        variables.insert(old.clone(), value.clone());
    }
    variables.insert(node.name.clone(), value);
}

fn is_condition(node: &Node) -> bool {
    matches!(node.kind, Kind::If { .. } | Kind::Mask { .. })
}

fn condition(node: &Node, variables: &HashMap<String, String>) -> Result<bool> {
    match &node.kind {
        Kind::If {
            parameter,
            value,
            inverted,
        } => {
            let actual = variables
                .get(parameter)
                .ok_or_else(|| anyhow!("Missing condition parameter {parameter:?}"))?;
            Ok(actual.eq_ignore_ascii_case(value) != *inverted)
        }
        Kind::Mask { parameter, value } => {
            let actual = variables
                .get(parameter)
                .ok_or_else(|| anyhow!("Missing mask parameter {parameter:?}"))?;
            let mask = actual
                .parse::<i32>()
                .with_context(|| format!("Invalid mask parameter {parameter}={actual}"))?;
            Ok(mask & value == *value)
        }
        _ => bail!("Expected condition"),
    }
}

pub fn encode(
    desc: &Descriptor,
    text: &str,
    options: &CodecOptions,
    names: &mut NameTable,
) -> Result<Vec<u8>> {
    let checkpoint = names.checkpoint();
    let result = encode_inner(desc, text, options, names);
    if result.is_err() {
        names.rollback(checkpoint);
    }
    result
}

fn encode_inner(
    desc: &Descriptor,
    text: &str,
    options: &CodecOptions,
    names: &mut NameTable,
) -> Result<Vec<u8>> {
    ensure!(
        text.len() <= MAX_OUTPUT,
        "{}: input text exceeds {MAX_OUTPUT} bytes",
        desc.source
    );
    let mut encoder = Encoder {
        desc,
        options,
        names,
        variables: HashMap::new(),
        bytes: Vec::new(),
        counters: Vec::new(),
        operations: 0,
    };
    if desc.raw {
        encoder
            .variable(&desc.nodes[0], text, true)
            .with_context(|| {
                format!(
                    "{}: raw field {} at offset 0x0",
                    desc.source, desc.nodes[0].name
                )
            })?;
    } else {
        let mut record =
            Record::parse(text).with_context(|| format!("{}: parsing text", desc.source))?;
        encoder.nodes(&desc.nodes, &mut Input::Named(&mut record), 0)?;
        record.finish().with_context(|| desc.source.clone())?;
    }
    if desc.safe_package {
        binary::string(&mut encoder.bytes, "SafePackage", true)?;
    }
    encoder.finish().with_context(|| desc.source.clone())
}

struct Counter<'a> {
    node: &'a Node,
    offset: usize,
    value: Option<usize>,
    bytes: [u8; 8],
    length: usize,
}

struct Encoder<'a, 'n> {
    desc: &'a Descriptor,
    options: &'a CodecOptions,
    names: &'n mut NameTable,
    variables: HashMap<String, String>,
    bytes: Vec<u8>,
    counters: Vec<Counter<'a>>,
    operations: usize,
}

struct Values<'a> {
    values: Vec<&'a str>,
    next: usize,
}

impl<'a> Values<'a> {
    fn new(text: &'a str, nodes: &[Node]) -> Result<Self> {
        let count = nodes.iter().filter(|node| !node.iterator).count();
        let values = if count > 1 {
            text::list(text)?
        } else {
            vec![text]
        };
        Ok(Self { values, next: 0 })
    }

    fn take(&mut self) -> Option<&'a str> {
        let value = self.values.get(self.next).copied();
        if value.is_some() {
            self.next += 1;
        }
        value
    }

    fn finish(&self) -> Result<()> {
        // Legacy hidden groups may end in a separator after an invisible iterator.
        ensure!(
            self.values[self.next..]
                .iter()
                .all(|value| value.is_empty()),
            "Unexpected positional values: {:?}",
            &self.values[self.next..]
        );
        Ok(())
    }
}

enum Input<'a, 'r> {
    Named(&'r mut Record<'a>),
    Hidden(&'r mut Values<'a>),
}

impl<'a> Input<'a, '_> {
    fn value(&mut self, node: &Node) -> Option<&'a str> {
        match self {
            Self::Named(record) => record.field(node),
            Self::Hidden(values) => values.take(),
        }
    }
}

impl<'a> Encoder<'a, '_> {
    fn nodes(&mut self, nodes: &'a [Node], input: &mut Input<'_, '_>, depth: usize) -> Result<()> {
        ensure!(
            depth < MAX_DEPTH,
            "{}: nesting exceeds {MAX_DEPTH}",
            self.desc.source
        );
        for node in nodes {
            self.operations += 1;
            ensure!(
                self.operations <= MAX_OPERATIONS,
                "{}: encoding exceeds {MAX_OPERATIONS} operations",
                self.desc.source
            );
            let offset = self.bytes.len();
            self.node(node, input, depth).with_context(|| {
                format!(
                    "{}: field {} near output offset 0x{offset:X}",
                    self.desc.source, node.name
                )
            })?;
        }
        Ok(())
    }

    fn node(&mut self, node: &'a Node, input: &mut Input<'_, '_>, depth: usize) -> Result<()> {
        if node.iterator {
            self.counters.push(Counter {
                node,
                offset: self.bytes.len(),
                value: None,
                bytes: [0; 8],
                length: 0,
            });
            return Ok(());
        }
        match &node.kind {
            Kind::Variable(_) => {
                let value = input
                    .value(node)
                    .or(node.default.as_deref())
                    .ok_or_else(|| anyhow!("Missing variable {}", node.name))?;
                self.variable(node, value, false)?;
            }
            Kind::Constant => {
                if let Input::Named(record) = input {
                    record.constant(&node.name.replace("\\t", "\t").replace("\\r\\n", "\r\n"))?;
                }
            }
            Kind::Wrapper => {
                let value = input
                    .value(node)
                    .ok_or_else(|| anyhow!("Missing wrapper {}", node.name))?;
                self.hidden(&node.children, value, depth + 1)?;
            }
            Kind::Cycle { .. } if !node.hidden => {
                let Input::Named(record) = input else {
                    bail!(
                        "Named cycle {} cannot be embedded in a positional list",
                        node.name
                    );
                };
                let mut rows = record.blocks(node);
                self.size(node, rows.len())?;
                for row in &mut rows {
                    self.nodes(&node.children, &mut Input::Named(row), depth + 1)?;
                    row.finish()?;
                }
            }
            Kind::Cycle { .. } => {
                let value = input
                    .value(node)
                    .ok_or_else(|| anyhow!("Missing cycle {}", node.name))?;
                let rows = text::list(value)?;
                self.size(node, rows.len())?;
                for row in rows {
                    self.hidden(&node.children, row, depth + 1)?;
                }
            }
            Kind::If { .. } | Kind::Mask { .. } => {
                if condition(node, &self.variables)? {
                    match input {
                        Input::Named(_) => self.nodes(&node.children, input, depth + 1)?,
                        Input::Hidden(values) => {
                            let visible =
                                node.children.iter().filter(|child| !child.iterator).count();
                            if visible > 1 {
                                let value = values.take().ok_or_else(|| {
                                    anyhow!("Missing conditional group {}", node.name)
                                })?;
                                self.hidden(&node.children, value, depth + 1)?;
                            } else {
                                self.nodes(&node.children, input, depth + 1)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn hidden(&mut self, nodes: &'a [Node], value: &str, depth: usize) -> Result<()> {
        let mut values = Values::new(value, nodes)?;
        self.nodes(nodes, &mut Input::Hidden(&mut values), depth)?;
        values.finish()
    }

    fn variable(&mut self, node: &Node, text: &str, raw: bool) -> Result<()> {
        let Kind::Variable(primitive) = node.kind else {
            bail!("Expected primitive variable");
        };
        let mut value = text;
        let mapped;
        let string = matches!(
            primitive,
            Primitive::Unicode | Primitive::Ascf | Primitive::String
        );
        if primitive == Primitive::MapInt {
            mapped = self.names.index(text)?.to_string();
            value = &mapped;
        } else if string && !raw {
            value = binary::unbracket(text)?;
        } else if self.options.enums {
            if let Some(index) = node
                .enumeration
                .as_ref()
                .and_then(|name| self.desc.enums.get(name))
                .and_then(|enumeration| enumeration.indices.get(text))
            {
                mapped = index.to_string();
                value = &mapped;
            }
        }
        binary::primitive(&mut self.bytes, primitive, value, raw)?;
        if !raw {
            let condition_value = if string { value } else { text };
            set_variable(&mut self.variables, node, condition_value.to_owned());
        }
        Ok(())
    }

    fn size(&mut self, node: &Node, count: usize) -> Result<()> {
        ensure!(
            count <= MAX_COUNT,
            "Cycle {} exceeds {MAX_COUNT} rows",
            node.name
        );
        let Kind::Cycle {
            size,
            counter,
            skip_size,
        } = &node.kind
        else {
            bail!("Expected cycle");
        };
        if let Some(expected) = size {
            ensure!(
                *expected == count,
                "Static cycle {} requires {expected} rows, got {count}",
                node.name
            );
            return Ok(());
        }
        let slot = self
            .counters
            .iter_mut()
            .rev()
            .find(|slot| slot.node.matches(counter))
            .ok_or_else(|| anyhow!("Missing counter {counter:?} for cycle {}", node.name))?;
        if let Some(previous) = slot.value {
            ensure!(
                previous == count,
                "Shared counter {counter} requires {previous} rows, cycle {} has {count}",
                node.name
            );
            return Ok(());
        }
        ensure!(
            !skip_size,
            "Cycle {} uses skipWriteSize before its counter {counter} is written",
            node.name
        );
        let Kind::Variable(primitive) = slot.node.kind else {
            bail!("Counter {counter} is not a variable");
        };
        slot.length = counter_bytes(primitive, count, &mut slot.bytes)?;
        slot.value = Some(count);
        set_variable(&mut self.variables, slot.node, count.to_string());
        Ok(())
    }

    fn finish(self) -> Result<Vec<u8>> {
        if self.counters.is_empty() {
            return Ok(self.bytes);
        }
        let mut length = self.bytes.len();
        for counter in &self.counters {
            ensure!(
                counter.value.is_some(),
                "Counter {} has no encoded cycle",
                counter.node.name
            );
            length = length
                .checked_add(counter.length)
                .ok_or_else(|| anyhow!("Binary length overflow"))?;
        }
        ensure!(
            length <= MAX_OUTPUT,
            "Binary output exceeds {MAX_OUTPUT} bytes"
        );
        let mut output = Vec::with_capacity(length);
        let mut previous = 0;
        for counter in &self.counters {
            output.extend_from_slice(&self.bytes[previous..counter.offset]);
            output.extend_from_slice(&counter.bytes[..counter.length]);
            previous = counter.offset;
        }
        output.extend_from_slice(&self.bytes[previous..]);
        Ok(output)
    }
}

fn counter_bytes(primitive: Primitive, count: usize, output: &mut [u8; 8]) -> Result<usize> {
    let length = match primitive {
        Primitive::Uchar => {
            output[0] = i8::try_from(count)? as u8;
            1
        }
        Primitive::Ubyte => {
            output[0] = u8::try_from(count)?;
            1
        }
        Primitive::Short => {
            output[..2].copy_from_slice(&i16::try_from(count)?.to_le_bytes());
            2
        }
        Primitive::Ushort => {
            output[..2].copy_from_slice(&u16::try_from(count)?.to_le_bytes());
            2
        }
        Primitive::Int | Primitive::Uint => {
            output[..4].copy_from_slice(&i32::try_from(count)?.to_le_bytes());
            4
        }
        Primitive::Long => {
            output.copy_from_slice(&i64::try_from(count)?.to_le_bytes());
            8
        }
        Primitive::Counter => {
            let mut value = u32::try_from(count)?;
            output[0] = (value & 0x3f) as u8;
            value >>= 6;
            let mut size = 1;
            if value != 0 {
                output[0] |= 0x40;
            }
            while value != 0 {
                output[size] = (value & 0x7f) as u8;
                value >>= 7;
                if value != 0 {
                    output[size] |= 0x80;
                }
                size += 1;
            }
            size
        }
        _ => bail!("Cycle counter must have an integer primitive, got {primitive:?}"),
    };
    Ok(length)
}
