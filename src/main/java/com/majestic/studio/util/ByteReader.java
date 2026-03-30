package com.majestic.studio.util;

import java.io.EOFException;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.charset.Charset;
import java.nio.charset.StandardCharsets;

public class ByteReader {
    private static final Charset DEFAULT_CHARSET = Charset.forName("cp1252");
    private static final Charset UTF_16_LE_CHARSET;

    public static char readChar(ByteBuffer buffer) {
        return (char) buffer.get();
    }

    public static int readUByte(ByteBuffer buffer) {
        return buffer.get() & 255;
    }

    public static int readInt(ByteBuffer buffer) {
        return Integer.reverseBytes(buffer.getInt());
    }

    public static int readUInt(ByteBuffer buffer) {
        return readInt(buffer);
    }

    public static short readShort(ByteBuffer buffer) {
        return Short.reverseBytes(buffer.getShort());
    }

    public static double readDouble(ByteBuffer buffer) {
        return Double.longBitsToDouble(Long.reverseBytes(buffer.getLong()));
    }

    public static long readLong(ByteBuffer buffer) {
        return Long.reverseBytes(buffer.getLong());
    }

    public static float readFloat(ByteBuffer buffer) {
        return Float.intBitsToFloat(Integer.reverseBytes(buffer.getInt()));
    }

    public static int readCompactInt(ByteBuffer input) throws IOException {
        int output = 0;
        boolean signed = false;

        for (int i = 0; i < 5; ++i) {
            int x = input.get() & 255;
            if (x < 0) {
                throw new EOFException();
            }

            if (i == 0) {
                if ((x & 128) > 0) {
                    signed = true;
                }

                output |= x & 63;
                if ((x & 64) == 0) {
                    break;
                }
            } else if (i == 4) {
                output |= (x & 31) << 27;
            } else {
                output |= (x & 127) << 6 + (i - 1) * 7;
                if ((x & 128) == 0) {
                    break;
                }
            }
        }

        if (signed) {
            output *= -1;
        }

        return output;
    }

    public static String readRGB(ByteBuffer buffer) {
        String r = Integer.toHexString(buffer.get() & 255).toUpperCase();
        if (r.length() < 2) {
            r = "0" + r;
        }

        String g = Integer.toHexString(buffer.get() & 255).toUpperCase();
        if (g.length() < 2) {
            g = "0" + g;
        }

        String b = Integer.toHexString(buffer.get() & 255).toUpperCase();
        if (b.length() < 2) {
            b = "0" + b;
        }

        return r + g + b;
    }

    public static String readRGBA(ByteBuffer buffer) {
        String a = Integer.toHexString(buffer.get() & 255).toUpperCase();
        if (a.length() < 2) {
            a = "0" + a;
        }

        return a + readRGB(buffer);
    }

    public static String readUtfString(ByteBuffer buffer, boolean isRaw) throws Exception {
        int size = readInt(buffer);
        if (size <= 0) {
            return "";
        } else if (size > 1000000) {
            throw new Exception("To much data.");
        } else {
            byte[] bytes = new byte[size];

            try {
                for (int i = 0; i < size; i += 2) {
                    bytes[i + 1] = buffer.get();
                    bytes[i] = buffer.get();
                }
            } catch (Exception e) {
                e.printStackTrace();
            }

            return checkAndReplaceNewLine(isRaw, new String((new String(bytes, "Unicode")).getBytes(StandardCharsets.UTF_8), StandardCharsets.UTF_8));
        }
    }

    public static String readString(ByteBuffer input, boolean isRaw) throws Exception {
        int len = readCompactInt(input);
        if (len == 0) {
            return "";
        } else {
            int size = len > 0 ? len : -2 * len;
            if (size > 1000000) {
                throw new Exception("To much data.");
            } else {
                byte[] bytes = new byte[size];
                input.get(bytes);
                return checkAndReplaceNewLine(isRaw, (new String(bytes, 0, bytes.length - (len > 0 ? 1 : 2), len > 0 ? DEFAULT_CHARSET : UTF_16_LE_CHARSET)).intern());
            }
        }
    }

    private static String checkAndReplaceNewLine(boolean isRaw, String str) {
        if (!isRaw && str.contains("\r\n")) {
            str = str.replace("\r\n", "\\r\\n");
        }

        return str;
    }

    static {
        UTF_16_LE_CHARSET = StandardCharsets.UTF_16LE;
    }
}
