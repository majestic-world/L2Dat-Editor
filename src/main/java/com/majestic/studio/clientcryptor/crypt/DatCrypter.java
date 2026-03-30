package com.majestic.studio.clientcryptor.crypt;

import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.locks.ReentrantLock;

public abstract class DatCrypter {
    private final ReentrantLock _lock = new ReentrantLock(true);
    private final String name;
    private final int code;
    private boolean useStructure;
    private final List<String> fileEndNames = new ArrayList<>();

    public DatCrypter(String name, int code) {
        this.name = name;
        this.code = code;
    }

    public abstract boolean update(byte[] var1) throws Exception;

    public abstract ByteBuffer decryptResult();

    public abstract ByteBuffer encryptResult();

    public abstract int getChunkSize(int var1);

    public abstract int getSkipSize();

    public boolean checkAquired() {
        return this._lock.isHeldByCurrentThread();
    }

    public void aquire() {
        this._lock.lock();
    }

    public void release() {
        this._lock.unlock();
    }

    public abstract boolean isEncrypt();

    public String getName() {
        return this.name;
    }

    public int getCode() {
        return this.code;
    }

    public void addFileExtension(String n) {
        this.fileEndNames.addAll(Arrays.asList(n.split(";")));
    }

    public boolean checkFileExtension(String n) {
        return n.contains(".") && this.fileEndNames.contains(n.toLowerCase().split("\\.")[1]);
    }

    public boolean isUseStructure() {
        return this.useStructure;
    }

    public void setUseStructure(boolean useStructure) {
        this.useStructure = useStructure;
    }
}
