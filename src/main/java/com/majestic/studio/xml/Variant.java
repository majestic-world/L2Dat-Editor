package com.majestic.studio.xml;

public class Variant {
    private final Object value;
    private final Class<?> type;

    Variant(Object value, Class<?> type) {
        this.value = type.cast(value);
        this.type = type;
    }

    public final boolean isInt() {
        return this.type == Integer.class;
    }

    public final boolean isShort() {
        return this.type == Short.class;
    }

    public final boolean isFloat() {
        return this.type == Float.class;
    }

    public final boolean isDouble() {
        return this.type == Double.class;
    }

    public final int getInt() {
        return (Integer) this.value;
    }

    public final short getShort() {
        return (Short) this.value;
    }

    public final float getFloat() {
        return (Float) this.value;
    }

    public double getDouble() {
        return (Double) this.value;
    }

    public long getLong() {
        return (Long) this.value;
    }

    public final String toString() {
        return String.valueOf(this.value);
    }
}
