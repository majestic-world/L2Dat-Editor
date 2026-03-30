package com.majestic.studio.xml;

import com.majestic.studio.Boot;
import com.majestic.studio.actions.ActionTask;
import com.majestic.studio.clientcryptor.crypt.DatCrypter;
import com.majestic.studio.config.ConfigDebug;
import com.majestic.studio.config.ConfigWindow;
import com.majestic.studio.data.GameDataName;
import com.majestic.studio.util.ByteReader;
import com.majestic.studio.util.DebugUtil;
import org.apache.commons.lang3.StringUtils;

import java.io.File;
import java.math.BigDecimal;
import java.nio.ByteBuffer;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class DescriptorReader {
    private static final DescriptorReader INSTANCE = new DescriptorReader();
    final String eq = "=";
    final String tab = "\t";
    final String nl = System.getProperty("line.separator");
    final String semi = ";";
    final String lb = "[";
    final String rb = "]";

    public static DescriptorReader getInstance() {
        return INSTANCE;
    }

    public String parseData(ActionTask actionTask, double weight, File currentFile, DatCrypter crypter, Descriptor desc, ByteBuffer data, boolean mass) throws Exception {
        String stringData = "";
        boolean error = false;
        double progress = actionTask.getCurrentProgress();
        boolean hasFormatter = desc.getFormat() != null && !ConfigWindow.CURRENT_FORMATTER.equalsIgnoreCase("Disabled");
        if (desc.isRawData()) {
            if (actionTask.isCancelled()) {
                return null;
            }

            progress = actionTask.addProgress(progress, hasFormatter ? (double) 20.0F : (double) 49.0F, weight);
            StringBuilder builder = new StringBuilder();
            ParamNode node = desc.getNodes().get(0);
            if (this.readVariables(currentFile, crypter, node, new HashMap<>(), data, builder, true, mass)) {
                stringData = builder.toString();
            } else {
                if (Boot.MASS_FULL_LOG || !mass) {
                    String errorMsg = String.format("Error #1 while parsing variable NAME[%s] TYPE[%s] in file NAME[%s]! Parsed data: %s", node.getName(), node.getType(), currentFile.getName(), builder);
                    stringData = errorMsg;
                    Boot.addLogConsole(errorMsg, true);
                }

                if (mass) {
                    return null;
                }
            }

            progress = actionTask.addProgress(progress, 50.0F, weight);
        } else {
            Map<String, Variant> vars = new HashMap<>();
            Data result = this.parseData(actionTask, actionTask.getWeightValue(69.0F, weight), currentFile, crypter, data, null, desc.getNodes(), 1, vars, false, 0, mass);
            if (result != null) {
                stringData = result.data.toString().replaceAll("\\s+$", "");
                error = result.error;
            } else {
                stringData = "";
                error = true;
            }

            if (actionTask.isCancelled()) {
                return null;
            }

            progress = actionTask.addProgress(progress, 69.0F, weight);
        }

        if (!error && desc.getFormat() != null && !ConfigWindow.CURRENT_FORMATTER.equalsIgnoreCase("Disabled")) {
            stringData = desc.getFormat().decode(actionTask, actionTask.getWeightValue(30.0F, weight), stringData);
            if (actionTask.isCancelled()) {
                return null;
            }

            progress = actionTask.addProgress(progress, 30.0F, weight);
        }

        actionTask.addProgress(progress, 1.0F, weight);
        if (desc.isSafePackage()) {
            try {
                String str = ByteReader.readString(data, true);
                if (!str.equals("SafePackage")) {
                    if (Boot.MASS_FULL_LOG || !mass) {
                        Boot.addErrorConsole("SafePackage descriptor not found 'SafePackage' end-tag in file: " + currentFile.getName(), true);
                    }

                    if (mass) {
                        return null;
                    }
                }
            } catch (Exception var17) {
                if (Boot.MASS_FULL_LOG || !mass) {
                    Boot.addErrorConsole("SafePackage descriptor not found 'SafePackage' end-tag in file: " + currentFile.getName(), true);
                }

                if (mass) {
                    return null;
                }
            }
        }

        int pos = data.position();
        if (data.limit() > pos) {
            if (Boot.MASS_FULL_LOG || !mass) {
                Boot.addErrorConsole("Unpacked not full " + data.position() + "/" + data.limit() + " diff: " + (data.limit() - pos), true);
            }

            if (mass) {
                return null;
            }
        }

        return stringData;
    }

    private Data parseData(ActionTask actionTask, double weight, File currentFile, DatCrypter crypter, ByteBuffer data, ParamNode lastNode, List<ParamNode> nodes, int cycleSize, Map<String, Variant> vars, boolean isNameHidden, int cycleNameLevel, boolean mass) throws Exception {
        if (cycleSize <= 0) {
            return null;
        } else if (cycleSize > 1000000) {
            throw new Exception("To much data. lastNode: " + lastNode.getName());
        } else {
            Data result = new Data();
            StringBuilder out = result.data;

            for (int i = 0; i < cycleSize; ++i) {
                if (actionTask.isCancelled()) {
                    return null;
                }

                boolean isAddCycleName = !isNameHidden && lastNode != null && lastNode.getEntityType().isCycle();
                if (isAddCycleName) {
                    for (int k = 0; k < cycleNameLevel; ++k) {
                        out.append("\t");
                    }

                    out.append(lastNode.getName().concat("_begin"));
                    ++cycleNameLevel;
                }

                int nodeSize = nodes.size();
                int nNode = 0;

                for (ParamNode n : nodes) {
                    if (!n.isIterator()) {
                        ++nNode;
                    }
                }

                if (isNameHidden && nNode > 1) {
                    out.append("{");
                }

                double progress = actionTask.getCurrentProgress();
                double progressWeight = (double) 100.0F / (double) cycleSize / (double) nodeSize;

                for (int j = 0; j < nodeSize; ++j) {
                    if (actionTask.isCancelled()) {
                        return null;
                    }

                    ParamNode node = nodes.get(j);
                    if (node.getEntityType().isIf()) {
                        Variant var = vars.get(node.getParamIf());
                        if (var != null && var.toString().equalsIgnoreCase(node.getValIf())) {
                            Data dataResult = this.parseData(actionTask, actionTask.getWeightValue(progressWeight, weight), currentFile, crypter, data, null, node.getSubNodes(), 1, vars, isNameHidden, cycleNameLevel, mass);
                            if (dataResult != null) {
                                out.append(dataResult.data);
                                if (dataResult.error) {
                                    result.error = true;
                                    break;
                                }
                            }
                        }
                    } else if (node.getEntityType().isElse()) {
                        Variant var = vars.get(node.getParamIf());
                        if (var != null && !var.toString().equalsIgnoreCase(node.getValIf())) {
                            Data dataResult = this.parseData(actionTask, actionTask.getWeightValue(progressWeight, weight), currentFile, crypter, data, null, node.getSubNodes(), 1, vars, isNameHidden, cycleNameLevel, mass);
                            if (dataResult != null) {
                                out.append(dataResult.data);
                                if (dataResult.error) {
                                    result.error = true;
                                    break;
                                }
                            }
                        }
                    } else if (node.getEntityType().isMask()) {
                        Variant var = vars.get(node.getParamMask());
                        if (var != null) {
                            int value = Integer.parseInt(var.toString());
                            if ((value & node.getValMask()) == node.getValMask()) {
                                Data dataResult = this.parseData(actionTask, actionTask.getWeightValue(progressWeight, weight), currentFile, crypter, data, null, node.getSubNodes(), 1, vars, isNameHidden, cycleNameLevel, mass);
                                if (dataResult != null) {
                                    out.append(dataResult.data);
                                    if (dataResult.error) {
                                        result.error = true;
                                        break;
                                    }
                                }
                            }
                        }
                    } else {
                        if (!node.isIterator() && !node.getEntityType().isConstant() && !isNameHidden && (node.getEntityType().isWrapper() || node.isNameHidden())) {
                            out.append("\t").append(node.getName()).append("=");
                        }

                        if (node.getEntityType().isWrapper()) {
                            Data dataResult = this.parseData(actionTask, actionTask.getWeightValue(progressWeight, weight), currentFile, crypter, data, null, node.getSubNodes(), 1, vars, true, cycleNameLevel, mass);
                            if (dataResult != null) {
                                out.append(dataResult.data);
                                if (dataResult.error) {
                                    result.error = true;
                                    break;
                                }
                            }
                        } else if (node.getEntityType().isCycle()) {
                            int size;
                            if (node.getSize() >= 0) {
                                size = node.getSize();
                            } else {
                                Variant var = vars.get(node.getCycleName());
                                if (var.isInt()) {
                                    size = var.getInt();
                                } else {
                                    if (!var.isShort()) {
                                        throw new Exception("Wrong cycle variable format for cycle: " + node.getName() + " iterator: " + node.getCycleName());
                                    }

                                    size = var.getShort();
                                }
                            }

                            if (node.isNameHidden()) {
                                out.append("{");
                            }

                            Data dataResult = this.parseData(actionTask, actionTask.getWeightValue(progressWeight, weight), currentFile, crypter, data, node, node.getSubNodes(), size, vars, node.isNameHidden(), cycleNameLevel, mass);
                            if (dataResult != null) {
                                out.append(dataResult.data);
                                if (dataResult.error) {
                                    result.error = true;
                                    break;
                                }
                            }

                            if (node.isNameHidden()) {
                                out.append("}");
                            }
                        } else if (node.getEntityType().isConstant()) {
                            out.append(node.getName().replace("\\t", "\t").replace("\\r\\n", "\r\n"));
                        } else if (node.getEntityType().isVariable() && !this.readVariables(currentFile, crypter, node, vars, data, out, false, mass)) {
                            if (!mass) {
                                Boot.addErrorConsole(String.format("Error #2 while parsing variable NAME[%s] TYPE[%s] in file NAME[%s]! Parsed data: %s", node.getName(), node.getType(), currentFile.getName(), out), true);
                            }

                            result.error = true;
                            break;
                        }

                        if (!node.isIterator() && !node.getEntityType().isConstant() && isNameHidden && j != nodeSize - 1) {
                            out.append(";");
                        }

                        if (vars.containsKey(node.getName())) {
                            DebugUtil.debugPos(data.position(), node.getName(), vars.get(node.getName()));
                        }
                    }

                    if (actionTask.isCancelled()) {
                        return null;
                    }

                    actionTask.addProgress(progress, progressWeight, weight);
                }

                if (isNameHidden) {
                    if (nNode > 1) {
                        out.append("}");
                    }

                    if (i < cycleSize - 1) {
                        out.append(";");
                    }
                }

                if (isAddCycleName) {
                    if (out.charAt(out.length() - 1) != '\n') {
                        out.append("\t");
                    }

                    out.append(lastNode.getName()).append("_end\r\n");
                    --cycleNameLevel;
                }

                if (result.error) {
                    break;
                }
            }

            return result;
        }
    }

    private boolean readVariables(File currentFile, DatCrypter crypter, ParamNode node, Map<String, Variant> vars, ByteBuffer data, StringBuilder out, boolean isRaw, boolean mass) {
        try {
            // Declare variables once to avoid redeclaration errors in switch cases
            short shortValue;
            int intValue;
            double doubleValue;
            float floatValue;
            long longValue;
            String strValue;

            switch (node.getType()) {
                case UCHAR:
                    shortValue = (short) ((byte) ByteReader.readChar(data));
                    this.replaceEnum(vars, out, node, shortValue, Short.class);
                    break;
                case UBYTE:
                    intValue = ByteReader.readUByte(data);
                    this.replaceEnum(vars, out, node, intValue, Integer.class);
                    break;
                case SHORT:
                    shortValue = ByteReader.readShort(data);
                    this.replaceEnum(vars, out, node, shortValue, Short.class);
                    break;
                case USHORT:
                    intValue = ByteReader.readShort(data) & '\uffff';
                    this.replaceEnum(vars, out, node, intValue, Integer.class);
                    break;
                case UINT:
                    intValue = ByteReader.readUInt(data);
                    this.replaceEnum(vars, out, node, intValue, Integer.class);
                    break;
                case INT:
                    intValue = ByteReader.readInt(data);
                    this.replaceEnum(vars, out, node, intValue, Integer.class);
                    break;
                case CNTR:
                    intValue = ByteReader.readCompactInt(data);
                    this.replaceEnum(vars, out, node, intValue, Integer.class);
                    break;
                case UNICODE:
                case UNICODE_TRANSLATABLE:
                    strValue = ByteReader.readUtfString(data, isRaw);
                    if (isRaw) {
                        out.append(strValue);
                    } else {
                        if (!node.isIterator()) {
                            out.append("[");
                            out.append(strValue);
                            out.append("]");
                        }

                        vars.put(node.getName(), new Variant(strValue, String.class));
                    }
                    break;
                case ASCF:
                case ASCF_TRANSLATABLE:
                    strValue = ByteReader.readString(data, isRaw);
                    if (isRaw) {
                        out.append(strValue);
                    } else {
                        if (!node.isIterator()) {
                            out.append("[");
                            out.append(strValue);
                            out.append("]");
                        }

                        vars.put(node.getName(), new Variant(strValue, String.class));
                    }
                    break;
                case DOUBLE:
                    doubleValue = ByteReader.readDouble(data);
                    if (!node.isIterator()) {
                        out.append((new BigDecimal(Double.toString(doubleValue))).toPlainString());
                    }

                    vars.put(node.getName(), new Variant(doubleValue, Double.class));
                    break;
                case FLOAT:
                    floatValue = ByteReader.readFloat(data);
                    if (!node.isIterator()) {
                        out.append(floatValue);
                    }

                    vars.put(node.getName(), new Variant(floatValue, Float.class));
                    break;
                case LONG:
                    longValue = ByteReader.readLong(data);
                    if (!node.isIterator()) {
                        out.append(longValue);
                    }

                    vars.put(node.getName(), new Variant(longValue, Long.class));
                    break;
                case RGBA:
                    strValue = ByteReader.readRGBA(data);
                    if (!node.isIterator()) {
                        out.append(strValue);
                    }

                    vars.put(node.getName(), new Variant(strValue, String.class));
                    break;
                case RGB:
                    strValue = ByteReader.readRGB(data);
                    if (!node.isIterator()) {
                        out.append(strValue);
                    }

                    vars.put(node.getName(), new Variant(strValue, String.class));
                    break;
                case HEX:
                    intValue = ByteReader.readUByte(data);
                    if (!node.isIterator()) {
                        String hex = Integer.toHexString(intValue).toUpperCase();
                        if (hex.length() == 1) {
                            hex = "0" + hex;
                        }

                        out.append(hex);
                    }

                    vars.put(node.getName(), new Variant(intValue, Integer.class));
                    break;
                case MAP_INT:
                case MAP_INT_TRANSLATABLE:
                    int index = ByteReader.readUInt(data);
                    if (ConfigDebug.DAT_REPLACEMENT_NAMES) {
                        String paramName = GameDataName.getInstance().getString(currentFile, crypter, index, mass);
                        if (!node.isIterator()) {
                            out.append(paramName);
                        }

                        vars.put(node.getName(), new Variant(paramName, String.class));
                    } else {
                        if (!node.isIterator()) {
                            out.append(index);
                        }

                        vars.put(node.getName(), new Variant(index, Integer.class));
                    }
                    break;
                default:
                    return false;
            }

            return true;
        } catch (Exception e) {
            Boot.addErrorConsole("Error while read variable TYPE[" + node.getType() + "]: " + e, true);
            e.printStackTrace();
            return false;
        }
    }

    private void replaceEnum(Map<String, Variant> vars, StringBuilder out, ParamNode node, Object value, Class<?> type) {
        if (!ConfigWindow.CURRENT_ENUM.equalsIgnoreCase("Disabled") && node.isEnum() && value instanceof Number) {
            String enumValue = DescriptorParser.getInstance().getEnumNameByIndex(node.getEnumName(), ((Number) value).intValue());
            if (StringUtils.isEmpty(enumValue)) {
                Boot.addErrorConsole("Not found enum value for enum: " + node.getEnumName() + ", index: " + value, true);
            } else {
                if (!node.isIterator()) {
                    out.append(enumValue);
                }

                vars.put(node.getName(), new Variant(enumValue, String.class));
            }
        } else {
            if (!node.isIterator()) {
                out.append(value);
            }

            vars.put(node.getName(), new Variant(value, type));
        }
    }

    private static class Data {
        public final StringBuilder data;
        public boolean error;

        private Data() {
            this.data = new StringBuilder();
            this.error = false;
        }
    }
}
