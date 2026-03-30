package com.majestic.studio.xml;

import com.majestic.studio.listeners.FormatListener;
import com.majestic.studio.util.DebugUtil;
import com.majestic.studio.util.Util;
import com.majestic.studio.xml.exceptions.CycleArgumentException;
import org.w3c.dom.Document;
import org.w3c.dom.Node;

import javax.xml.parsers.DocumentBuilderFactory;
import java.io.File;
import java.util.*;

public class DescriptorParser {
    private static final DescriptorParser INSTANCE = new DescriptorParser();
    private final Map<String, Map<Integer, String>> enumsById = new HashMap<>();
    private final Map<String, Map<String, String>> enumsByName = new HashMap<>();
    private final Map<String, Map<String, Descriptor>> descriptors = new HashMap<>();
    private final Map<String, List<ParamNode>> definitions = new HashMap<>();
    private final Map<String, DescriptorLinks> linksByName = new LinkedHashMap<>();
    private final Map<Integer, DescriptorLinks> linksById = new TreeMap<>();

    public static DescriptorParser getInstance() {
        return INSTANCE;
    }

    public void parse() {
        this.parseDefinitions();
        Util.loadFiles("./data/enums/", ".xml", true).forEach(this::parseEnum);
        Util.loadFiles("./data/structure/", ".xml", false).forEach(this::parseDescriptorLinks);
        this.parseDescriptors();
        this.applyParents();
    }

    private void parseDefinitions() {
        File file = new File("./data/definitions.xml");
        if (!file.exists()) {
            DebugUtil.debug("File " + file.getName() + " not found.");
        } else {
            DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance();
            factory.setValidating(false);
            factory.setIgnoringElementContentWhitespace(true);
            factory.setIgnoringComments(true);

            try {
                Document doc = factory.newDocumentBuilder().parse(file);

                for (Node defsNode = doc.getFirstChild(); defsNode != null; defsNode = doc.getNextSibling()) {
                    if (defsNode.getNodeName().equals("definitions")) {
                        for (Node defNode = defsNode.getFirstChild(); defNode != null; defNode = defNode.getNextSibling()) {
                            if (defNode.getNodeName().equals("definition")) {
                                String defName = defNode.getAttributes().getNamedItem("name").getNodeValue();
                                List<ParamNode> nodes = this.parseNodes(defNode, true, "definitions->" + defName, Collections.emptyList());
                                this.definitions.put(defName, nodes);
                            }
                        }
                    }
                }
            } catch (Exception e) {
                DebugUtil.getLogger().error(e.getMessage(), e);
            }

        }
    }

    private void parseEnum(File file) {
        if (!file.exists()) {
            DebugUtil.debug("File " + file.getName() + " not found.");
        } else {
            DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance();
            factory.setValidating(false);
            factory.setIgnoringElementContentWhitespace(true);
            factory.setIgnoringComments(true);

            try {
                Document doc = factory.newDocumentBuilder().parse(file);

                for (Node defsNode = doc.getFirstChild(); defsNode != null; defsNode = doc.getNextSibling()) {
                    if (defsNode.getNodeName().equals("list")) {
                        for (Node defNode = defsNode.getFirstChild(); defNode != null; defNode = defNode.getNextSibling()) {
                            if (defNode.getNodeName().equals("enum")) {
                                Node nodeName = defNode.getAttributes().getNamedItem("name");
                                if (nodeName == null) {
                                    DebugUtil.getLogger().warn("parseEnum name == null, fileName: " + file.getName());
                                } else {
                                    String defName = nodeName.getNodeValue();
                                    if (this.enumsById.containsKey(defName)) {
                                        DebugUtil.getLogger().warn("parseEnum Node name duplicated [" + defName + "]  fileName: " + file.getName());
                                    }

                                    Map<Integer, String> eTypes = this.enumsById.computeIfAbsent(defName, (m) -> new HashMap<>());
                                    Map<String, String> eReverseTypes = this.enumsByName.computeIfAbsent(defName, (m) -> new HashMap<>());

                                    for (Node node = defNode.getFirstChild(); node != null; node = node.getNextSibling()) {
                                        if (node.getNodeName().equals("node")) {
                                            String eName = node.getAttributes().getNamedItem("name").getNodeValue();
                                            int eIndex = Integer.parseInt(node.getAttributes().getNamedItem("index").getNodeValue());
                                            if (eReverseTypes.containsKey(eName)) {
                                                DebugUtil.getLogger().warn("parseEnum Node name duplicated [" + eName + "]  fileName: " + file.getName() + " name: " + defName);
                                            }

                                            if (eTypes.containsKey(eIndex)) {
                                                DebugUtil.getLogger().warn("parseEnum Node index duplicated [" + eIndex + "]  fileName: " + file.getName() + " name: " + defName);
                                            }

                                            eTypes.put(eIndex, eName);
                                            eReverseTypes.put(eName, String.valueOf(eIndex));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } catch (Exception e) {
                DebugUtil.getLogger().error(e.getMessage(), e);
            }

        }
    }

    private void parseDescriptorLinks(File file) {
        if (!file.exists()) {
            DebugUtil.debug("File " + file.getName() + " not found.");
        } else {
            try {
                DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance();
                factory.setValidating(false);
                factory.setIgnoringElementContentWhitespace(true);
                factory.setIgnoringComments(true);
                Document doc = factory.newDocumentBuilder().parse(file);
                String dir = file.getName().substring(0, file.getName().length() - 4);

                for (Node fileNode0 = doc.getFirstChild(); fileNode0 != null; fileNode0 = doc.getNextSibling()) {
                    if (fileNode0.getNodeName().equalsIgnoreCase("list")) {
                        for (Node fileNode = fileNode0.getFirstChild(); fileNode != null; fileNode = fileNode.getNextSibling()) {
                            if (fileNode.getNodeName().equalsIgnoreCase("links")) {
                                int id = Integer.parseInt(fileNode.getAttributes().getNamedItem("id").getNodeValue());
                                int parentId = fileNode.getAttributes().getNamedItem("parent_id") == null ? -1 : Integer.parseInt(fileNode.getAttributes().getNamedItem("parent_id").getNodeValue());
                                String name = fileNode.getAttributes().getNamedItem("name").getNodeValue();
                                if (parentId >= id) {
                                    DebugUtil.debug("Parent ID cannot be greater than or equal to own ID! Descriptor links file: [" + dir + "]");
                                } else {
                                    DescriptorLinks descriptorLinks = new DescriptorLinks(id, parentId, name);
                                    if (this.linksByName.containsKey(descriptorLinks.getName())) {
                                        DebugUtil.debug("Duplicate descriptor links by name: [" + descriptorLinks.getName() + "]!");
                                    }

                                    this.linksByName.put(descriptorLinks.getName(), descriptorLinks);
                                    if (this.linksById.containsKey(descriptorLinks.getId())) {
                                        DebugUtil.debug("Duplicate descriptor links by ID: [" + descriptorLinks.getId() + "]!");
                                    }

                                    this.linksById.put(descriptorLinks.getId(), descriptorLinks);

                                    for (Node fileNode1 = fileNode.getFirstChild(); fileNode1 != null; fileNode1 = fileNode1.getNextSibling()) {
                                        if (fileNode1.getNodeName().equalsIgnoreCase("link")) {
                                            String namePattern = fileNode1.getAttributes().getNamedItem("pattern").getNodeValue();
                                            String linkFile = fileNode1.getAttributes().getNamedItem("file").getNodeValue();
                                            String linkVersion = fileNode1.getAttributes().getNamedItem("version").getNodeValue();
                                            descriptorLinks.addLink(new DescriptorLink(namePattern, linkFile, linkVersion));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } catch (Exception e) {
                DebugUtil.getLogger().error("Error while parse descriptor link file [" + file.getName() + "]: " + e.getMessage(), e);
            }

        }
    }

    private void parseDescriptors() {
        String path = "./data/structure/dats/";
        File dir = new File(path);

        for (File file : Objects.requireNonNull(dir.listFiles())) {
            if (file.isDirectory()) {
                Util.loadFiles(file, ".xml", false).forEach((f) -> this.parseDescriptor(f, file.getName()));
            }
        }

        Util.loadFiles(dir, ".xml", false).forEach((f) -> this.parseDescriptor(f, f.getName().substring(0, f.getName().length() - 4)));
    }

    private void parseDescriptor(File file, String name) {
        if (!file.exists()) {
            DebugUtil.debug("File " + file.getName() + " not found.");
        } else {
            try {
                DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance();
                factory.setValidating(false);
                factory.setIgnoringElementContentWhitespace(true);
                factory.setIgnoringComments(true);
                Document doc = factory.newDocumentBuilder().parse(file);

                for (Node fileNode0 = doc.getFirstChild(); fileNode0 != null; fileNode0 = doc.getNextSibling()) {
                    if (fileNode0.getNodeName().equalsIgnoreCase("list")) {
                        for (Node fileNode = fileNode0.getFirstChild(); fileNode != null; fileNode = fileNode.getNextSibling()) {
                            if (fileNode.getNodeName().equalsIgnoreCase("file")) {
                                String namePattern = fileNode.getAttributes().getNamedItem("pattern").getNodeValue();
                                boolean isRawData = this.parseBoolNode(fileNode, "isRaw", false);
                                boolean isSafePackage = this.parseBoolNode(fileNode, "isSafePackage", false);
                                String formatName = this.parseStringNode(fileNode, "format", null);
                                DebugUtil.debug("Boot of parsing file: " + namePattern);
                                List<ParamNode> nodes = this.parseNodes(fileNode, false, name + "->" + namePattern, Collections.emptyList());
                                Descriptor desc = new Descriptor(file.getName(), namePattern, nodes);
                                desc.setIsRawData(isRawData);
                                desc.setIsSafePackage(isSafePackage);
                                if (formatName != null) {
                                    Object obj = Util.loadJavaClass(formatName, "./data/structure/format/");
                                    if (obj == null) {
                                        DebugUtil.getLogger().warn("Format src file '" + formatName + ".java' not found!");
                                    } else if (obj instanceof FormatListener) {
                                        desc.setFormat((FormatListener) obj);
                                    }
                                }

                                Map<String, Descriptor> versions = this.descriptors.computeIfAbsent(name, (k) -> new HashMap<>());
                                if (versions.containsKey(namePattern)) {
                                    DebugUtil.getLogger().warn("Duplicate descriptor by name: [" + namePattern + "] in file '" + file.getName() + "!");
                                }

                                versions.put(namePattern, desc);
                                this.descriptors.put(name, versions);
                                DebugUtil.debug("End of parsing file: " + namePattern);
                            }
                        }
                    }
                }
            } catch (Exception e) {
                DebugUtil.getLogger().error("Error while parse descriptor file [" + file.getName() + "]: " + e.getMessage(), e);
            }

        }
    }

    private List<ParamNode> parseNodes(Node fileNode, boolean isHideName, String fileName, List<ParamNode> parentNodes) throws Exception {
        Set<String> names = new HashSet<>();
        Map<String, Integer> defsCounter = new HashMap<>();
        List<ParamNode> nodes = new LinkedList<>();

        for (Node node = fileNode.getFirstChild(); node != null; node = node.getNextSibling()) {
            String nodeName = node.getNodeName();
            if (!nodeName.equals("#text")) {
                boolean isHide = isHideName || this.parseBoolNode(node, "hidden", true);
                if (nodeName.equalsIgnoreCase("else")) {
                    ParamNode prevNode = nodes.isEmpty() ? null : nodes.get(nodes.size() - 1);
                    if (prevNode == null) {
                        DebugUtil.debug("Not found previous IF for [else] data!");
                    } else {
                        ParamNode beginNode = new ParamNode(prevNode.getName(), ParamNodeType.ELSE, null);
                        beginNode.setParamIf(prevNode.getParamIf());
                        beginNode.setValIf(prevNode.getValIf());
                        beginNode.addSubNodes(this.parseNodes(node, false, fileName, nodes));
                        nodes.add(beginNode);
                        DebugUtil.debug("Found [else] data: " + prevNode.getName());
                    }
                } else if (node.getAttributes().getNamedItem("name") == null) {
                    DebugUtil.getLogger().warn("Node name == null, fileName: " + fileName);
                } else {
                    String entityName = node.getAttributes().getNamedItem("name").getNodeValue();
                    String entityOldName = null;
                    Node oldName = node.getAttributes().getNamedItem("old_name");
                    if (oldName != null) {
                        entityOldName = oldName.getNodeValue();
                    }

                    if (nodeName.equalsIgnoreCase("node")) {
                        String type = node.getAttributes().getNamedItem("reader").getNodeValue();
                        boolean key = node.getAttributes().getNamedItem("key") != null && Boolean.parseBoolean(node.getAttributes().getNamedItem("key").getNodeValue());
                        if (this.definitions.containsKey(type)) {
                            if (!defsCounter.containsKey(type)) {
                                defsCounter.put(type, 1);
                            } else {
                                defsCounter.put(type, defsCounter.get(type) + 1);
                            }

                            for (ParamNode defNode : this.definitions.get(type)) {
                                ParamNode copied = defNode.copy();
                                copied.setName(entityName);
                                copied.setOldName(entityOldName);
                                copied.setKey(key);
                                if (isHide) {
                                    copied.setHidden();
                                }

                                nodes.add(copied);
                            }
                        } else {
                            ParamNode dataNode = new ParamNode(entityName, ParamNodeType.VARIABLE, ParamType.valueOf(type));
                            dataNode.setKey(key);
                            if (isHide) {
                                dataNode.setHidden();
                            }

                            Node enumName = node.getAttributes().getNamedItem("enum_name");
                            if (enumName != null) {
                                dataNode.setEnumName(enumName.getNodeValue());
                            }

                            Node defaultValue = node.getAttributes().getNamedItem("default_value");
                            if (defaultValue != null) {
                                dataNode.setDefaultValue(defaultValue.getNodeValue());
                            }

                            dataNode.setOldName(entityOldName);
                            nodes.add(dataNode);
                            DebugUtil.debug("Found node: " + dataNode.getName());
                        }

                        if (names.contains(entityName)) {
                            DebugUtil.getLogger().warn("Node name duplicated [" + entityName + "]\tfileName: " + fileName);
                        } else {
                            names.add(entityName);
                        }

                        if (entityOldName != null) {
                            if (names.contains(entityOldName)) {
                                DebugUtil.getLogger().warn("Node old name duplicated [" + entityOldName + "]\tfileName: " + fileName);
                            } else {
                                names.add(entityOldName);
                            }
                        }
                    } else if (!nodeName.equalsIgnoreCase("for")) {
                        if (nodeName.equalsIgnoreCase("wrapper")) {
                            ParamNode beginNode = new ParamNode(entityName, ParamNodeType.WRAPPER, null);
                            beginNode.setOldName(entityOldName);
                            beginNode.addSubNodes(this.parseNodes(node, true, fileName, nodes));
                            nodes.add(beginNode);
                            DebugUtil.debug("Found [wrapper] data " + entityName);
                        } else if (nodeName.equalsIgnoreCase("write")) {
                            ParamNode beginNode = new ParamNode(entityName, ParamNodeType.CONSTANT, ParamType.STRING);
                            beginNode.setOldName(entityOldName);
                            beginNode.setHidden();
                            nodes.add(beginNode);
                            DebugUtil.debug("Found [constant] data: " + entityName);
                        } else if (nodeName.equalsIgnoreCase("if")) {
                            String paramName = node.getAttributes().getNamedItem("param").getNodeValue();
                            if (!paramName.startsWith("#")) {
                                throw new Exception("Invalid argument [" + entityName + "] for [if]");
                            }

                            String vsl = node.getAttributes().getNamedItem("val").getNodeValue();
                            paramName = paramName.substring(1);
                            ParamNode beginNode = new ParamNode(entityName, ParamNodeType.IF, null);
                            beginNode.setOldName(entityOldName);
                            beginNode.setParamIf(paramName);
                            beginNode.setValIf(vsl);
                            beginNode.addSubNodes(this.parseNodes(node, false, fileName, nodes));
                            nodes.add(beginNode);
                            DebugUtil.debug("Found [if] data: " + entityName);
                        } else if (nodeName.equalsIgnoreCase("mask")) {
                            String paramName = node.getAttributes().getNamedItem("param").getNodeValue();
                            if (!paramName.startsWith("#")) {
                                throw new Exception("Invalid argument [" + entityName + "] for [mask]");
                            }

                            int value = Integer.parseInt(node.getAttributes().getNamedItem("val").getNodeValue());
                            paramName = paramName.substring(1);
                            ParamNode beginNode = new ParamNode(entityName, ParamNodeType.MASK, null);
                            beginNode.setOldName(entityOldName);
                            beginNode.setParamMask(paramName);
                            beginNode.setValMask(value);
                            beginNode.addSubNodes(this.parseNodes(node, false, fileName, nodes));
                            nodes.add(beginNode);
                            DebugUtil.debug("Found [mask] data: " + entityName);
                        }
                    } else {
                        String iteratorName = entityName;
                        boolean skipWriteSize = this.parseBoolNode(node, "skipWriteSize", false);
                        int size = -1;
                        if (node.getAttributes().getNamedItem("size") != null) {
                            String sizeStr = node.getAttributes().getNamedItem("size").getNodeValue();
                            if (sizeStr.startsWith("#")) {
                                iteratorName = sizeStr.substring(1);
                            } else {
                                size = Integer.parseInt(sizeStr);
                            }
                        }

                        if (size == 0) {
                            DebugUtil.getLogger().warn("Size of cycle [" + iteratorName + "] was set to zero. Deprecated cycle?");
                        }

                        DebugUtil.debug("Found cycle for variable: " + entityName);
                        ParamNode beginNode = new ParamNode(entityName, ParamNodeType.FOR, null);
                        beginNode.setOldName(entityOldName);
                        if (size >= 0) {
                            beginNode.setSize(size);
                        }

                        if (isHide) {
                            beginNode.setHidden();
                        }

                        beginNode.setSkipWriteSize(skipWriteSize);
                        beginNode.addSubNodes(this.parseNodes(node, false, fileName, nodes));
                        beginNode.setCycleName(iteratorName);
                        nodes.add(beginNode);
                        boolean iteratorFound = false;

                        for (ParamNode n : nodes) {
                            if (iteratorName.equals(n.getName()) || iteratorName.equals(n.getOldName())) {
                                n.setIterator();
                                iteratorFound = true;
                                break;
                            }
                        }

                        if (!iteratorFound) {
                            for (ParamNode n : parentNodes) {
                                if (iteratorName.equals(n.getName()) || iteratorName.equals(n.getOldName())) {
                                    n.setIterator();
                                    iteratorFound = true;
                                    break;
                                }
                            }
                        }

                        if (!iteratorFound && size < 0) {
                            throw new CycleArgumentException("Invalid argument [" + iteratorName + "] for [cycle]");
                        }
                    }
                }
            }
        }

        return nodes;
    }

    private void applyParents() {
        for (DescriptorLinks links : this.linksById.values()) {
            DescriptorLinks parent = this.linksById.get(links.getParentId());
            if (parent != null) {
                links.setParentLinks(parent);
            }
        }

    }

    public Descriptor findDescriptorForFile(String dir, String fileName, boolean reload) {
        if (reload) {
            this.reload();
        }

        DescriptorLinks links = this.linksByName.get(dir);
        return links == null ? null : links.findDescriptor(fileName, this.descriptors);
    }

    private boolean parseBoolNode(Node node, String name, boolean def) {
        if (node.getAttributes() == null) {
            return def;
        } else {
            return node.getAttributes().getNamedItem(name) == null ? def : node.getAttributes().getNamedItem(name).getNodeValue().equalsIgnoreCase("true");
        }
    }

    private String parseStringNode(Node node, String name, String def) {
        if (node.getAttributes() == null) {
            return def;
        } else {
            return node.getAttributes().getNamedItem(name) == null ? def : node.getAttributes().getNamedItem(name).getNodeValue();
        }
    }

    public String getEnumNameByIndex(String eName, int index) {
        Map<Integer, String> map = this.enumsById.get(eName);
        if (map == null) {
            DebugUtil.getLogger().warn("Enum [" + eName + "] enum not found! index: " + index);
            return String.valueOf(index);
        } else {
            String result = map.get(index);
            if (result == null) {
                DebugUtil.getLogger().warn("Enum [" + eName + "] Enum var not found! var: " + index);
                return String.valueOf(index);
            } else {
                return result;
            }
        }
    }

    public String getEnumNameByName(String eName, String index) {
        Map<String, String> map = this.enumsByName.get(eName);
        if (map == null) {
            DebugUtil.getLogger().warn("Enum [" + eName + "] enum not found! index: " + index);
            return index;
        } else {
            String result = map.get(index);
            if (result == null) {
                DebugUtil.getLogger().warn("Enum [" + eName + "] Enum var not found! var: " + index);
                return index;
            } else {
                return result;
            }
        }
    }

    public Set<String> getChronicleNames() {
        return this.linksByName.keySet();
    }

    private void reload() {
        this.enumsById.clear();
        this.enumsByName.clear();
        this.descriptors.clear();
        this.definitions.clear();
        this.linksByName.clear();
        this.linksById.clear();
        this.parse();
    }
}
