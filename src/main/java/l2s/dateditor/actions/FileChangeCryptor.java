package l2s.dateditor.actions;

import l2s.dateditor.Boot;
import l2s.dateditor.clientcryptor.DatFile;
import l2s.dateditor.clientcryptor.crypt.DatCrypter;
import l2s.dateditor.clientcryptor.crypt.RSADatCrypter;
import l2s.dateditor.config.ConfigWindow;
import l2s.dateditor.util.DebugUtil;
import l2s.dateditor.util.Pair;
import l2s.dateditor.xml.CryptVersionParser;
import org.apache.commons.io.FileUtils;

import java.io.File;
import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;

public class FileChangeCryptor extends ActionTask {
    private final File[] files;
    private final String path;

    public FileChangeCryptor(Boot boot, File[] files, String path) {
        super(boot);
        this.files = files;
        this.path = path;
    }

    protected void action() {
        DatCrypter encrypter = CryptVersionParser.getInstance().getEncryptKey(ConfigWindow.CURRENT_ENCRYPT);
        if (!(encrypter instanceof RSADatCrypter)) {
            Boot.addErrorConsole("Selected encryptor not RSA! Please select RSA encryptor.", true);
        } else if (!encrypter.isEncrypt()) {
            Boot.addErrorConsole("Selected encryptor dont have encrypt RSA key.", true);
        } else {
            int filesCount = getFilesCount(this.files, encrypter);
            double progress = this.getCurrentProgress();
            this.addProgress(progress, 1.0F, 100.0F);
            String backupDirPath = this.path + "/backup";
            File backupDir = new File(backupDirPath);
            if (!backupDir.exists() && !backupDir.mkdir()) {
                Boot.addErrorConsole("Cannot create backup directory [" + backupDirPath + "].", true);
            } else {
                List<DatCrypter> decryptors = new ArrayList<>();

                for (DatCrypter crypter : CryptVersionParser.getInstance().getDecryptKeys().values()) {
                    if (this.isCancelled()) {
                        return;
                    }

                    if (crypter instanceof RSADatCrypter && !crypter.getName().equals(encrypter.getName())) {
                        decryptors.add(crypter);
                    }
                }

                if (decryptors.isEmpty()) {
                    Boot.addErrorConsole("Decryptors not found for encryptor: " + encrypter.getName(), true);
                } else {
                    this.addProgress(progress, 2.0F, 100.0F);
                    long startTime = System.currentTimeMillis();
                    Boot.addLogConsole("---------------------------------------", true);
                    Boot.addLogConsole("Files to recrypt: " + filesCount, true);
                    this.changeCrypt(this.files, encrypter, null, decryptors, backupDir, progress, (double) 100.0F / (double) filesCount);
                    long diffTime = (System.currentTimeMillis() - startTime) / 1000L;
                    Boot.addLogConsole("Completed. Elapsed ".concat(String.valueOf(diffTime)).concat(" sec"), true);
                }
            }
        }
    }

    private static int getFilesCount(File[] files, DatCrypter encrypter) {
        int count = 0;

        for (File file : files) {
            if (file.isDirectory()) {
                count += getFilesCount(Objects.requireNonNull(file.listFiles()), encrypter);
            } else if (encrypter.checkFileExtension(file.getName())) {
                ++count;
            }
        }

        return count;
    }

    private double changeCrypt(File[] files, DatCrypter encrypter, DatCrypter prevDecryptor, List<DatCrypter> decryptors, File backupDir, double progress, double progressWeight) {
        for (File file : files) {
            if (this.isCancelled()) {
                Boot.addErrorConsole("Cancelled.", true);
                return progress;
            }

            if (file.isDirectory()) {
                if (!file.equals(backupDir)) {
                    progress = this.changeCrypt(Objects.requireNonNull(file.listFiles()), encrypter, prevDecryptor, decryptors, new File(backupDir, file.getName()), progress, progressWeight);
                }
            } else if (encrypter.checkFileExtension(file.getName())) {
                try {
                    Pair<ByteBuffer, DatCrypter> decrypted = OpenDat.decrypt(file, prevDecryptor, decryptors, false);
                    if (decrypted == null) {
                        continue;
                    }

                    if (decrypted.getSecond() != null) {
                        prevDecryptor = decrypted.getSecond();
                    }

                    FileUtils.copyFile(file, new File(backupDir, file.getName()));
                    DatFile.encrypt(decrypted.getFirst().array(), file.getPath(), encrypter);
                    Boot.addLogConsole(file.getName() + " change crypt by " + encrypter.getName() + " encryptor success.", true);
                } catch (Exception e) {
                    Boot.addErrorConsole(file.getName() + " change crypt by " + encrypter.getName() + " encryptor failed!", true);
                    if (Boot.MASS_FULL_LOG) {
                        DebugUtil.getLogger().error(e.getMessage(), e);
                    }
                } finally {
                    progress = this.addProgress(progress, progressWeight, 97.0F);
                }

                Boot.addLogConsole("---------------------------------------", true);
            }
        }

        return progress;
    }
}
