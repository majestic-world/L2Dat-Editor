package l2s.dateditor.actions;

import l2s.dateditor.Boot;
import l2s.dateditor.clientcryptor.DatFile;
import l2s.dateditor.clientcryptor.crypt.DatCrypter;
import l2s.dateditor.config.ConfigWindow;
import l2s.dateditor.data.GameDataName;
import l2s.dateditor.util.DebugUtil;
import l2s.dateditor.util.Pair;
import l2s.dateditor.xml.CryptVersionParser;
import l2s.dateditor.xml.Descriptor;
import l2s.dateditor.xml.DescriptorParser;
import l2s.dateditor.xml.DescriptorReader;

import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.channels.FileChannel;
import java.nio.charset.StandardCharsets;
import java.util.Collection;
import java.util.HashMap;
import java.util.Map;
import java.util.logging.Level;
import java.util.logging.Logger;

public class OpenDat extends ActionTask {
    private static final Map<String, DatCrypter> LAST_DAT_CRYPTERS = new HashMap<>();
    protected final File file;

    public static DatCrypter getLastDatCrypter(File file) {
        return LAST_DAT_CRYPTERS.get(file.getAbsolutePath().toLowerCase());
    }

    public OpenDat(Boot boot, File file) {
        super(boot);
        this.file = file;
    }

    protected void action() {
        this.boot.getTextPaneMain().cleanUp();
        Pair<String, DatCrypter> result = null;

        try {
            result = decryptToTxt(this, 100.0F, this.file, null, false);
        } catch (Exception ex) {
            Logger.getLogger(Boot.class.getName()).log(Level.SEVERE, null, ex);
        } finally {
            this.boot.setEditorText(result != null ? result.getFirst() : "");
            this.boot.getTextPaneMain().discardAllEdits();
        }

    }

    public static Pair<ByteBuffer, DatCrypter> decrypt(File file, DatCrypter prevDecryptor, Collection<DatCrypter> decryptors, boolean mass) throws Exception {
        String fileName = file.getName();
        if (!file.exists()) {
            if (!mass) {
                Boot.addErrorConsole("File " + fileName + " does not exist.", true);
            }

            return null;
        } else if (!file.canRead()) {
            if (!mass) {
                Boot.addErrorConsole("Unable to read " + fileName + " file.", true);
            }

            return null;
        } else {
            FileInputStream fis = new FileInputStream(file);
            if (fis.available() < 28) {
                Boot.addErrorConsole("The file " + fileName + " is too small.", true);
                return null;
            } else {
                boolean crypt = true;
                byte[] head = new byte[28];
                fis.read(head);
                fis.close();
                String header = new String(head, StandardCharsets.UTF_16LE);
                if (!header.matches("Lineage2Ver(\\d{3})")) {
                    if (mass) {
                        return null;
                    }

                    Boot.addLogConsole("File " + fileName + " not encrypted. Skip decrypt.", true);
                    crypt = false;
                }

                ByteBuffer buffer = null;
                DatCrypter crypter = null;
                if (crypt) {
                    int cryptCode = Integer.parseInt(header.replaceFirst("Lineage2Ver(\\d{3})", "$1"));
                    if (!mass) {
                        Boot.addLogConsole("File " + fileName + " encrypted. " + header + " decrypt ...", true);
                    }

                    if (prevDecryptor != null && prevDecryptor.getCode() == cryptCode) {
                        try {
                            DatFile dat = new DatFile(file.getPath());
                            dat.decrypt(prevDecryptor);
                            buffer = dat.getBuff();
                            if (buffer != null) {
                                crypter = prevDecryptor;
                            }
                        } catch (Exception ignored) {
                        }
                    }

                    if (crypter == null) {
                        for (DatCrypter c : decryptors) {
                            if (c.getCode() == cryptCode) {
                                try {
                                    DatFile dat = new DatFile(file.getPath());
                                    dat.decrypt(c);
                                    buffer = dat.getBuff();
                                    if (buffer != null) {
                                        crypter = c;
                                        break;
                                    }
                                } catch (Exception ignored) {
                                }
                            }
                        }
                    }

                    if (crypter == null) {
                        if (!mass) {
                            Boot.addErrorConsole("Error decrypt " + fileName + " file.", true);
                        }

                        return null;
                    }

                    LAST_DAT_CRYPTERS.put(file.getAbsolutePath().toLowerCase(), crypter);
                    DebugUtil.save(buffer, file);
                    if (!mass) {
                        Boot.addLogConsole("Decrypt " + fileName + " file successfully by " + crypter.getName() + " decrypter.", true);
                    }
                } else {
                    try {
                        FileInputStream fIn = new FileInputStream(file);
                        Throwable var32 = null;

                        try {
                            FileChannel fChan = fIn.getChannel();
                            ByteBuffer mBuf = ByteBuffer.allocate((int) fChan.size());
                            fChan.read(mBuf);
                            buffer = mBuf;
                            fChan.close();
                        } catch (Throwable var25) {
                            var32 = var25;
                            throw var25;
                        } finally {
                            if (var32 != null) {
                                try {
                                    fIn.close();
                                } catch (Throwable var24) {
                                    var32.addSuppressed(var24);
                                }
                            } else {
                                fIn.close();
                            }

                        }
                    } catch (IOException var29) {
                        Boot.addErrorConsole("Error reading" + fileName + "  file.", true);
                    }
                }

                return buffer == null ? null : new Pair<>(buffer, crypter);
            }
        }
    }

    public static Pair<ByteBuffer, DatCrypter> decrypt(File file, DatCrypter prevDecryptor, boolean mass) throws Exception {
        return decrypt(file, prevDecryptor, CryptVersionParser.getInstance().getDecryptKeys().values(), mass);
    }

    public static Pair<String, DatCrypter> decryptToTxt(ActionTask actionTask, double weight, File file, DatCrypter prevDecryptor, boolean mass) throws Exception {
        Pair<ByteBuffer, DatCrypter> decrypted = decrypt(file, prevDecryptor, mass);
        if (decrypted == null) {
            return new Pair<>("", prevDecryptor);
        } else {
            ByteBuffer buffer = decrypted.getFirst();
            double progress = actionTask.getCurrentProgress();
            DatCrypter crypter = getLastDatCrypter(file);
            String fileName = file.getName();
            String text = null;
            if (!fileName.contains(".ini") && !fileName.contains(".txt")) {
                if (fileName.contains(".htm")) {
                    if (buffer.hasArray()) {
                        text = new String(buffer.array(), StandardCharsets.UTF_16);
                        if (actionTask.isCancelled()) {
                            return null;
                        }

                        progress = actionTask.addProgress(progress, 99.0F, weight);
                    }
                } else {
                    if (crypter == null || !crypter.isUseStructure()) {
                        if (!mass) {
                            Boot.addErrorConsole("Unknown file " + fileName + " type!", true);
                        }

                        return null;
                    }

                    Descriptor desc = DescriptorParser.getInstance().findDescriptorForFile(ConfigWindow.CURRENT_STRUCTURE, fileName, !mass && Boot.DEV_MODE);
                    if (desc == null) {
                        Boot.addErrorConsole("Not found descriptor in structures: " + ConfigWindow.CURRENT_STRUCTURE + ", for file: " + fileName, true);
                        return null;
                    }

                    if (actionTask.isCancelled()) {
                        return null;
                    }

                    if (!mass) {
                        Boot.addLogConsole("Read the file structure by pattern: " + desc.getFilePattern() + "...", true);
                    }

                    progress = actionTask.addProgress(progress, 5.0F, weight);
                    buffer.position(0);
                    DebugUtil.debug("Buffer size: " + buffer.limit());
                    if (!mass) {
                        GameDataName.getInstance().clear();
                    }

                    text = DescriptorReader.getInstance().parseData(actionTask, actionTask.getWeightValue(94.0F, weight), file, crypter, desc, buffer, mass);
                    System.gc();
                    if (actionTask.isCancelled()) {
                        return null;
                    }

                    progress = actionTask.addProgress(progress, 94.0F, weight);

                    if (text == null) {
                        if (!mass) {
                            Boot.addErrorConsole("Structure is not found in the directory: " + ConfigWindow.CURRENT_STRUCTURE + " file: " + fileName, true);
                        }

                        return null;
                    }
                }
            } else if (buffer.hasArray()) {
                text = new String(buffer.array(), StandardCharsets.UTF_8);
            }

            actionTask.addProgress(progress, 1.0F, weight);
            if (!mass) {
                Boot.addLogConsole("Completed.", true);
            }

            if (text == null) {
                return null;
            } else {
                return new Pair<>(text, decrypted.getSecond());
            }
        }
    }
}
