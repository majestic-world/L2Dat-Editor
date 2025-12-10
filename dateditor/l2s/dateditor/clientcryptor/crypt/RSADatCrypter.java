package l2s.dateditor.clientcryptor.crypt;

import l2s.dateditor.util.DebugUtil;
import l2s.dateditor.util.Util;

import javax.crypto.Cipher;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.math.BigInteger;
import java.nio.ByteBuffer;
import java.security.KeyFactory;
import java.security.interfaces.RSAPrivateKey;
import java.security.interfaces.RSAPublicKey;
import java.security.spec.RSAPrivateKeySpec;
import java.security.spec.RSAPublicKeySpec;
import java.util.Arrays;
import java.util.zip.Deflater;
import java.util.zip.DeflaterOutputStream;
import java.util.zip.Inflater;
import java.util.zip.InflaterInputStream;

public class RSADatCrypter extends DatCrypter {
    private Cipher _cipher;
    private ByteArrayOutputStream _result;
    private boolean encrypt = false;

    public RSADatCrypter(String name, int code, String modulus, String exp, boolean deCrypt) {
        super(name, code);

        try {
            this._cipher = Cipher.getInstance("RSA/ECB/nopadding");
            if (deCrypt) {
                RSAPublicKeySpec keyspec = new RSAPublicKeySpec(new BigInteger(modulus, 16), new BigInteger(exp, 16));
                RSAPublicKey rsaKey = (RSAPublicKey) KeyFactory.getInstance("RSA").generatePublic(keyspec);
                this._cipher.init(2, rsaKey);
            } else {
                this.encrypt = true;
                RSAPrivateKeySpec keyspec = new RSAPrivateKeySpec(new BigInteger(modulus, 16), new BigInteger(exp, 16));
                RSAPrivateKey rsaKey = (RSAPrivateKey) KeyFactory.getInstance("RSA").generatePrivate(keyspec);
                this._cipher.init(1, rsaKey);
            }
        } catch (Exception e) {
            e.printStackTrace();
        }

    }

    public ByteBuffer decryptResult() {
        if (!this.checkAquired()) {
            throw new IllegalStateException("Dont even think about using a DatCrypter that you didnt aquired");
        } else {
            byte[] compressed = this._result.toByteArray();
            int inflatedSize = compressed[0] & 255;
            inflatedSize += compressed[1] << 8 & '\uff00';
            inflatedSize += compressed[2] << 16 & 16711680;
            inflatedSize += compressed[3] << 24 & -16777216;
            ByteArrayInputStream bais = new ByteArrayInputStream(compressed, 4, compressed.length - 4);
            InflaterInputStream iis = new InflaterInputStream(bais, new Inflater());
            ByteArrayOutputStream baos = new ByteArrayOutputStream(128);
            byte[] inflatedResult = new byte[128];

            int len;
            try {
                while ((len = iis.read(inflatedResult)) > 0) {
                    baos.write(inflatedResult, 0, len);
                }
            } catch (IOException e) {
                e.printStackTrace();
            }

            if (baos.size() != inflatedSize) {
                DebugUtil.getLogger().error("[RSADatCrypter] Hum inflated result doesnt have the expected length..(" + baos.size() + "!=" + inflatedSize + ")");
            }

            return ByteBuffer.wrap(baos.toByteArray());
        }
    }

    public ByteBuffer encryptResult() {
        if (!this.checkAquired()) {
            throw new IllegalStateException("Dont even think about using a DatCrypter that you didnt aquired");
        } else {
            ByteArrayOutputStream result = new ByteArrayOutputStream();

            try {
                ByteArrayInputStream input = new ByteArrayInputStream(this._result.toByteArray());
                byte[] buffer = new byte[124];
                byte[] block = new byte[128];

                int len;
                while ((len = input.read(buffer)) > 0) {
                    Arrays.fill(block, (byte) 0);
                    block[0] = (byte) (len >> 24 & 255);
                    block[1] = (byte) (len >> 16 & 255);
                    block[2] = (byte) (len >> 8 & 255);
                    block[3] = (byte) (len & 255);
                    System.arraycopy(buffer, 0, block, 128 - len - (124 - len) % 4, len);
                    result.write(this._cipher.doFinal(block));
                }
            } catch (Exception e) {
                e.printStackTrace();
            }

            return ByteBuffer.wrap(result.toByteArray());
        }
    }

    public boolean update(byte[] b) throws Exception {
        if (!this.checkAquired()) {
            throw new IllegalStateException("Dont even think about using a DatCrypter that you didnt aquired");
        } else {
            Exception exception = null;

            try {
                if (!this.encrypt) {
                    byte[] chunk = this._cipher.doFinal(b);
                    int size = chunk[3];
                    size += chunk[2] << 8 & '\uff00';
                    size += chunk[1] << 16 & 16711680;
                    size += chunk[0] << 24 & -16777216;
                    int pad = (-size & 1) + (-size & 2);
                    DebugUtil.debug("Size:" + size + " pad:" + pad);
                    if (size > 128) {
                        return false;
                    }

                    this._result.write(chunk, 128 - size - pad, size);
                    DebugUtil.debug("--- BLOCK:\n" + Util.printData(chunk) + "-----");
                } else {
                    try {
                        ByteArrayOutputStream s = new ByteArrayOutputStream(b.length);
                        DeflaterOutputStream dos = new DeflaterOutputStream(s, new Deflater());
                        dos.write(b);
                        dos.finish();
                        dos.close();
                        int l = b.length;
                        this._result = new ByteArrayOutputStream(10 + s.toByteArray().length);
                        this._result.write(l & 255);
                        this._result.write((l & '\uff00') >> 8);
                        this._result.write((l & 16711680) >> 16);
                        this._result.write((l & -16777216) >> 24);
                        this._result.write(s.toByteArray());
                    } catch (IOException e) {
                        exception = e;
                    }
                }
            } catch (Exception e) {
                exception = e;
            }

            if (exception != null) {
                throw exception;
            } else {
                return true;
            }
        }
    }

    public void aquire() {
        super.aquire();
        this._result = new ByteArrayOutputStream(128);
    }

    public boolean isEncrypt() {
        return this.encrypt;
    }

    public int getChunkSize(int available) {
        return 128;
    }

    public int getSkipSize() {
        return 20;
    }
}
