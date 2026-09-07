package com.majestic.studio.clientcryptor;

import com.majestic.studio.clientcryptor.crypt.DatCrypter;
import com.majestic.studio.config.ConfigDebug;

import java.io.BufferedInputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.zip.CRC32;

public class DatFile extends File {
    private ByteBuffer _buff;
    private Footer _foot;

    public DatFile(String pathname) {
        super(pathname);
    }

    public static void encrypt(byte[] buff, String file, DatCrypter crypter) throws Exception {
        crypter.aquire();
        FileOutputStream os = new FileOutputStream(file, false);
        String header = "Lineage2Ver" + crypter.getCode();
        os.write(header.getBytes(StandardCharsets.UTF_16LE));
        crypter.update(buff);
        byte[] res = crypter.encryptResult().array();
        os.write(res);
        if (ConfigDebug.DAT_ADD_END_BYTES) {
            byte[] endBytes = new byte[20];
            endBytes[19] = 100;
            os.write(endBytes);
        }

        os.close();
        crypter.release();
    }

    public ByteBuffer getBuff() {
        return this._buff;
    }

    public void decrypt(DatCrypter crypter) throws Exception {
        this.loadInfo();

        try {
            BufferedInputStream fis = new BufferedInputStream(new FileInputStream(this), 65536);
            Throwable var3 = null;

            try {
                crypter.aquire();

                try {
                    fis.skip(28L);
                    byte[] buff = new byte[crypter.getChunkSize(fis.available())];
                    int len = fis.available() - crypter.getSkipSize();

                    do {
                        if (len <= 0) {
                            this._buff = crypter.decryptResult();
                            return;
                        }

                        len -= fis.read(buff);
                    } while (crypter.update(buff));

                    this._buff = null;
                } finally {
                    crypter.release();
                }
            } catch (Throwable var24) {
                var3 = var24;
                throw var24;
            } finally {
                if (fis != null) {
                    if (var3 != null) {
                        try {
                            fis.close();
                        } catch (Throwable var22) {
                            var3.addSuppressed(var22);
                        }
                    } else {
                        fis.close();
                    }
                }

            }

        } catch (Exception e) {
            throw e;
        }
    }

    private boolean checkCrc32(DatCrypter crypter) {
        if (this._foot == null) {
            try {
                this.loadInfo();
            } catch (IOException var7) {
                return false;
            }
        }

        try {
            FileInputStream fis = new FileInputStream(this);
            CRC32 chksum = new CRC32();
            byte[] buff = new byte[1024];
            int len = fis.available() - 20;

            while (len > 0) {
                if (len < 1024) {
                    buff = new byte[len];
                }

                len -= fis.read(buff);
                chksum.update(buff);
            }

            return chksum.getValue() == this._foot.crc32;
        } catch (Exception e) {
            e.printStackTrace();
            return false;
        }
    }

    private void loadInfo() throws IOException {
        if (!this.exists() || !this.canRead()) {
            throw new IOException("Can not read the dat file");
        }
        try (FileInputStream fis = new FileInputStream(this)) {
            if (fis.available() < 28) {
                throw new IOException("Can not read the dat file : too small");
            }
            byte[] head = new byte[28];
            fis.read(head);
            String header = new String(head, StandardCharsets.UTF_16LE);
            if (!header.startsWith("Lineage2Ver")) {
                throw new IOException("Can not read the dat file : wrong header");
            }
            if (header.endsWith("111") || header.endsWith("120") || header.endsWith("211")
                    || header.endsWith("212") || header.endsWith("311")) {
                return;
            }
            if (!header.endsWith("411") && !header.endsWith("412") && !header.endsWith("413") && !header.endsWith("414")) {
                throw new IOException("Can not read the dat file : unknown header : '" + header + "'");
            }
            if (fis.available() < 20) {
                throw new IOException("Can not read the dat file : too small");
            }
            fis.skip((long) (fis.available() - 20));
            byte[] foot = new byte[20];
            fis.read(foot);
            int min = foot[4] & 255;
            min += foot[5] << 8 & '\uff00';
            min += foot[6] << 16 & 16711680;
            min += foot[7] << 24 & -16777216;
            int maj = foot[8] & 255;
            maj += foot[9] << 8 & '\uff00';
            maj += foot[10] << 16 & 16711680;
            maj += foot[11] << 24 & -16777216;
            long crc = (long) foot[12] & 255L;
            crc += (long) (foot[13] << 8) & 65280L;
            crc += (long) (foot[14] << 16) & 16711680L;
            crc += (long) (foot[15] << 24) & 4278190080L;
            this._foot = new Footer(crc, min, maj);
        }
    }

    private static class Footer {
        long crc32;
        int majorVersion;
        int minorVersion;

        Footer(long crc, int maj, int min) {
            this.crc32 = crc;
            this.majorVersion = maj;
            this.minorVersion = min;
        }
    }
}
