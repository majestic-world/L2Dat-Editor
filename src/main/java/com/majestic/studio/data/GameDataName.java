package com.majestic.studio.data;

import com.majestic.studio.Boot;
import com.majestic.studio.clientcryptor.DatFile;
import com.majestic.studio.clientcryptor.crypt.DatCrypter;
import com.majestic.studio.config.ConfigDebug;
import com.majestic.studio.util.ByteReader;
import com.majestic.studio.util.ByteWriter;
import com.majestic.studio.xml.CryptVersionParser;
import com.majestic.studio.xml.ParamNode;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;
import java.util.TreeMap;
import java.util.concurrent.locks.Lock;
import java.util.concurrent.locks.ReentrantLock;

public class GameDataName {
    private final Lock lock = new ReentrantLock();
    private final Map<Integer, String> names = new TreeMap<>();
    private final Map<String, Integer> nameHash = new HashMap<>();
    private boolean updated = false;
    private File currDataNameFile = null;

    public static GameDataName getInstance() {
        return GameDataName.SingletonHolder._instance;
    }

    private void load(File currentFile, DatCrypter decCrypter) throws Exception {
        this.lock.lock();

        try {
            this.names.clear();
            this.nameHash.clear();
            if (decCrypter.isEncrypt()) {
                DatCrypter crypter = CryptVersionParser.getInstance().getDecryptKey(decCrypter.getName());
                if (crypter == null) {
                    Boot.addErrorConsole("GameDataName: Not found the decryptor for encryptor: " + decCrypter.getName() + ", code: " + decCrypter.getCode(), true);
                    return;
                }

                decCrypter = crypter;
            }

            File file = new File(currentFile.getParent(), "L2GameDataName.dat");
            if (!file.equals(this.currDataNameFile)) {
                if (!file.exists()) {
                    Boot.addErrorConsole("GameDataName: File '" + file.getAbsolutePath() + "' not found!", true);
                } else {
                    FileInputStream fis = new FileInputStream(file);
                    if (fis.available() < 28) {
                        Boot.addErrorConsole(file.getAbsolutePath() + " The file is too small.", true);
                        return;
                    }

                    byte[] head = new byte[28];
                    fis.read(head);
                    fis.close();
                    String header = new String(head, StandardCharsets.UTF_16LE);
                    if (!header.startsWith("Lineage2Ver")) {
                        Boot.addLogConsole("GameDataName: File " + file.getAbsolutePath() + " not encrypted. Skip decrypt.", true);
                        return;
                    }

                    if (Integer.parseInt(header.substring(11)) != decCrypter.getCode()) {
                        Boot.addLogConsole("GameDataName: File " + file.getName() + " encrypted code: " + header + ". Skip decrypt.", true);
                        return;
                    }

                    Boot.addLogConsole("Unpacking [" + file.getAbsolutePath() + "]", true);
                    DatFile dat = new DatFile(file.getAbsolutePath());
                    dat.decrypt(decCrypter);
                    ByteBuffer buff = dat.getBuff();
                    if (buff == null) {
                        Boot.addErrorConsole("GameDataName: File " + file.getAbsolutePath() + " encrypted code: " + header + " cant decrypt by encryptor: " + decCrypter.getName(), true);
                        return;
                    }

                    int size = ByteReader.readUInt(buff);

                    for (int i = 0; i < size; ++i) {
                        String name = ByteReader.readUtfString(buff, false);
                        this.names.put(i, name);
                        if (!this.nameHash.containsKey(name.toLowerCase())) {
                            this.nameHash.put(name.toLowerCase(), i);
                        }
                    }

                    try {
                        String str = ByteReader.readString(buff, true);
                        if (!str.equals("SafePackage")) {
                            Boot.addErrorConsole("SafePackage descriptor not found 'SafePackage' end-tag in file: " + currentFile.getName(), true);
                            return;
                        }
                    } catch (Exception var15) {
                        Boot.addErrorConsole("SafePackage descriptor not found 'SafePackage' end-tag in file: " + currentFile.getName(), true);
                        return;
                    }

                    Boot.addLogConsole("GameDataName: Load " + this.names.size() + " count from file: " + file.getAbsolutePath(), true);
                }

                this.currDataNameFile = file;
                this.updated = false;
            }
        } finally {
            this.lock.unlock();
        }
    }

    public String getString(File currentFile, DatCrypter crypter, int index, boolean mass) throws Exception {
        this.lock.lock();

        String var6;
        try {
            if (this.currDataNameFile == null) {
                this.load(currentFile, crypter);
            }

            if (!mass && !this.names.containsKey(index) && this.currDataNameFile.exists()) {
                Boot.addErrorConsole("GameDataName: Not found string for index: " + index, true);
            }

            String val = this.names.getOrDefault(index, "<StrID:" + index + ">");
            if (!mass && val.isEmpty()) {
                Boot.addErrorConsole("GameDataName: String name Empty!!! Index: " + index + ", file: " + currentFile.getName(), true);
            }

            var6 = "[" + val + "]";
        } finally {
            this.lock.unlock();
        }

        return var6;
    }

    public int getId(File currentFile, DatCrypter crypter, ParamNode node, String str, boolean mass) throws Exception {
        if ((!str.startsWith("[") || !str.endsWith("]")) && !mass) {
            Boot.addErrorConsole("GameDataName: String name not brackets!!! file: " + currentFile.getName() + " str: " + str + " node: " + node, true);
        }

        str = str.substring(1, str.length() - 1);
        if (str.isEmpty()) {
            if (!mass) {
                Boot.addErrorConsole("GameDataName: String name Empty!!! file: " + currentFile.getName() + ", node: " + node, true);
            }

            return -1;
        } else {
            this.lock.lock();

            int newIndex;
            try {
                if (this.currDataNameFile == null) {
                    this.load(currentFile, crypter);
                }

                if (!this.nameHash.containsKey(str.toLowerCase())) {
                    if (str.matches("^<StrID:(\\d+)>$")) {
                        newIndex = Integer.parseInt(str.replaceAll("^<StrID:(\\d+)>$", "$1"));
                        return newIndex;
                    }

                    newIndex = this.names.size();
                    this.names.put(newIndex, str);
                    this.nameHash.put(str.toLowerCase(), newIndex);
                    this.updated = true;
                    int var7 = newIndex;
                    return var7;
                }

                newIndex = this.nameHash.get(str.toLowerCase());
            } finally {
                this.lock.unlock();
            }

            return newIndex;
        }
    }

    public void checkAndUpdate(String currentDir, DatCrypter crypter) throws Exception {
        this.lock.lock();

        try {
            if (!this.updated) {
                Boot.addLogConsole("GameDataName: Does not require updating. Repacking skipped.", true);
            } else if (!this.nameHash.isEmpty()) {
                ByteArrayOutputStream baos = new ByteArrayOutputStream();
                baos.write(ByteWriter.writeInt(this.names.size()));

                for (String key : this.names.values()) {
                    baos.write(ByteWriter.writeUtfString(key, false));
                }

                byte[] resultBytes = baos.toByteArray();
                baos.write(ByteWriter.writeString("SafePackage", true));
                String file = currentDir + "\\L2GameDataName.dat";
                if (ConfigDebug.ENCRYPT) {
                    DatFile.encrypt(baos.toByteArray(), file, crypter);
                } else {
                    FileOutputStream os = new FileOutputStream(file, false);
                    os.write(resultBytes);
                    os.close();
                }

                Boot.addLogConsole("GameDataName: Packed " + this.names.size() + " count in file: " + file, true);
            }
        } finally {
            this.lock.unlock();
        }

    }

    public void clear() {
        this.lock.lock();

        try {
            this.names.clear();
            this.nameHash.clear();
            this.currDataNameFile = null;
            this.updated = false;
        } finally {
            this.lock.unlock();
        }

    }

    private static class SingletonHolder {
        static final GameDataName _instance = new GameDataName();
    }
}
