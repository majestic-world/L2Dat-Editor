package com.majestic.studio.util;

import com.majestic.studio.config.ConfigDebug;
import com.majestic.studio.xml.Variant;
import org.apache.log4j.LogManager;
import org.apache.log4j.Logger;
import org.apache.log4j.xml.DOMConfigurator;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;

public class DebugUtil {
    private static final Logger _log = LogManager.getLogger(DebugUtil.class);

    public static void debug(String message) {
        if (ConfigDebug.DAT_DEBUG_MSG) {
            _log.info(message);
        }

    }

    public static void debugPos(int pos, String name, Variant val) {
        if (ConfigDebug.DAT_DEBUG_POS) {
            _log.info("pos: " + pos + " " + name + ": " + val);
            if (ConfigDebug.DAT_DEBUG_POS_LIMIT != 0 && pos > ConfigDebug.DAT_DEBUG_POS_LIMIT) {
                System.exit(0);
            }
        }

    }

    public static void save(ByteBuffer buffer, File path) {
        if (ConfigDebug.SAVE_DECODE) {
            try {
                String unpackDirPath = path.getParent() + "/!decrypted";
                File decryptedDir = new File(unpackDirPath);
                decryptedDir.mkdir();
                File file = new File(decryptedDir + "/" + path.getName());
                FileOutputStream fos = new FileOutputStream(file);
                fos.write(buffer.array());
                fos.close();
            } catch (IOException e) {
                e.printStackTrace();
            }
        }

    }

    public static Logger getLogger() {
        return _log;
    }

    static {
        DOMConfigurator.configure("./data/config/log4j.xml");
    }
}
