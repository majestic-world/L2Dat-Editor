package jfork.typecaster;

import jfork.typecaster.exception.IllegalTypeException;

import java.lang.reflect.Field;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;

public class TypeCaster {
    private static final Class<?>[] _allowedTypes;

    public static void cast(Object object, Field field, String value) throws IllegalAccessException, IllegalTypeException {
        if (!isCastable(field)) {
            throw new IllegalTypeException("Unsupported type [" + field.getType().getName() + "] for field [" + field.getName() + "]");
        } else {
            Class<?> type = field.getType();
            boolean oldAccess = field.isAccessible();
            field.setAccessible(true);
            if (type.isEnum()) {
                field.set(object, enumValueOf(type, value));
            } else if (type != Integer.class && type != Integer.TYPE) {
                if (type != Short.class && type != Short.TYPE) {
                    if (type != Double.class && type != Double.TYPE) {
                        if (type != Long.class && type != Long.TYPE) {
                            if (type != Boolean.class && type != Boolean.TYPE) {
                                if (type == String.class) {
                                    field.set(object, value);
                                } else if (type != Byte.class && type != Byte.TYPE) {
                                    if (type == AtomicInteger.class) {
                                        field.set(object, new AtomicInteger(Integer.parseInt(value)));
                                    } else if (type == AtomicBoolean.class) {
                                        field.set(object, new AtomicBoolean(Boolean.parseBoolean(value)));
                                    } else {
                                        if (type != AtomicLong.class) {
                                            field.setAccessible(oldAccess);
                                            throw new IllegalTypeException("Unsupported type [" + type.getName() + "] for field [" + field.getName() + "]");
                                        }

                                        field.set(object, new AtomicLong(Long.parseLong(value)));
                                    }
                                } else {
                                    field.set(object, Byte.parseByte(value));
                                }
                            } else {
                                field.set(object, Boolean.parseBoolean(value));
                            }
                        } else {
                            field.set(object, Long.parseLong(value));
                        }
                    } else {
                        field.set(object, Double.parseDouble(value));
                    }
                } else {
                    field.set(object, Short.parseShort(value));
                }
            } else {
                field.set(object, Integer.parseInt(value));
            }

            field.setAccessible(oldAccess);
        }
    }

    public static <T> T cast(Class<T> type, String value) throws IllegalTypeException {
        if (!isCastable(type)) {
            throw new IllegalTypeException("Unsupported type [" + type.getName() + "]");
        } else if (type.isEnum()) {
            return (T) enumValueOf(type, value);
        } else if (type != Integer.class && type != Integer.TYPE) {
            if (type != Short.class && type != Short.TYPE) {
                if (type != Double.class && type != Double.TYPE) {
                    if (type != Long.class && type != Long.TYPE) {
                        if (type != Boolean.class && type != Boolean.TYPE) {
                            if (type == String.class) {
                                return (T) value;
                            } else if (type != Byte.class && type != Byte.TYPE) {
                                if (type == AtomicInteger.class) {
                                    return (T) (new AtomicInteger(Integer.parseInt(value)));
                                } else if (type == AtomicBoolean.class) {
                                    return (T) (new AtomicBoolean(Boolean.parseBoolean(value)));
                                } else if (type == AtomicLong.class) {
                                    return (T) (new AtomicLong(Long.parseLong(value)));
                                } else if (type == BigInteger.class) {
                                    return (T) (new BigInteger(value));
                                } else if (type == BigDecimal.class) {
                                    return (T) (new BigDecimal(value));
                                } else {
                                    throw new IllegalTypeException("Unsupported type [" + type.getName() + "]");
                                }
                            } else {
                                return (T) (Byte) Byte.parseByte(value);
                            }
                        } else {
                            return (T) (Boolean) Boolean.parseBoolean(value);
                        }
                    } else {
                        return (T) (Long) Long.parseLong(value);
                    }
                } else {
                    return (T) (Double) Double.parseDouble(value);
                }
            } else {
                return (T) (Short) Short.parseShort(value);
            }
        } else {
            return (T) (Integer) Integer.parseInt(value);
        }
    }

    public static boolean isCastable(Class<?> type) {
        if (type.isEnum()) {
            return true;
        } else {
            for (Class<?> t : _allowedTypes) {
                if (t == type) {
                    return true;
                }
            }

            return false;
        }
    }

    public static boolean isCastable(Object object) {
        return isCastable(object.getClass());
    }

    public static boolean isCastable(Field field) {
        return isCastable(field.getType());
    }

    @SuppressWarnings({"unchecked", "rawtypes"})
    private static Enum<?> enumValueOf(Class<?> type, String value) {
        return Enum.valueOf((Class<Enum>) type, value);
    }

    static {
        _allowedTypes = new Class<?>[]{Integer.class, Integer.TYPE, Short.class, Short.TYPE, Double.class, Double.TYPE, Long.class, Long.TYPE, Boolean.class, Boolean.TYPE, String.class, Character.class, Character.TYPE, Byte.class, Byte.TYPE, AtomicInteger.class, AtomicBoolean.class, AtomicLong.class};
    }
}
