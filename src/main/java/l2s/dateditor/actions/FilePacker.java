package l2s.dateditor.actions;

import l2s.dateditor.Boot;
import l2s.dateditor.clientcryptor.DatFile;
import l2s.dateditor.clientcryptor.crypt.DatCrypter;
import l2s.dateditor.clientcryptor.crypt.RSADatCrypter;
import l2s.dateditor.config.ConfigDebug;
import l2s.dateditor.config.ConfigWindow;
import l2s.dateditor.data.GameDataName;
import l2s.dateditor.util.DebugUtil;
import l2s.dateditor.xml.CryptVersionParser;
import l2s.dateditor.xml.Descriptor;
import l2s.dateditor.xml.DescriptorParser;
import l2s.dateditor.xml.DescriptorWriter;
import org.apache.commons.io.FileUtils;

import java.io.File;
import java.io.FileFilter;
import java.io.FileOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.Objects;
import java.util.logging.Level;
import java.util.logging.Logger;

public class FilePacker extends ActionTask {
    private static final FileFilter FILE_FILER = (f) -> {
        if (!Boot.INCLUDE_ARENA) {
            String fileName = f.getName().toLowerCase();
            return !fileName.matches("^.+_arena(-\\w+)?\\.\\w+$");
        }

        return true;
    };
    private final File[] files;
    private final File outputDir;

    public FilePacker(Boot boot, File[] files, File outputDir) {
        super(boot);
        this.files = files;
        this.outputDir = outputDir;
    }

    protected void action() {
        try {
            this.action0();
        } catch (Exception ex) {
            Logger.getLogger(Boot.class.getName()).log(Level.SEVERE, null, ex);
        }

    }

    public void action0() throws Exception {
        Boot.addLogConsole("Packer with using '" + ConfigWindow.CURRENT_STRUCTURE + "' structures.", true);
        DatCrypter encrypter = CryptVersionParser.getInstance().getEncryptKey(ConfigWindow.CURRENT_ENCRYPT);
        if (!(encrypter instanceof RSADatCrypter)) {
            Boot.addErrorConsole("Selected encryptor not RSA! Please select RSA encryptor.", true);
        } else if (!encrypter.isEncrypt()) {
            Boot.addErrorConsole("Selected encryptor dont have encrypt RSA key.", true);
        } else {
            double progress = this.getCurrentProgress();
            progress = this.addProgress(progress, 1.0F, 100.0F);
            long startTime = System.currentTimeMillis();
            progress = this.addProgress(progress, 1.0F, 100.0F);
            double progressWeight = (double) 100.0F / (double) getFilesCount(this.files, encrypter);
            progress = this.pack(encrypter, this.files, this.outputDir, progress, progressWeight);
            this.addProgress(progress, 3.0F, 100.0F);
            long diffTime = (System.currentTimeMillis() - startTime) / 1000L;
            Boot.addLogConsole("Completed. Elapsed ".concat(String.valueOf(diffTime)).concat(" sec"), true);
        }
    }

    private static int getFilesCount(File[] files, DatCrypter encrypter) {
        int count = 0;

        for (File file : files) {
            if (file.isDirectory()) {
                count += getFilesCount(Objects.requireNonNull(file.listFiles()), encrypter);
            } else if (FILE_FILER.accept(file) && encrypter.checkFileExtension(file.getName())) {
                ++count;
            }
        }

        return count;
    }

    private double pack(DatCrypter encrypter, File[] files, File outputDir, double progress, double progressWeight) throws Exception {
        GameDataName.getInstance().clear();

        for (File file : files) {
            if (this.isCancelled()) {
                Boot.addErrorConsole("Cancelled.", true);
                return progress;
            }

            if (!file.isDirectory() && FILE_FILER.accept(file) && encrypter.checkFileExtension(file.getName()) && file.getName().equalsIgnoreCase("L2GameDataName.txt")) {
                progress = this.pack0(encrypter, file, outputDir, progress, progressWeight);
                break;
            }
        }

        for (File file : files) {
            if (this.isCancelled()) {
                Boot.addErrorConsole("Cancelled.", true);
                return progress;
            }

            if (!file.isDirectory() && FILE_FILER.accept(file) && encrypter.checkFileExtension(file.getName()) && !file.getName().equalsIgnoreCase("L2GameDataName.txt")) {
                progress = this.pack0(encrypter, file, outputDir, progress, progressWeight);
            }
        }

        GameDataName.getInstance().checkAndUpdate(outputDir.getPath(), encrypter);

        for (File file : files) {
            if (this.isCancelled()) {
                Boot.addErrorConsole("Cancelled.", true);
                return progress;
            }

            if (file.isDirectory() && !file.equals(outputDir)) {
                progress = this.pack(encrypter, Objects.requireNonNull(file.listFiles()), new File(outputDir, file.getName()), progress, progressWeight);
            }
        }

        return progress;
    }

    private double pack0(DatCrypter encrypter, File file, File outputDir, double progress, double progressWeight) {
        Boot.addLogConsole("Start packing [" + file.getName() + "]...", true);

        try {
            byte[] buff = null;
            File outFile = new File(outputDir, file.getName().replace(".txt", ".dat"));

            label199:
            {
                double v;
                try {
                    if (!file.getName().endsWith(".dat") && !file.getName().endsWith(".txt")) {
                        if (!file.getName().endsWith(".ini")) {
                            Boot.addErrorConsole("Unknown file [" + file.getName() + "] type!", true);
                            return progress;
                        }

                        String readFileToString = FileUtils.readFileToString(file, StandardCharsets.UTF_8);
                        readFileToString = readFileToString.replace("\n", "\r\n");
                        buff = readFileToString.getBytes();
                        break label199;
                    }

                    try {
                        Descriptor desc = DescriptorParser.getInstance().findDescriptorForFile(ConfigWindow.CURRENT_STRUCTURE, file.getName().replace(".txt", ".dat"), false);
                        if (desc != null) {
                            String joined = FileUtils.readFileToString(file, StandardCharsets.UTF_8);
                            buff = DescriptorWriter.parseData(this, this.getWeightValue(progressWeight * 0.8, 95.0F), outFile, encrypter, desc, joined, true);
                        } else {
                            Boot.addErrorConsole("Not found the structure of the file: " + file.getName(), true);
                        }
                        break label199;
                    } catch (Exception e) {
                        Boot.addErrorConsole("Fail on packing [" + file.getName() + "]", true);
                        if (Boot.MASS_FULL_LOG) {
                            DebugUtil.getLogger().error(e.getMessage(), e);
                        }
                    }

                    v = progress;
                } finally {
                    progress = this.addProgress(progress, progressWeight * 0.8, 95.0F);
                }

                return v;
            }

            try {
                if (buff == null) {
                    return progress;
                }

                try {
                    if (ConfigDebug.ENCRYPT) {
                        DatFile.encrypt(buff, outFile.getPath(), encrypter);
                    } else {
                        FileOutputStream os = new FileOutputStream(outFile, false);
                        os.write(buff);
                        os.close();
                    }

                    Boot.addLogConsole("Success packed [" + file.getName() + "]", true);
                } catch (Exception e) {
                    DebugUtil.getLogger().error(e.getMessage(), e);
                }
            } finally {
                progress = this.addProgress(progress, progressWeight * 0.2, 95.0F);
            }
        } catch (Exception e) {
            DebugUtil.getLogger().error(e.getMessage(), e);
        }

        return progress;
    }
}
