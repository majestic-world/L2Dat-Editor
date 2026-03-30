package l2s.dateditor.actions;

import l2s.dateditor.Boot;
import l2s.dateditor.clientcryptor.crypt.DatCrypter;
import l2s.dateditor.config.ConfigWindow;
import l2s.dateditor.data.GameDataName;
import l2s.dateditor.util.DebugUtil;
import l2s.dateditor.util.Pair;
import org.apache.commons.io.FileUtils;

import java.io.File;
import java.io.FileFilter;
import java.io.FileInputStream;
import java.nio.charset.Charset;
import java.nio.charset.StandardCharsets;
import java.util.Objects;
import java.util.logging.Level;
import java.util.logging.Logger;

public class FileUnpacker extends ActionTask {
    private static final FileFilter FILE_FILER = (f) -> {
        String fileName = f.getName().toLowerCase();
        if (fileName.endsWith(".dat")) {
            if (Boot.INCLUDE_ARENA) {
                return true;
            } else {
                return !fileName.matches("^.+_arena(-\\w+)?\\.dat$");
            }
        } else if (fileName.endsWith(".ini")) {
            return true;
        } else {
            return fileName.endsWith(".htm");
        }
    };
    private final File[] files;
    private final File outputDir;

    public FileUnpacker(Boot boot, File[] files, File outputDir) {
        super(boot);
        this.files = files;
        this.outputDir = outputDir;
    }

    public void action() {
        try {
            this.action0();
        } catch (Exception ex) {
            Logger.getLogger(Boot.class.getName()).log(Level.SEVERE, null, ex);
        }

    }

    public void action0() {
        Boot.addLogConsole("Unpacker with using '" + ConfigWindow.CURRENT_STRUCTURE + "' structures.", true);
        GameDataName.getInstance().clear();
        long startTime = System.currentTimeMillis();
        StringBuilder todoDats = new StringBuilder();
        int filesCount = getFilesCount(this.files);
        double progress = this.getCurrentProgress();
        this.addProgress(progress, 1.0F, 100.0F);
        double progressWeight = (double) 100.0F / (double) filesCount;
        this.unpack(this.files, todoDats, this.outputDir, null, progress, progressWeight);
        if (Boot.DEV_MODE) {
            try {
                FileUtils.write(new File("todo_dat.txt"), todoDats.toString(), StandardCharsets.UTF_8);
            } catch (Exception var11) {
            }
        }

        long diffTime = (System.currentTimeMillis() - startTime) / 1000L;
        Boot.addLogConsole("Completed. Elapsed ".concat(String.valueOf(diffTime)).concat(" sec"), true);
    }

    private static int getFilesCount(File[] files) {
        int count = 0;

        for (File file : files) {
            if (file.isDirectory()) {
                count += getFilesCount(Objects.requireNonNull(file.listFiles()));
            } else if (FILE_FILER.accept(file)) {
                ++count;
            }
        }

        return count;
    }

    private double unpack(File[] files, StringBuilder todoDats, File outputDir, DatCrypter prevDecryptor, double progress, double progressWeight) {
        GameDataName.getInstance().clear();

        for (File file : files) {
            if (this.isCancelled()) {
                Boot.addErrorConsole("Cancelled.", true);
                return progress;
            }

            if (!file.isDirectory() && FILE_FILER.accept(file)) {
                try {
                    FileInputStream fis = new FileInputStream(file);
                    Throwable var14 = null;

                    try {
                        Boot.addLogConsole("Start unpacking [" + file.getName() + "]...", true);
                        if (fis.available() < 28) {
                            Boot.addErrorConsole("[" + file.getName() + "] is too small.", true);
                        } else {
                            byte[] head = new byte[28];
                            fis.read(head);
                            fis.close();
                            String header = new String(head, StandardCharsets.UTF_16LE);
                            if (!header.matches("Lineage2Ver41[1-4]")) {
                                Boot.addLogConsole("[" + file.getName() + "] not encrypted. Skip decrypt.", true);
                            } else {
                                Pair<String, DatCrypter> decrypted = OpenDat.decryptToTxt(this, progressWeight, file, prevDecryptor, true);
                                if (decrypted == null) {
                                    todoDats.append(file.getName()).append("\n");
                                    Boot.addErrorConsole("Cannot parse [" + file.getName() + "]", true);
                                } else {
                                    if (decrypted.getSecond() != null) {
                                        prevDecryptor = decrypted.getSecond();
                                    }

                                    String text = decrypted.getFirst();
                                    DatCrypter crypter = OpenDat.getLastDatCrypter(file);
                                    Charset charset = StandardCharsets.UTF_8;
                                    String name = file.getName();
                                    if (crypter.isUseStructure() && file.getName().endsWith(".dat")) {
                                        name = name.replace(".dat", ".txt");
                                    } else if (name.endsWith(".htm")) {
                                        charset = StandardCharsets.UTF_16;
                                    }

                                    FileUtils.write(new File(outputDir, name), text, charset);
                                    Boot.addLogConsole("Success unpacked [" + file.getName() + "]", true);
                                }
                            }
                        }
                    } catch (Throwable var44) {
                        var14 = var44;
                        throw var44;
                    } finally {
                        if (fis != null) {
                            if (var14 != null) {
                                try {
                                    fis.close();
                                } catch (Throwable var43) {
                                    var14.addSuppressed(var43);
                                }
                            } else {
                                fis.close();
                            }
                        }

                    }
                } catch (Exception e3) {
                    DebugUtil.getLogger().error("[" + file.getName() + "] decrypt failed.");
                    if (Boot.MASS_FULL_LOG) {
                        DebugUtil.getLogger().error(e3.getMessage(), e3);
                    }
                } finally {
                    progress = this.addProgress(progress, progressWeight, 100.0F);
                }
            }
        }

        for (File file : files) {
            if (this.isCancelled()) {
                Boot.addErrorConsole("Cancelled.", true);
                return progress;
            }

            if (file.isDirectory() && !file.equals(outputDir)) {
                progress = this.unpack(Objects.requireNonNull(file.listFiles()), todoDats, new File(outputDir, file.getName()), prevDecryptor, progress, progressWeight);
            }
        }

        return progress;
    }
}
