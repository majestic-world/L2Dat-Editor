package com.majestic.studio.clientcryptor.crypt;

import java.nio.ByteBuffer;

public class BlowFishDatCrypter extends DatCrypter {
    private boolean encrypt = false;
    private final BlowfishEngine blowfish = new BlowfishEngine();

    public BlowFishDatCrypter(String name, int code, String key, boolean deCrypt) {
        super(name, code);
        this.encrypt = !deCrypt;
        this.blowfish.init(this.encrypt, key.getBytes());
    }

    public ByteBuffer decryptResult() {
        return null;
    }

    public ByteBuffer encryptResult() {
        return null;
    }

    public boolean update(byte[] b) throws Exception {
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
