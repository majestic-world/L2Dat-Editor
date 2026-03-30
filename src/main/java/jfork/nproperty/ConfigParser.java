package jfork.nproperty;

import jfork.typecaster.TypeCaster;
import jfork.typecaster.exception.IllegalTypeException;

import java.io.FileInputStream;
import java.io.IOException;
import java.lang.reflect.*;
import java.util.Hashtable;
import java.util.List;
import java.util.Map;
import java.util.Properties;

public class ConfigParser {
    private static volatile Map<String, Properties> _cache;

    public static synchronized void cleanCache(String path) {
        if (_cache != null) {
            _cache.remove(path);
        }
    }

    public static synchronized void parse(Object object, String path, boolean cache) throws InvocationTargetException, IOException, NoSuchMethodException, InstantiationException, IllegalAccessException {
        if (cache) {
            if (_cache == null) {
                _cache = new Hashtable<>();
            }
            _cache.put(path, parse(object, path));
        } else {
            parse(object, path);
        }
    }

    public static Properties parse(Object object, String path) throws IOException, IllegalAccessException, InstantiationException, NoSuchMethodException, InvocationTargetException {
        boolean callEvents = object instanceof IPropertyListener;
        if (callEvents) {
            ((IPropertyListener) object).onStart(path);
        }

        Properties props;
        if (_cache != null && _cache.containsKey(path)) {
            props = _cache.get(path);
        } else {
            props = new Properties();
            try (FileInputStream fis = new FileInputStream(path)) {
                props.load(fis);
            }
        }

        boolean classAnnotationPresent = object instanceof Class<?> ? ((Class<?>) object).isAnnotationPresent(Cfg.class) || ((Class<?>) object).isAnnotationPresent(Config.class) : object.getClass().isAnnotationPresent(Cfg.class) || object.getClass().isAnnotationPresent(Config.class);
        Field[] fields = object instanceof Class<?> ? ((Class<?>) object).getDeclaredFields() : object.getClass().getDeclaredFields();

        for (Field field : fields) {
            String name;
            if (field.isAnnotationPresent(Cfg.class)) {
                name = field.getAnnotation(Cfg.class).value();
            } else if (field.isAnnotationPresent(Config.class)) {
                name = field.getAnnotation(Config.class).value();
            } else if (field.isAnnotationPresent(CfgSplit.class)) {
                name = field.getAnnotation(CfgSplit.class).value();
            } else {
                if (!classAnnotationPresent) {
                    continue;
                }

                name = field.getName();
            }

            if (name.length() <= 0) {
                name = field.getName();
            }

            boolean oldAccess = field.isAccessible();
            field.setAccessible(true);
            if (props.containsKey(name)) {
                if (!field.isAnnotationPresent(CfgIgnore.class) && (field.isAnnotationPresent(Cfg.class) || field.isAnnotationPresent(Config.class) || classAnnotationPresent && !field.isAnnotationPresent(CfgSplit.class))) {
                    String propValue = getProperty(object, props, name);
                    if (propValue != null) {
                        if (TypeCaster.isCastable(field)) {
                            try {
                                TypeCaster.cast(object, field, propValue);
                            } catch (NumberFormatException | IllegalTypeException var25) {
                                if (callEvents) {
                                    ((IPropertyListener) object).onInvalidPropertyCast(name, propValue);
                                }
                            }
                        } else {
                            Constructor<?> construct = field.getType().getConstructor(String.class);
                            field.set(object, construct.newInstance(propValue));
                        }
                    }
                } else if (field.isAnnotationPresent(CfgSplit.class)) {
                    if (field.getType().isArray()) {
                        Class<?> baseType = field.getType().getComponentType();
                        String propValue = getProperty(object, props, name);
                        if (propValue != null) {
                            String[] values = propValue.split(field.getAnnotation(CfgSplit.class).splitter());
                            Object array = Array.newInstance(baseType, values.length);
                            field.set(object, array);
                            int index = 0;

                            for (String value : values) {
                                try {
                                    Array.set(array, index, TypeCaster.cast(baseType, value));
                                } catch (NumberFormatException | IllegalTypeException var24) {
                                    if (callEvents) {
                                        ((IPropertyListener) object).onInvalidPropertyCast(name, value);
                                    }
                                }

                                ++index;
                            }

                            field.set(object, array);
                        }
                    } else if (field.getType().isAssignableFrom(List.class)) {
                        if (field.get(object) == null) {
                            throw new NullPointerException("Cannot use null-object for parsing List splitter.");
                        }

                        Class<?> genericType = (Class<?>) ((ParameterizedType) field.getGenericType()).getActualTypeArguments()[0];
                        String propValue = getProperty(object, props, name);
                        if (propValue != null) {
                            String[] values = propValue.split(field.getAnnotation(CfgSplit.class).splitter());
                            Method add = field.getType().getMethod("add", Object.class);

                            for (String value : values) {
                                try {
                                    add.invoke(field.get(object), TypeCaster.cast(genericType, value));
                                } catch (NumberFormatException | IllegalTypeException var23) {
                                    if (callEvents) {
                                        ((IPropertyListener) object).onInvalidPropertyCast(name, value);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            field.setAccessible(oldAccess);
        }

        Method[] methods = object instanceof Class<?> ? ((Class<?>) object).getDeclaredMethods() : object.getClass().getDeclaredMethods();

        for (Method method : methods) {
            boolean annotated = true;
            String propName = null;
            if (method.isAnnotationPresent(Cfg.class)) {
                propName = method.getAnnotation(Cfg.class).value();
            } else if (method.isAnnotationPresent(Config.class)) {
                propName = method.getAnnotation(Config.class).value();
            } else {
                annotated = false;
            }

            if (annotated && method.getParameterTypes().length == 1) {
                String propValue = getProperty(object, props, propName);
                boolean oldAccess = method.isAccessible();
                method.setAccessible(true);
                if (propValue != null) {
                    try {
                        method.invoke(object, TypeCaster.cast(method.getParameterTypes()[0], propValue));
                    } catch (NumberFormatException | InvocationTargetException | IllegalTypeException var22) {
                        if (callEvents) {
                            ((IPropertyListener) object).onInvalidPropertyCast(propName, propValue);
                        }
                    }
                } else {
                    method.invoke(object, propValue);
                }

                method.setAccessible(oldAccess);
            }
        }

        if (callEvents) {
            ((IPropertyListener) object).onDone(path);
        }

        return props;
    }

    private static String getProperty(Object object, Properties properties, String name) {
        String property = properties.getProperty(name);
        if (property == null && object instanceof IPropertyListener) {
            ((IPropertyListener) object).onPropertyMiss(name);
        }

        return property;
    }
}
