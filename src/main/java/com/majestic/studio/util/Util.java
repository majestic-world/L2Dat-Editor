package com.majestic.studio.util;

import com.majestic.studio.compiler.Compiler;
import com.majestic.studio.compiler.MemoryClassLoader;
import org.apache.commons.lang3.ClassUtils;

import java.io.File;
import java.lang.reflect.Modifier;
import java.nio.ByteBuffer;
import java.util.*;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class Util {
    private static final Pattern TEXT_PATTERN = Pattern.compile("\\[(.*?)]");
    private static final Pattern MAP_PATTERN = Pattern.compile("\\b(\\S+)\\b\\s*=\\s*(.*?)\t");

    public static void printBytes(String paramString, byte[] paramArrayOfByte) {
        StringBuilder stringBuilder = new StringBuilder(paramArrayOfByte.length);
        stringBuilder.append(paramString).append(": [");

        for (byte b : paramArrayOfByte) {
            stringBuilder.append(b).append(" ");
        }

        stringBuilder.append("]");
        DebugUtil.getLogger().info(stringBuilder.toString());
    }

    public static boolean compareBuffers(byte[] paramArrayOfByte1, byte[] paramArrayOfByte2) {
        if (paramArrayOfByte1 != null && paramArrayOfByte2 != null && paramArrayOfByte1.length == paramArrayOfByte2.length) {
            for (byte b = 0; b < paramArrayOfByte1.length; ++b) {
                if (paramArrayOfByte1[b] != paramArrayOfByte2[b]) {
                    return false;
                }
            }

            return true;
        } else {
            return false;
        }
    }

    private static String printData(byte[] paramArrayOfByte, int paramInt) {
        StringBuilder stringBuilder = new StringBuilder();
        byte b = 0;

        for (int i = 0; i < paramInt; ++i) {
            if (b % 16 == 0) {
                stringBuilder.append(fillHex(i, 4)).append(": ");
            }

            stringBuilder.append(fillHex(paramArrayOfByte[i] & 255, 2)).append(" ");
            ++b;
            if (b == 16) {
                stringBuilder.append("\t ");
                int b1 = i - 15;

                for (byte b2 = 0; b2 < 16; ++b2) {
                    byte b3 = paramArrayOfByte[b1++];
                    if (b3 > 31 && b3 < 128) {
                        stringBuilder.append((char) b3);
                    } else {
                        stringBuilder.append('.');
                    }
                }

                stringBuilder.append("\n");
                b = 0;
            }
        }

        int var8 = paramArrayOfByte.length % 16;
        if (var8 > 0) {
            for (int j = 0; j < 17 - var8; ++j) {
                stringBuilder.append("\t ");
            }

            int var10 = paramArrayOfByte.length - var8;

            for (byte b1 = 0; b1 < var8; ++b1) {
                byte b2 = paramArrayOfByte[var10++];
                if (b2 > 31 && b2 < 128) {
                    stringBuilder.append((char) b2);
                } else {
                    stringBuilder.append('.');
                }
            }

            stringBuilder.append("\n");
        }

        return stringBuilder.toString();
    }

    private static String fillHex(int paramInt1, int paramInt2) {
        StringBuilder str = new StringBuilder(Integer.toHexString(paramInt1));

        for (int i = str.length(); i < paramInt2; ++i) {
            str.insert(0, "0");
        }

        return str.toString();
    }

    public static String printData(byte[] paramArrayOfByte) {
        return printData(paramArrayOfByte, paramArrayOfByte.length);
    }

    public static List<File> loadFiles(File file, String ext, boolean recursive) {
        if (!file.isDirectory()) {
            return Collections.singletonList(file);
        } else {
            File[] arrayOfFile = file.listFiles();
            if (arrayOfFile == null) {
                return Collections.emptyList();
            } else {
                List<File> files = new ArrayList<>();

                for (File f : arrayOfFile) {
                    if (recursive && f.isDirectory()) {
                        files.addAll(loadFiles(f, ext, true));
                    } else if (f.getName().endsWith(ext)) {
                        files.add(f);
                    }
                }

                return files;
            }
        }
    }

    public static List<File> loadFiles(String path, String ext, boolean recursive) {
        return loadFiles(new File(path), ext, recursive);
    }

    public static String[] getDirsNames(String paramString1, String paramString2) {
        List<String> arrayList = new ArrayList<>();
        File file = new File(paramString1);
        File[] arrayOfFile = file.listFiles();
        if (arrayOfFile == null) {
            return null;
        } else {
            for (File file1 : arrayOfFile) {
                if (file1.isDirectory() && file1.getName().endsWith(paramString2)) {
                    arrayList.add(file1.getName());
                }
            }

            String[] arrayOfString = new String[arrayList.size()];
            byte b = 0;

            for (String str : arrayList) {
                arrayOfString[b] = str;
                ++b;
            }

            return arrayOfString;
        }
    }

    public static String[] getFilesNames(String paramString1, String paramString2) {
        List<String> arrayList = new ArrayList<>();
        File file = new File(paramString1);
        File[] arrayOfFile = file.listFiles();
        if (arrayOfFile == null) {
            return null;
        } else {
            for (File file1 : arrayOfFile) {
                if (file1.isFile() && file1.getName().endsWith(paramString2)) {
                    arrayList.add(file1.getName().replace(paramString2, ""));
                }
            }

            String[] arrayOfString = new String[arrayList.size()];
            byte b = 0;

            for (String str : arrayList) {
                arrayOfString[b] = str;
                ++b;
            }

            return arrayOfString;
        }
    }

    public static String printData(ByteBuffer paramByteBuffer) {
        byte[] arrayOfByte = new byte[paramByteBuffer.remaining()];
        paramByteBuffer.get(arrayOfByte);
        String str = printData(arrayOfByte, arrayOfByte.length);
        paramByteBuffer.position(paramByteBuffer.position() - arrayOfByte.length);
        return str;
    }

    public static List<String> splitList(String paramString) {
        if (paramString.startsWith("{")) {
            paramString = paramString.substring(1, paramString.length() - 1);
        }

        ArrayList<String> arrayList = new ArrayList<>();
        StringBuilder stringBuffer = new StringBuilder();
        byte a = 0;
        boolean b = false;

        for (char c : paramString.toCharArray()) {
            if (!b && c == '{') {
                ++a;
            } else if (!b && c == '}') {
                --a;
            } else if (c == '[') {
                b = true;
            } else if (c == ']') {
                b = false;
            } else if (c == ';' && a == 0 && !b) {
                arrayList.add(stringBuffer.toString());
                stringBuffer = new StringBuilder();
                continue;
            }

            stringBuffer.append(c);
        }

        arrayList.add(stringBuffer.toString());
        return arrayList;
    }

    public static Map<String, String> stringToMap(String paramString) {
        paramString = paramString + "\t";
        LinkedHashMap<String, String> linkedHashMap = new LinkedHashMap<>();

        for (Matcher m = TEXT_PATTERN.matcher(paramString); m.find();) {
            String text = m.group(0);
            // Skip no-op replacements without changing the legacy escaping order.
            if (text.indexOf('\t') >= 0) {
                paramString = paramString.replace(text, text.replace("\t", "%_$tab$_%"));
            }
            if (text.indexOf('=') >= 0) {
                paramString = paramString.replace(text, text.replace("=", "%_$eq$_%"));
            }
        }

        Matcher var6 = MAP_PATTERN.matcher(paramString);

        while (var6.find()) {
            linkedHashMap.put(var6.group(1), var6.group(2).replace("%_$tab$_%", "\t").replace("%_$eq$_%", "="));
        }

        return linkedHashMap;
    }

    public static String mapToString(Map<String, String> paramMap) {
        StringBuilder stringBuilder = new StringBuilder();

        for (String str : paramMap.keySet()) {
            stringBuilder.append(str).append("=").append(paramMap.get(str)).append("\t");
        }

        return stringBuilder.toString();
    }

    public static Object loadJavaClass(String paramString1, String paramString2) {
        File file = new File(paramString2 + paramString1 + ".java");
        if (!file.exists()) {
            return null;
        } else {
            Compiler compiler = new Compiler();
            if (compiler.compile(Collections.singleton(file))) {
                MemoryClassLoader classLoader = compiler.getClassLoader();

                for (String name : classLoader.getLoadedClasses()) {
                    if (!name.contains(ClassUtils.INNER_CLASS_SEPARATOR)) {
                        try {
                            Class<?> clazz = classLoader.loadClass(name);
                            if (!Modifier.isAbstract(clazz.getModifiers()) && paramString1.equals(name)) {
                                return clazz.newInstance();
                            }
                        } catch (Exception e) {
                            e.printStackTrace();
                            break;
                        }
                    }
                }
            }

            return null;
        }
    }
}
