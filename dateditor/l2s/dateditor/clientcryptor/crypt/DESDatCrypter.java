package l2s.dateditor.clientcryptor.crypt;

import javax.crypto.Cipher;
import javax.crypto.SecretKey;
import javax.crypto.SecretKeyFactory;
import javax.crypto.spec.DESKeySpec;
import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;

public class DESDatCrypter extends DatCrypter {
    private final boolean encrypt;
    private ByteArrayOutputStream _result;
    private final Cipher _cipher;

    public DESDatCrypter(String name, int code, String sKey, boolean deCrypt) throws Exception {
        super(name, code);
        this.encrypt = !deCrypt;
        byte[] key = sKey.getBytes();
        byte[] keyXor = new byte[key.length];

        for (int i = 0; i < key.length; ++i) {
            keyXor[i % 8] ^= key[i];
        }

        DESKeySpec dks = new DESKeySpec(keyXor);
        SecretKeyFactory skf = SecretKeyFactory.getInstance("DES");
        SecretKey desKey = skf.generateSecret(dks);
        this._cipher = Cipher.getInstance("DES/ECB/NoPadding");
        this._cipher.init(deCrypt ? 2 : 1, desKey);
    }

    public ByteBuffer decryptResult() {
        return ByteBuffer.wrap(this._result.toByteArray());
    }

    public ByteBuffer encryptResult() {
        return ByteBuffer.wrap(this._result.toByteArray());
    }

    public boolean update(byte[] bArray) throws Exception {
        if (!this.encrypt) {
            this._result = new ByteArrayOutputStream(bArray.length);
            byte[] bytes = new byte[8];

            int size;
            for (int position = 0; position < bArray.length; position += size) {
                size = Math.min(8, bArray.length - position);
                System.arraycopy(bArray, position, bytes, 0, size);
                this._result.write(size == 8 ? this._cipher.doFinal(bytes) : bytes, 0, size);
            }
        }

        return true;
    }

    public int getChunkSize(int available) {
        return available;
    }

    public int getSkipSize() {
        return 0;
    }

    public boolean isEncrypt() {
        return this.encrypt;
    }
}
