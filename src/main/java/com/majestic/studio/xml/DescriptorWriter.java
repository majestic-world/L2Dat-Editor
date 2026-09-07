package com.majestic.studio.xml;

import com.majestic.studio.Boot;
import com.majestic.studio.actions.ActionTask;
import com.majestic.studio.clientcryptor.crypt.DatCrypter;
import com.majestic.studio.config.ConfigDebug;
import com.majestic.studio.config.ConfigWindow;
import com.majestic.studio.data.GameDataName;
import com.majestic.studio.util.ByteWriter;
import com.majestic.studio.util.DebugUtil;
import com.majestic.studio.util.Util;
import com.majestic.studio.xml.exceptions.CycleArgumentException;
import com.majestic.studio.xml.exceptions.PackDataException;
import org.apache.commons.lang3.StringUtils;
import org.apache.log4j.LogManager;
import org.apache.log4j.Logger;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.util.*;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class DescriptorWriter {
    public static final String SAFE_PACKAGE_END = "SafePackage";
    private static final Logger LOGGER = LogManager.getLogger(DescriptorWriter.class);

    public static byte[] parseData(ActionTask actionTask, double weight, File outputFile, DatCrypter crypter, Descriptor desc, String data, boolean mass) throws Exception {
        ByteArrayOutputStream stream = new ByteArrayOutputStream(data.length() / 2);
        double progress = actionTask.getCurrentProgress();
        if (desc.getFormat() != null && !ConfigWindow.CURRENT_FORMATTER.equalsIgnoreCase("Disabled")) {
            data = desc.getFormat().encode(actionTask, actionTask.getWeightValue(20.0F, weight), data);
            if (actionTask.isCancelled()) {
                return null;
            }

            progress = actionTask.addProgress(progress, 20.0F, weight);
        }

        if (desc.isRawData()) {
            if (actionTask.isCancelled()) {
                return null;
            }

            progress = actionTask.addProgress(progress, 50.0F, weight);
            byte[] bytes = parseNodeValue(outputFile, crypter, data, desc.getNodes().get(0), true, mass);
            if (bytes != null) {
                stream.write(bytes);
            } else {
                LOGGER.error("Failed to parse raw data.");
            }
        } else {
            String lines = data.replace("\r\n", "\t");
            List<WriteData> writeData = new ArrayList<>();
            packData(actionTask, actionTask.getWeightValue((double) 30.0F, weight), outputFile, crypter, writeData, lines, new HashMap<>(), new HashMap<>(), desc.getNodes(), mass);
            if (actionTask.isCancelled()) {
                return null;
            }

            progress = actionTask.addProgress(progress, 30.0F, weight);
            double progressWeight = actionTask.getWeightValue(40.0F, weight);
            double progressDiff = (double) 100.0F / (double) writeData.size();

            for (WriteData wr : writeData) {
                if (wr.isIterator()) {
                    DebugUtil.getLogger().error("Found iterator without writed size: " + wr.getParamNode().getName());
                } else {
                    stream.writeBytes(wr.getBytes());
                    if (actionTask.isCancelled()) {
                        return null;
                    }

                    progress = actionTask.addProgress(progress, progressDiff, progressWeight);
                }
            }

            if (actionTask.isCancelled()) {
                return null;
            }
        }

        actionTask.addProgress(progress, 10.0F, weight);
        if (desc.isSafePackage()) {
            stream.write(ByteWriter.writeString("SafePackage", true));
        }

        return stream.toByteArray();
    }

    private static void packData(ActionTask actionTask, double weight, File currentFile, DatCrypter crypter, List<WriteData> writeData, String lines, Map<String, String> paramMap, Map<ParamNode, String> mapData, List<ParamNode> nodes, boolean mass) throws Exception {
        List<WriteData> subWriteData = new ArrayList<>();

        for (ParamNode node : nodes) {
            if (actionTask != null && actionTask.isCancelled()) {
                return;
            }

            if (node.isIterator()) {
                subWriteData.add(new WriteIterator(node));
            } else if (node.getEntityType().isCycle()) {
                if (!node.isNameHidden()) {
                    String oldName = node.getOldName();
                    Pattern pattern;
                    if (oldName != null) {
                        pattern = Pattern.compile("\\b(" + node.getName() + "|" + oldName + ")_begin\\b(.*?\\s)(" + node.getName() + "|" + oldName + ")_end\\b", 32);
                    } else {
                        pattern = Pattern.compile("\\b(" + node.getName() + ")_begin\\b(.*?\\s)(" + node.getName() + ")_end\\b", 32);
                    }

                    Matcher m = pattern.matcher(lines);
                    List<String> list = new ArrayList<>();

                    while (m.find()) {
                        list.add(m.group(2));
                    }

                    double progress = actionTask != null ? actionTask.getCurrentProgress() : (double) 0.0F;
                    double progressWeight = actionTask != null ? actionTask.getWeightValue((double) 100.0F / (double) list.size(), weight) : (double) 0.0F;
                    writeSize(currentFile, crypter, subWriteData, node, list.size(), mass);

                    for (String str : list) {
                        paramMap.putAll(Util.stringToMap(str));
                        packData(actionTask, progressWeight, currentFile, crypter, subWriteData, str, paramMap, mapData, node.getSubNodes(), mass);
                    }

                    if (actionTask != null) {
                        if (actionTask.isCancelled()) {
                            return;
                        }

                        actionTask.addProgress(progress, 100.0F, weight);
                    }
                } else {
                    String param = getDataString(node, node.getName(), paramMap, mapData);
                    if (param == null) {
                        if (node.getOldName() != null) {
                            param = getDataString(node, node.getOldName(), paramMap, mapData);
                        }

                        if (param == null) {
                            throw new PackDataException("Not found data for cycle: " + node.getName() + "\r\n-node: " + node + "\r\n\tparam: " + paramMap.get(node.getName()));
                        }
                    }

                    if (!param.isEmpty() && !param.equals("{}")) {
                        List<String> subParams = Util.splitList(param);
                        int cycleSize = subParams.size();
                        if (node.getSize() > 0 && node.getSize() != cycleSize) {
                            throw new PackDataException("Wrong static cycle count for cycle: " + node.getName() + " size: " + subParams.size() + " params: " + param + "\r\n-node: " + node + "\r\n\tparam: " + paramMap.get(node.getName()));
                        }

                        writeSize(currentFile, crypter, subWriteData, node, cycleSize, mass);
                        int nPramNode = 0;
                        int nCycleNode = 0;

                        for (ParamNode n : node.getSubNodes()) {
                            if (n.getEntityType().isCycle()) {
                                ++nCycleNode;
                            } else if (!n.isIterator()) {
                                ++nPramNode;
                            }
                        }

                        for (String subParam : subParams) {
                            int paramIndex = 0;
                            List<String> sub2Params = nPramNode <= 0 && nCycleNode <= 1 ? Collections.singletonList(subParam) : Util.splitList(subParam);

                            for (ParamNode n : node.getSubNodes()) {
                                if (!n.isIterator() && !n.getEntityType().isConstant()) {
                                    if (paramIndex >= sub2Params.size()) {
                                        throw new PackDataException("Wrong param count for cycle: " + node.getName() + ", paramIndex: " + paramIndex + ", params: " + param + "\r\n-node: " + node + "\r\n\tparam: " + paramMap.get(node.getName()));
                                    }

                                    mapData.put(n, sub2Params.get(paramIndex++));
                                }
                            }

                            packData((ActionTask) null, (double) 0.0F, currentFile, crypter, subWriteData, lines, paramMap, mapData, node.getSubNodes(), mass);
                        }
                    } else {
                        writeSize(currentFile, crypter, subWriteData, node, 0, mass);
                    }
                }
            } else if (!node.getEntityType().isWrapper()) {
                if (node.getEntityType().isVariable()) {
                    String param = getDataString(node, node.getName(), paramMap, mapData);
                    if (param == null) {
                        if (node.getOldName() != null) {
                            param = getDataString(node, node.getOldName(), paramMap, mapData);
                        }

                        if (param == null) {
                            param = node.getDefaultValue();
                            if (param == null) {
                                throw new PackDataException("Not found data for variable: " + node.getName() + "\r\n-node: " + node + "\r\n\tparam: " + paramMap.get(node.getName()));
                            }
                        }
                    }

                    byte[] bytes = parseNodeValue(currentFile, crypter, param, node, false, mass);
                    if (bytes == null) {
                        throw new PackDataException("Node value is null.\r\n-node: " + node + "\r\n\tparam: " + paramMap.get(node.getName()));
                    }

                    subWriteData.add(new WriteBytes(node, bytes));
                } else if (node.getEntityType().isIf()) {
                    String param = getDataString(node, node.getParamIf(), paramMap, mapData);
                    if (param == null) {
                        throw new PackDataException("Not found data for if: " + node.getParamIf() + "\r\n-node: " + node + "\r\n\tparam: " + paramMap.get(node.getParamIf()));
                    }

                    if (node.getValIf().equalsIgnoreCase(param)) {
                        packData((ActionTask) null, (double) 0.0F, currentFile, crypter, subWriteData, lines, paramMap, mapData, node.getSubNodes(), mass);
                    }
                } else if (node.getEntityType().isElse()) {
                    String param = getDataString(node, node.getParamIf(), paramMap, mapData);
                    if (param == null) {
                        throw new PackDataException("Not found data for else: " + node.getParamIf() + "\r\n-node: " + node + "\r\n\tparam: " + paramMap.get(node.getParamIf()));
                    }

                    if (!node.getValIf().equalsIgnoreCase(param)) {
                        packData((ActionTask) null, (double) 0.0F, currentFile, crypter, subWriteData, lines, paramMap, mapData, node.getSubNodes(), mass);
                    }
                } else if (node.getEntityType().isMask()) {
                    String param = getDataString(node, node.getParamMask(), paramMap, mapData);
                    if (param == null) {
                        throw new PackDataException("Not found data for mask: " + node.getParamMask() + "\r\n-node: " + node + "\r\n\tparam: " + paramMap.get(node.getParamMask()));
                    }

                    int mask = Integer.parseInt(param);
                    if ((mask & node.getValMask()) == node.getValMask()) {
                        packData((ActionTask) null, (double) 0.0F, currentFile, crypter, subWriteData, lines, paramMap, mapData, node.getSubNodes(), mass);
                    }
                }
            } else {
                String param = getDataString(node, node.getName(), paramMap, mapData);
                if (param == null) {
                    if (node.getOldName() != null) {
                        param = getDataString(node, node.getOldName(), paramMap, mapData);
                    }

                    if (param == null) {
                        throw new PackDataException("Not found data for wrapper: " + node.getName() + "\r\n-node: " + node + "\r\n\tparam: " + paramMap.get(node.getName()));
                    }
                }

                List<String> subParams = Util.splitList(param);
                int paramIndex = 0;

                for (ParamNode n : node.getSubNodes()) {
                    if (!n.isIterator() && !n.getEntityType().isConstant()) {
                        if (paramIndex >= subParams.size()) {
                            throw new PackDataException("Wrong param count for wrapper: " + node.getName() + ", paramIndex: " + paramIndex + ", params: " + param + "\r\n-node: " + node + "\r\n\tparam: " + paramMap.get(node.getName()));
                        }

                        mapData.put(n, subParams.get(paramIndex++));
                    }
                }

                packData((ActionTask) null, (double) 0.0F, currentFile, crypter, subWriteData, lines, paramMap, mapData, node.getSubNodes(), mass);
            }
        }

        writeData.addAll(subWriteData);
    }

    private static byte[] parseNodeValue(File outputFile, DatCrypter crypter, String data, ParamNode node, boolean isRaw, boolean mass) {
        ParamType nodeType = node.getType();
        if (nodeType == null) {
            if (!mass) {
                LOGGER.error("Incorrect node type for node " + node);
            }

            return null;
        } else {
            try {
                switch (nodeType) {
                    case UCHAR:
                        return ByteWriter.writeUByte(Byte.parseByte(replaceEnum(node, data)));
                    case CNTR:
                        return ByteWriter.writeCompactInt(Integer.parseInt(replaceEnum(node, data)));
                    case UBYTE:
                        return ByteWriter.writeUByte(Short.parseShort(replaceEnum(node, data)));
                    case SHORT:
                        return ByteWriter.writeShort(Short.parseShort(replaceEnum(node, data)));
                    case USHORT:
                        return ByteWriter.writeUShort(Integer.parseInt(replaceEnum(node, data)));
                    case UINT:
                    case INT:
                        return ByteWriter.writeInt(Integer.parseInt(replaceEnum(node, data)));
                    case UNICODE:
                    case UNICODE_TRANSLATABLE:
                        return ByteWriter.writeUtfString(isRaw ? data : data.substring(1, data.length() - 1), isRaw);
                    case ASCF:
                    case ASCF_TRANSLATABLE:
                        return ByteWriter.writeString(isRaw ? data : data.substring(1, data.length() - 1), isRaw);
                    case DOUBLE:
                        return ByteWriter.writeDouble(Double.parseDouble(data));
                    case FLOAT:
                        return ByteWriter.writeFloat(Float.parseFloat(data));
                    case LONG:
                        return ByteWriter.writeLong(Long.parseLong(data));
                    case RGBA:
                        return ByteWriter.writeRGBA(data);
                    case RGB:
                        return ByteWriter.writeRGB(data);
                    case HEX:
                        return ByteWriter.writeByte((byte) (Integer.parseInt(data, 16) & 255));
                    case MAP_INT:
                        if (ConfigDebug.DAT_REPLACEMENT_NAMES) {
                            return ByteWriter.writeInt(GameDataName.getInstance().getId(outputFile, crypter, node, data, mass));
                        }

                        return ByteWriter.writeInt(Integer.parseInt(data));
                    default:
                        DebugUtil.getLogger().error("Unsupported primitive type " + nodeType);
                }
            } catch (Exception e) {
                if (!mass) {
                    LOGGER.error("Failed to parse value for node " + node + " data: " + data, e);
                }
            }

            return null;
        }
    }

    private static String replaceEnum(ParamNode node, String data) {
        if (!ConfigWindow.CURRENT_ENUM.equalsIgnoreCase("Disabled") && node.isEnum()) {
            String enumData = String.valueOf(DescriptorParser.getInstance().getEnumNameByName(node.getEnumName(), data));
            if (StringUtils.isEmpty(enumData)) {
                Boot.addErrorConsole("Not found enum index for enum: " + node.getEnumName() + ", value: " + data, true);
                return data;
            } else {
                return enumData;
            }
        } else {
            return data;
        }
    }

    private static String getDataString(ParamNode node, String name, Map<String, String> paramMap, Map<ParamNode, String> mapData) {
        return mapData != null && mapData.containsKey(node) ? mapData.get(node) : paramMap.get(name);
    }

    private static void writeSize(File currentFile, DatCrypter crypter, List<WriteData> writeData, ParamNode node, int cycleSize, boolean mass) throws CycleArgumentException, PackDataException {
        if (!node.isSkipWriteSize() && node.getSize() < 0) {
            WriteData iterator = null;

            for (int i = writeData.size() - 1; i >= 0; --i) {
                WriteData wd = writeData.get(i);
                if (wd.isIterator()) {
                    if (wd.getParamNode().getName().equals(node.getCycleName())) {
                        iterator = wd;
                        break;
                    }

                    if (wd.getParamNode().getOldName() != null && wd.getParamNode().getOldName().equals(node.getCycleName())) {
                        iterator = wd;
                        break;
                    }
                }
            }

            if (iterator == null) {
                throw new CycleArgumentException("Not found iterator for cycle: " + node.getName());
            }

            int index = writeData.indexOf(iterator);
            writeData.remove(index);
            ParamNode iteratorNode = iterator.getParamNode();
            byte[] bytes = parseNodeValue(currentFile, crypter, String.valueOf(cycleSize), iteratorNode, false, mass);
            if (bytes == null) {
                throw new PackDataException("Cant write size! Node value is null.\r\n-node: " + node);
            }

            writeData.add(index, new WriteBytes(iteratorNode, bytes));
        }

    }

    private static class WriteData {
        final ParamNode paramNode;

        WriteData(ParamNode paramNode) {
            this.paramNode = paramNode;
        }

        boolean isIterator() {
            return false;
        }

        ParamNode getParamNode() {
            return this.paramNode;
        }

        byte[] getBytes() {
            return new byte[0];
        }
    }

    public static class WriteIterator extends WriteData {
        WriteIterator(ParamNode paramNode) {
            super(paramNode);
        }

        public boolean isIterator() {
            return true;
        }
    }

    private static class WriteBytes extends WriteData {
        final byte[] bytes;

        WriteBytes(ParamNode paramNode, byte[] bytes) {
            super(paramNode);
            this.bytes = bytes;
        }

        public byte[] getBytes() {
            return this.bytes;
        }
    }
}
