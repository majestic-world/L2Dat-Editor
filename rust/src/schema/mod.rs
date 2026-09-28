mod binary;
mod codec;
mod names;
#[cfg(test)]
mod tests;
mod text;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail, ensure};
use regex::Regex;
use roxmltree::{Document, Node as XmlNode};

pub use codec::{decode, encode};
pub use names::NameTable;

const MAX_COUNT: usize = 1_000_000;
const MAX_DEPTH: usize = 128;
const MAX_OPERATIONS: usize = 20_000_000;
const MAX_OUTPUT: usize = 512 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default)]
pub struct CodecOptions {
    pub enums: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Primitive {
    Uchar,
    Counter,
    Ubyte,
    Ushort,
    Short,
    Uint,
    String,
    Int,
    Unicode,
    Ascf,
    Double,
    Float,
    Long,
    Rgba,
    Rgb,
    Hex,
    MapInt,
}

impl Primitive {
    fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "UCHAR" => Self::Uchar,
            "CNTR" => Self::Counter,
            "UBYTE" => Self::Ubyte,
            "USHORT" => Self::Ushort,
            "SHORT" => Self::Short,
            "UINT" => Self::Uint,
            "STRING" => Self::String,
            "INT" => Self::Int,
            "UNICODE" | "UNICODE_TRANSLATABLE" => Self::Unicode,
            "ASCF" | "ASCF_TRANSLATABLE" => Self::Ascf,
            "DOUBLE" => Self::Double,
            "FLOAT" => Self::Float,
            "LONG" => Self::Long,
            "RGBA" => Self::Rgba,
            "RGB" => Self::Rgb,
            "HEX" => Self::Hex,
            "MAP_INT" | "MAP_INT_TRANSLATABLE" => Self::MapInt,
            _ => bail!("Unknown primitive or definition {value:?}"),
        })
    }
}

#[derive(Clone, Debug)]
enum Kind {
    Variable(Primitive),
    Cycle {
        size: Option<usize>,
        counter: String,
        skip_size: bool,
    },
    Wrapper,
    Constant,
    If {
        parameter: String,
        value: String,
        inverted: bool,
    },
    Mask {
        parameter: String,
        value: i32,
    },
}

#[derive(Clone, Debug)]
struct Node {
    id: usize,
    name: String,
    old_name: Option<String>,
    hidden: bool,
    iterator: bool,
    enumeration: Option<String>,
    default: Option<String>,
    kind: Kind,
    children: Vec<Node>,
}

impl Node {
    fn matches(&self, name: &str) -> bool {
        self.name == name || self.old_name.as_deref() == Some(name)
    }
}

#[derive(Default, Debug)]
struct Enumeration {
    names: HashMap<i32, String>,
    indices: HashMap<String, i32>,
}

type Enums = HashMap<String, Enumeration>;

#[derive(Clone, Debug)]
pub struct Descriptor {
    pub format: Option<String>,
    source: String,
    nodes: Arc<Vec<Node>>,
    enums: Arc<Enums>,
    raw: bool,
    safe_package: bool,
}

impl Descriptor {
    pub fn uses_names(&self) -> bool {
        fn contains(nodes: &[Node]) -> bool {
            nodes.iter().any(|node| {
                matches!(node.kind, Kind::Variable(Primitive::MapInt)) || contains(&node.children)
            })
        }
        contains(&self.nodes)
    }
}

struct Link {
    pattern: Regex,
    file: String,
    version: String,
}

struct Chronicle {
    name: String,
    parent: Option<i32>,
    links: Vec<Link>,
}

pub struct Catalog {
    chronicles: BTreeMap<i32, Chronicle>,
    order: Vec<i32>,
    descriptors: HashMap<String, HashMap<String, Descriptor>>,
    diagnostics: Vec<String>,
}

impl Catalog {
    pub fn load(data_dir: &Path) -> Result<Self> {
        let definitions_file = data_dir.join("definitions.xml");
        let definitions_text = fs::read_to_string(&definitions_file)
            .with_context(|| format!("Reading {}", definitions_file.display()))?;
        let definitions_doc =
            Document::parse(&definitions_text).context("Parsing definitions.xml")?;
        let mut definitions = HashMap::new();
        for definition in definitions_doc
            .root_element()
            .children()
            .filter(XmlNode::is_element)
        {
            ensure!(
                definition.has_tag_name("definition"),
                "Unknown definitions element {}",
                definition.tag_name().name()
            );
            let name = attr(definition, "name")?.to_owned();
            let nodes = parse_nodes(definition, true, &definitions, 0)
                .with_context(|| format!("Definition {name}"))?;
            definitions.insert(name, nodes);
        }
        let mut enums = Enums::new();
        for path in xml_files(&data_dir.join("enums"), true)? {
            let text =
                fs::read_to_string(&path).with_context(|| format!("Reading {}", path.display()))?;
            let doc =
                Document::parse(&text).with_context(|| format!("Parsing {}", path.display()))?;
            for enumeration in doc
                .root_element()
                .children()
                .filter(|n| n.has_tag_name("enum"))
            {
                let target = enums
                    .entry(attr(enumeration, "name")?.to_owned())
                    .or_default();
                for node in enumeration.children().filter(|n| n.has_tag_name("node")) {
                    let index = attr(node, "index")?.parse::<i32>()?;
                    let name = attr(node, "name")?.to_owned();
                    target.names.insert(index, name.clone());
                    target.indices.insert(name, index);
                }
            }
        }
        let enums = Arc::new(enums);
        let structure_dir = data_dir.join("structure");
        let mut catalog = Self {
            chronicles: BTreeMap::new(),
            order: Vec::new(),
            descriptors: HashMap::new(),
            diagnostics: Vec::new(),
        };
        let mut patterns: HashMap<String, Regex> = HashMap::new();
        for path in xml_files(&structure_dir, false)? {
            let text = fs::read_to_string(&path)?;
            let doc =
                Document::parse(&text).with_context(|| format!("Parsing {}", path.display()))?;
            for node in doc
                .root_element()
                .children()
                .filter(|n| n.has_tag_name("links"))
            {
                let id = attr(node, "id")?.parse::<i32>()?;
                let parent = node
                    .attribute("parent_id")
                    .map(str::parse::<i32>)
                    .transpose()?
                    .filter(|id| *id >= 0);
                let mut links = Vec::new();
                let mut link_positions = HashMap::new();
                for link in node.children().filter(|n| n.has_tag_name("link")) {
                    let pattern = attr(link, "pattern")?;
                    let compiled = if let Some(compiled) = patterns.get(pattern) {
                        compiled.clone()
                    } else {
                        let compiled = Regex::new(&format!("(?i)\\A(?:{pattern})\\z"))
                            .with_context(|| {
                                format!("File pattern {pattern:?} in {}", path.display())
                            })?;
                        patterns.insert(pattern.to_owned(), compiled.clone());
                        compiled
                    };
                    let parsed = Link {
                        pattern: compiled,
                        file: attr(link, "file")?.to_owned(),
                        version: attr(link, "version")?.to_owned(),
                    };
                    if let Some(index) = link_positions.get(pattern) {
                        links[*index] = parsed;
                    } else {
                        link_positions.insert(pattern, links.len());
                        links.push(parsed);
                    }
                }
                ensure!(
                    !catalog.chronicles.contains_key(&id),
                    "Duplicate chronicle ID {id}"
                );
                catalog.order.push(id);
                catalog.chronicles.insert(
                    id,
                    Chronicle {
                        name: attr(node, "name")?.to_owned(),
                        parent,
                        links,
                    },
                );
            }
        }
        for (id, chronicle) in &catalog.chronicles {
            let mut ancestors = HashSet::from([*id]);
            let mut parent = chronicle.parent;
            while let Some(parent_id) = parent {
                ensure!(
                    ancestors.insert(parent_id),
                    "Chronicle inheritance cycle from {} through ID {parent_id}",
                    chronicle.name
                );
                let Some(ancestor) = catalog.chronicles.get(&parent_id) else {
                    if chronicle.parent == Some(parent_id) {
                        catalog.diagnostics.push(format!(
                            "Chronicle {:?} (ID {id}) references missing parent ID {parent_id}",
                            chronicle.name
                        ));
                    }
                    break;
                };
                parent = ancestor.parent;
            }
        }
        let dat_dir = structure_dir.join("dats");
        for path in xml_files(&dat_dir, true)? {
            let text = fs::read_to_string(&path)?;
            let doc =
                Document::parse(&text).with_context(|| format!("Parsing {}", path.display()))?;
            let relative = path.strip_prefix(&dat_dir)?;
            let group = if relative.components().count() == 1 {
                path.file_stem()
            } else {
                relative.components().next().map(|c| c.as_os_str())
            }
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("Invalid structure filename {}", path.display()))?;
            for file in doc
                .root_element()
                .children()
                .filter(|n| n.has_tag_name("file"))
            {
                let version = attr(file, "pattern")?.to_owned();
                let source = format!("{} [{version}]", path.display());
                let mut nodes =
                    parse_nodes(file, false, &definitions, 0).with_context(|| source.clone())?;
                for diagnostic in compile_nodes(&mut nodes).with_context(|| source.clone())? {
                    catalog.diagnostics.push(format!("{source}: {diagnostic}"));
                }
                let raw = boolean(file, "isRaw", false);
                ensure!(
                    !raw || nodes.len() == 1 && matches!(nodes[0].kind, Kind::Variable(_)),
                    "Raw structure {source} must have exactly one variable"
                );
                let descriptor = Descriptor {
                    source,
                    nodes: Arc::new(nodes),
                    enums: enums.clone(),
                    raw,
                    safe_package: boolean(file, "isSafePackage", false),
                    format: file.attribute("format").map(str::to_owned),
                };
                catalog
                    .descriptors
                    .entry(group.to_owned())
                    .or_default()
                    .insert(version, descriptor);
            }
        }
        Ok(catalog)
    }

    pub fn chronicles(&self) -> Vec<String> {
        self.order
            .iter()
            .map(|id| self.chronicles[id].name.clone())
            .collect()
    }

    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    pub fn descriptor(&self, chronicle: &str, filename: &str) -> Result<Descriptor> {
        let mut current = self
            .chronicles
            .values()
            .find(|c| c.name == chronicle)
            .ok_or_else(|| anyhow!("Unknown chronicle {chronicle:?}"))?;
        loop {
            for link in &current.links {
                if link.pattern.is_match(filename) {
                    if let Some(descriptor) = self
                        .descriptors
                        .get(&link.file)
                        .and_then(|versions| versions.get(&link.version))
                    {
                        let mut descriptor = descriptor.clone();
                        descriptor.source = format!("{filename}: {}", descriptor.source);
                        return Ok(descriptor);
                    }
                }
            }
            match current.parent {
                Some(id) => current = self.chronicles.get(&id).ok_or_else(|| anyhow!("No structure for {filename:?} in {chronicle:?}; {:?} references missing parent ID {id}", current.name))?,
                None => bail!("No structure for {filename:?} in chronicle {chronicle:?}"),
            }
        }
    }
}

fn xml_files(dir: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    for entry in
        fs::read_dir(dir).with_context(|| format!("Reading directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() && recursive {
            result.extend(xml_files(&path, true)?);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"))
        {
            result.push(path);
        }
    }
    result.sort();
    Ok(result)
}

fn attr<'a>(node: XmlNode<'a, '_>, name: &str) -> Result<&'a str> {
    node.attribute(name)
        .ok_or_else(|| anyhow!("Missing {name:?} on XML element {}", node.tag_name().name()))
}

fn boolean(node: XmlNode<'_, '_>, name: &str, default: bool) -> bool {
    node.attribute(name)
        .map_or(default, |value| value.eq_ignore_ascii_case("true"))
}

fn parse_nodes(
    parent: XmlNode<'_, '_>,
    hide: bool,
    definitions: &HashMap<String, Vec<Node>>,
    depth: usize,
) -> Result<Vec<Node>> {
    ensure!(depth < MAX_DEPTH, "XML nesting exceeds {MAX_DEPTH}");
    let mut nodes: Vec<Node> = Vec::new();
    for xml in parent.children().filter(XmlNode::is_element) {
        let tag = xml.tag_name().name();
        if tag == "else" {
            let previous = nodes
                .last()
                .ok_or_else(|| anyhow!("Else without preceding if"))?;
            let Kind::If {
                parameter,
                value,
                inverted: false,
            } = &previous.kind
            else {
                bail!("Else must follow if");
            };
            nodes.push(Node {
                id: 0,
                name: previous.name.clone(),
                old_name: None,
                hidden: false,
                iterator: false,
                enumeration: None,
                default: None,
                kind: Kind::If {
                    parameter: parameter.clone(),
                    value: value.clone(),
                    inverted: true,
                },
                children: parse_nodes(xml, false, definitions, depth + 1)?,
            });
            continue;
        }
        let name = attr(xml, "name")?.to_owned();
        let old_name = xml.attribute("old_name").map(str::to_owned);
        let hidden = hide || boolean(xml, "hidden", true);
        if tag == "node" {
            let reader = attr(xml, "reader")?;
            if let Some(definition) = definitions.get(reader) {
                for template in definition {
                    let mut copy = template.clone();
                    copy.name = name.clone();
                    copy.old_name = old_name.clone();
                    copy.hidden |= hidden;
                    nodes.push(copy);
                }
                continue;
            }
        }
        let kind = match tag {
            "node" => Kind::Variable(Primitive::parse(attr(xml, "reader")?)?),
            "for" => {
                let size = xml.attribute("size");
                let (size, counter) = match size {
                    Some(value) if value.starts_with('#') => (None, value[1..].to_owned()),
                    Some(value) => {
                        let count = value.parse::<usize>()?;
                        ensure!(
                            count <= MAX_COUNT,
                            "Cycle {name} exceeds {MAX_COUNT} entries"
                        );
                        (Some(count), name.clone())
                    }
                    None => (None, name.clone()),
                };
                Kind::Cycle {
                    size,
                    counter,
                    skip_size: boolean(xml, "skipWriteSize", false),
                }
            }
            "wrapper" => Kind::Wrapper,
            "write" => Kind::Constant,
            "if" | "mask" => {
                let parameter = attr(xml, "param")?
                    .strip_prefix('#')
                    .ok_or_else(|| anyhow!("Condition {name} parameter must start with #"))?
                    .to_owned();
                if tag == "if" {
                    Kind::If {
                        parameter,
                        value: attr(xml, "val")?.to_owned(),
                        inverted: false,
                    }
                } else {
                    Kind::Mask {
                        parameter,
                        value: attr(xml, "val")?.parse()?,
                    }
                }
            }
            _ => bail!("Unknown XML node {tag:?} ({name})"),
        };
        nodes.push(Node {
            id: 0,
            name,
            old_name,
            hidden,
            iterator: false,
            enumeration: xml.attribute("enum_name").map(str::to_owned),
            default: xml.attribute("default_value").map(str::to_owned),
            children: parse_nodes(xml, tag == "wrapper", definitions, depth + 1)?,
            kind,
        });
    }
    Ok(nodes)
}

fn compile_nodes(nodes: &mut [Node]) -> Result<Vec<String>> {
    fn identify(nodes: &mut [Node], next: &mut usize) {
        for node in nodes {
            node.id = *next;
            *next += 1;
            identify(&mut node.children, next);
        }
    }
    fn references(
        nodes: &[Node],
        inherited: &HashMap<String, usize>,
        counters: &mut HashSet<usize>,
        diagnostics: &mut Vec<String>,
    ) {
        let mut scope = inherited.clone();
        for node in nodes {
            if matches!(node.kind, Kind::Variable(_)) {
                scope.insert(node.name.clone(), node.id);
                if let Some(old) = &node.old_name {
                    scope.insert(old.clone(), node.id);
                }
            }
            if let Kind::Cycle {
                size: None,
                counter,
                ..
            } = &node.kind
            {
                if let Some(id) = scope.get(counter) {
                    counters.insert(*id);
                } else {
                    diagnostics.push(format!(
                        "Cycle {} references missing counter {counter:?}",
                        node.name
                    ));
                }
            }
            references(&node.children, &scope, counters, diagnostics);
        }
    }
    fn mark(nodes: &mut [Node], counters: &HashSet<usize>) {
        for node in nodes {
            node.iterator = counters.contains(&node.id);
            mark(&mut node.children, counters);
        }
    }
    identify(nodes, &mut 0);
    let mut counters = HashSet::new();
    let mut diagnostics = Vec::new();
    references(nodes, &HashMap::new(), &mut counters, &mut diagnostics);
    mark(nodes, &counters);
    Ok(diagnostics)
}
