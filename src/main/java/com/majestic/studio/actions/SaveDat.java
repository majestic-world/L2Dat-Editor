package com.majestic.studio.actions;

import com.majestic.studio.Boot;
import com.majestic.studio.clientcryptor.DatFile;
import com.majestic.studio.clientcryptor.crypt.DatCrypter;
import com.majestic.studio.config.ConfigDebug;
import com.majestic.studio.config.ConfigWindow;
import com.majestic.studio.data.GameDataName;
import com.majestic.studio.util.DebugUtil;
import com.majestic.studio.xml.Descriptor;
import com.majestic.studio.xml.DescriptorParser;
import com.majestic.studio.xml.DescriptorWriter;

import java.io.File;
import java.io.FileOutputStream;

public class SaveDat extends ActionTask {
    private final File inputFile;
    private final File outputFile;
    private final String text;

    public SaveDat(Boot boot, File inputFile, File outputFile, String text) {
        super(boot);
        this.inputFile = inputFile;
        this.outputFile = outputFile;
        this.text = text;
    }

    public SaveDat(Boot boot, File file) {
        this(boot, file, file, boot.getTextPaneMain().getText().replace("\n", "\r\n"));
    }

    protected void action() {
        long startTime = System.currentTimeMillis();
        byte[] buff = null;
        DatCrypter encrypter = null;
        double progress = this.getCurrentProgress();
        if (!this.outputFile.getName().endsWith(".dat") && !this.inputFile.getName().endsWith(".txt")) {
            if (!this.outputFile.getName().endsWith(".ini")) {
                Boot.addErrorConsole("Unknown file " + this.outputFile.getAbsolutePath() + " type!", true);
                return;
            }

            encrypter = this.boot.getEncryptor(this.inputFile);
            if (encrypter == null) {
                return;
            }

            if (this.isCancelled()) {
                return;
            }

            buff = this.text.getBytes();
        } else {
            try {
                Descriptor desc = DescriptorParser.getInstance().findDescriptorForFile(ConfigWindow.CURRENT_STRUCTURE, this.outputFile.getName(), Boot.DEV_MODE);
                if (desc != null) {
                    encrypter = this.boot.getEncryptor(this.outputFile);
                    if (encrypter == null) {
                        Boot.addErrorConsole("Not found the encryptor for chronicle: " + ConfigWindow.CURRENT_STRUCTURE + ",  of the file: " + this.outputFile.getAbsolutePath(), true);
                        return;
                    }

                    buff = DescriptorWriter.parseData(this, 90.0F, this.outputFile, encrypter, desc, this.text, false);
                    if (this.isCancelled()) {
                        return;
                    }

                    progress = this.addProgress(progress, 90.0F, 100.0F);
                    GameDataName.getInstance().checkAndUpdate(this.outputFile.getParent(), encrypter);
                    if (this.isCancelled()) {
                        return;
                    }

                    progress = this.addProgress(progress, 5.0F, 100.0F);
                } else {
                    Boot.addErrorConsole("Not found the structure of the file: " + this.outputFile.getAbsolutePath(), true);
                }
            } catch (Exception e) {
                DebugUtil.getLogger().error(e.getMessage(), e);
                return;
            }
        }

        if (buff != null) {
            if (!this.isCancelled()) {
                try {
                    if (ConfigDebug.ENCRYPT) {
                        DatFile.encrypt(buff, this.outputFile.getPath(), encrypter);
                    } else {
                        FileOutputStream os = new FileOutputStream(this.outputFile.getPath(), false);
                        os.write(buff);
                        os.close();
                    }
                } catch (Exception e) {
                    DebugUtil.getLogger().error(e.getMessage(), e);
                    return;
                }

                if (!this.isCancelled()) {
                    this.addProgress(progress, 5.0F, 100.0F);
                    long diffTime = (System.currentTimeMillis() - startTime) / 1000L;
                    Boot.addLogConsole("Packed successfully by " + encrypter.getName() + " encrypter. Elapsed ".concat(String.valueOf(diffTime)).concat(" sec"), true);
                }
            }
        } else {
            Boot.addErrorConsole("buff == null.", true);
        }
    }
}
