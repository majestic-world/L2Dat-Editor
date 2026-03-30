package com.majestic.studio.config;

import jfork.nproperty.Cfg;
import jfork.nproperty.CfgIgnore;
import jfork.nproperty.ConfigParser;
import com.majestic.studio.util.DebugUtil;

import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.lang.reflect.InvocationTargetException;
import java.util.Properties;

@Cfg
public class ConfigWindow extends ConfigParser {
    @CfgIgnore
    private static final ConfigWindow _instance = new ConfigWindow();
    @CfgIgnore
    private static final String PATH = "./data/config/config_window.ini";
    public static String INPUT_DIRECTORY = ".";
    public static String OUTPUT_DIRECTORY = ".";
    public static String CURRENT_STRUCTURE = "";
    public static int WINDOW_HEIGHT = 600;
    public static int WINDOW_WIDTH = 800;
    public static String CURRENT_ENCRYPT = ".";
    public static String LAST_FILE_SELECTED = ".";
    public static String CURRENT_FORMATTER = ".";
    public static String CURRENT_ENUM = ".";

    public static void load() {
        try {
            parse(_instance, "./data/config/config_window.ini");
        } catch (IllegalAccessException | InstantiationException | NoSuchMethodException | InvocationTargetException | IOException e) {
            DebugUtil.getLogger().error("Failed to load configuration file.", e);
        }

    }

    public static void save(String key, String var) {
        try {
            Properties props = new Properties();
            props.load(new FileInputStream("./data/config/config_window.ini"));
            props.setProperty(key, var);
            FileOutputStream output = new FileOutputStream("./data/config/config_window.ini");
            props.store(output, "Saved settings");
            output.close();
            load();
        } catch (Exception var4) {
        }

    }
}
