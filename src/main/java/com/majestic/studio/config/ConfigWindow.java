package com.majestic.studio.config;

import jfork.nproperty.Cfg;
import jfork.nproperty.CfgIgnore;
import jfork.nproperty.ConfigParser;
import com.majestic.studio.util.DebugUtil;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.lang.reflect.InvocationTargetException;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.List;
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
    public static String RECENT_DIRECTORIES = "";

    public static void load() {
        try {
            parse(_instance, "./data/config/config_window.ini");
        } catch (IllegalAccessException | InstantiationException | NoSuchMethodException | InvocationTargetException | IOException e) {
            DebugUtil.getLogger().error("Failed to load configuration file.", e);
        }

    }

    public static List<File> getRecentDirectories() {
        List<File> directories = new ArrayList<>();
        for (String path : RECENT_DIRECTORIES.split("\n")) {
            if (!path.isEmpty()) {
                File directory = new File(path).toPath().toAbsolutePath().normalize().toFile();
                if (!directories.contains(directory)) {
                    directories.add(directory);
                    if (directories.size() == 10) {
                        break;
                    }
                }
            }
        }
        return directories;
    }

    public static void rememberDirectory(File directory) {
        File normalized = directory.toPath().toAbsolutePath().normalize().toFile();
        List<File> directories = getRecentDirectories();
        directories.remove(normalized);
        directories.add(0, normalized);
        if (directories.size() > 10) {
            directories.remove(directories.size() - 1);
        }
        RECENT_DIRECTORIES = String.join("\n", directories.stream().map(File::getPath).toList());
        save("RECENT_DIRECTORIES", RECENT_DIRECTORIES);
    }

    public static void save(String key, String var) {
        try {
            Properties props = new Properties();
            File configFile = new File(PATH);
            if (configFile.isFile()) {
                try (FileInputStream input = new FileInputStream(configFile)) {
                    props.load(input);
                }
            }
            props.setProperty(key, var);
            Files.createDirectories(configFile.toPath().getParent());
            try (FileOutputStream output = new FileOutputStream(configFile)) {
                props.store(output, "Saved settings");
            }
            load();
        } catch (IOException e) {
            DebugUtil.getLogger().error("Failed to save configuration file.", e);
        }

    }
}
