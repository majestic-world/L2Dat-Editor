package com.majestic.studio.util;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.Charset;
import java.nio.charset.StandardCharsets;

public class ByteWriter {
    private static final ByteOrder BYTE_ORDER;
    private static final Charset DEFAULT_CHARSET;
    private static final Charset UTF_16_LE_CHARSET;

    public static byte[] writeCompactInt(int count) {
        return compactIntToByteArray(count);
    }

    public static byte[] writeByte(byte value) {
        return new byte[]{value};
    }

    public static byte[] writeUByte(short value) {
        return writeByte((byte) value);
    }

    public static byte[] writeInt(int value) {
        byte[] result = new byte[4];
        result[0] = (byte) (value & 255);
        result[1] = (byte) ((value & '\uff00') >> 8);
        result[2] = (byte) ((value & 16711680) >> 16);
        result[3] = (byte) ((value & -16777216) >> 24);
        return result;
    }

    public static byte[] writeUInt(int value) {
        return writeInt(value);
    }

    public static byte[] writeShort(int value) {
        byte[] result = new byte[2];
        result[0] = (byte) (value & 255);
        result[1] = (byte) ((value & '\uff00') >> 8);
        return result;
    }

    public static byte[] writeUShort(int value) {
        return writeShort(value);
    }

    public static byte[] writeRGB(String rgb) {
        ByteBuffer buffer = ByteBuffer.allocate(3).order(BYTE_ORDER);
        buffer.put((byte) Integer.parseInt(rgb.substring(0, 2), 16));
        buffer.put((byte) Integer.parseInt(rgb.substring(2, 4), 16));
        buffer.put((byte) Integer.parseInt(rgb.substring(4, 6), 16));
        return buffer.array();
    }

    public static byte[] writeRGBA(String rgba) {
        ByteBuffer buffer = ByteBuffer.allocate(4).order(BYTE_ORDER);
        buffer.put(writeRGB(rgba.substring(0, 6)));
        buffer.put((byte) Integer.parseInt(rgba.substring(6, 8), 16));
        return buffer.array();
    }

    public static byte[] writeUtfString(String str, boolean isRaw) {
        int size = str.length();
        if (size <= 0) {
            return ByteBuffer.allocate(4).order(BYTE_ORDER).putInt(0).array();
        } else {
            if (!isRaw) {
                str = checkAndReplaceNewLine(str);
                size = str.length();
            }

            ByteBuffer buffer = ByteBuffer.allocate(size * 2 + 4).order(BYTE_ORDER);
            buffer.putInt(size * 2);

            for (int i = 0; i < size; ++i) {
                buffer.putChar(str.charAt(i));
            }

            return buffer.array();
        }
    }

    public static byte[] writeString(String s, boolean isRaw) {
        if (s != null && !s.isEmpty()) {
            if (!isRaw) {
                s = checkAndReplaceNewLine(s);
            }

            s = s + '\u0000';
            boolean def = DEFAULT_CHARSET.newEncoder().canEncode(s);
            byte[] bytes = s.getBytes(def ? DEFAULT_CHARSET : UTF_16_LE_CHARSET);
            byte[] bSize = compactIntToByteArray(def ? bytes.length : -bytes.length / 2);
            ByteBuffer buffer = ByteBuffer.allocate(bytes.length + bSize.length).order(BYTE_ORDER);
            buffer.put(bSize);
            buffer.put(bytes);
            return buffer.array();
        } else {
            return writeCompactInt(0);
        }
    }

    public static byte[] writeDouble(double value) {
        ByteBuffer buffer = ByteBuffer.allocate(8).order(BYTE_ORDER);
        buffer.putDouble(value);
        return buffer.array();
    }

    public static byte[] writeFloat(float value) {
        ByteBuffer buffer = ByteBuffer.allocate(4).order(BYTE_ORDER);
        buffer.putFloat(value);
        return buffer.array();
    }

    public static byte[] writeLong(long value) {
        ByteBuffer buffer = ByteBuffer.allocate(8).order(BYTE_ORDER);
        buffer.putLong(value);
        return buffer.array();
    }

    private static byte[] compactIntToByteArray(int v) {
        boolean negative = v < 0;
        v = Math.abs(v);
        int[] bytes = new int[]{v & 63, v >> 6 & 127, v >> 13 & 127, v >> 20 & 127, v >> 27 & 127};
        if (negative) {
            bytes[0] |= 128;
        }

        int size = 5;

        for (int i = 4; i > 0 && bytes[i] == 0; --i) {
            --size;
        }

        byte[] res = new byte[size];

        for (int i = 0; i < size; ++i) {
            if (i != size - 1) {
                bytes[i] |= i == 0 ? 64 : 128;
            }

            res[i] = (byte) bytes[i];
        }

        return res;
    }

    private static String checkAndReplaceNewLine(String str) {
        if (str.contains("\\r\\n")) {
            str = str.replace("\\r\\n", "\r\n");
        }

        return str;
    }

    static {
        BYTE_ORDER = ByteOrder.LITTLE_ENDIAN;
        DEFAULT_CHARSET = StandardCharsets.US_ASCII;
        UTF_16_LE_CHARSET = StandardCharsets.UTF_16LE;
    }
}
