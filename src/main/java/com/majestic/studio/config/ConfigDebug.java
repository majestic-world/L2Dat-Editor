package com.majestic.studio.config;

import jfork.nproperty.Cfg;
import jfork.nproperty.CfgIgnore;
import jfork.nproperty.ConfigParser;
import com.majestic.studio.util.DebugUtil;

import java.io.IOException;
import java.lang.reflect.InvocationTargetException;

@Cfg
public class ConfigDebug extends ConfigParser {
    @CfgIgnore
    private static final ConfigDebug _instance = new ConfigDebug();
    public static boolean DAT_ADD_END_BYTES = true;
    public static boolean DAT_DEBUG_MSG = false;
    public static boolean DAT_DEBUG_POS = false;
    public static int DAT_DEBUG_POS_LIMIT = 100000;
    public static boolean DAT_REPLACEMENT_NAMES = true;
    public static boolean ENCRYPT = true;
    public static boolean SAVE_DECODE = false;

    public static void load() {
        try {
            parse(_instance, "./data/config/config_debug.ini");
        } catch (IllegalAccessException | InstantiationException | NoSuchMethodException | InvocationTargetException | IOException e) {
            DebugUtil.getLogger().error("Failed to load configuration file.", e);
        }

    }
}
