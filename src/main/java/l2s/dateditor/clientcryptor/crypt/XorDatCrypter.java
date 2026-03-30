package l2s.dateditor.clientcryptor.crypt;

import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;

public class XorDatCrypter extends DatCrypter {
    private final boolean encrypt;
    private final int xorKey;
    private ByteArrayOutputStream _result;

    public XorDatCrypter(String name, int code, int key, boolean deCrypt) {
        super(name, code);
        this.encrypt = !deCrypt;
        this.xorKey = key;
    }

    public ByteBuffer decryptResult() {
        return ByteBuffer.wrap(this._result.toByteArray());
    }

    public ByteBuffer encryptResult() {
        return ByteBuffer.wrap(this._result.toByteArray());
    }

    public boolean update(byte[] bArray) throws Exception {
        for (byte b : bArray) {
            this._result.write(b ^ this.xorKey);
        }

        return true;
    }

    public int getChunkSize(int available) {
        return 1;
    }

    public int getSkipSize() {
        return 0;
    }

    public boolean isEncrypt() {
        return this.encrypt;
    }

    public void aquire() {
        super.aquire();
        this._result = new ByteArrayOutputStream(128);
    }
}
