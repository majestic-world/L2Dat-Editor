package com.majestic.studio.clientcryptor;

import com.majestic.studio.clientcryptor.crypt.RSADatCrypter;
import com.majestic.studio.clientcryptor.crypt.XorDatCrypter;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.KeyPairGenerator;
import java.security.interfaces.RSAPrivateKey;
import java.security.interfaces.RSAPublicKey;
import java.util.Random;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

class DatFileBufferingTest {
    @TempDir
    Path directory;

    @ParameterizedTest
    @ValueSource(ints = {0, 1, 65535, 65536, 65537, 131075})
    void preservesXorBytesAtBufferBoundaries(int size) throws Exception {
        byte[] expected = new byte[size];
        new Random(111).nextBytes(expected);
        byte[] encrypted = expected.clone();
        for (int i = 0; i < encrypted.length; i++) {
            encrypted[i] ^= (byte) 172;
        }
        Path file = directory.resolve("xor.dat");
        ByteArrayOutputStream contents = new ByteArrayOutputStream();
        contents.write("Lineage2Ver111".getBytes(StandardCharsets.UTF_16LE));
        contents.write(encrypted);
        Files.write(file, contents.toByteArray());

        DatFile dat = new DatFile(file.toString());
        dat.decrypt(new XorDatCrypter("test", 111, 172, true));

        assertArrayEquals(expected, dat.getBuff().array());
    }

    @Test
    void releasesInputWhenHeaderValidationFails() throws Exception {
        Path file = directory.resolve("invalid.dat");
        Files.write(file, new byte[28]);

        assertThrows(IOException.class, () -> new DatFile(file.toString())
                .decrypt(new XorDatCrypter("test", 111, 172, true)));

        Files.delete(file);
    }

    @Test
    void preservesRsaBlocksAndExcludesFooterAcrossBufferBoundary() throws Exception {
        KeyPairGenerator generator = KeyPairGenerator.getInstance("RSA");
        generator.initialize(1024);
        var keys = generator.generateKeyPair();
        RSAPublicKey publicKey = (RSAPublicKey) keys.getPublic();
        RSAPrivateKey privateKey = (RSAPrivateKey) keys.getPrivate();
        String modulus = publicKey.getModulus().toString(16);
        byte[] expected = new byte[65537];
        new Random(413).nextBytes(expected);
        RSADatCrypter encryptor = new RSADatCrypter("test", 413, modulus, privateKey.getPrivateExponent().toString(16), false);
        ByteArrayOutputStream contents = new ByteArrayOutputStream();
        contents.write("Lineage2Ver413".getBytes(StandardCharsets.UTF_16LE));
        encryptor.aquire();
        try {
            encryptor.update(expected);
            contents.write(encryptor.encryptResult().array());
        } finally {
            encryptor.release();
        }
        byte[] footer = new byte[20];
        new Random(20).nextBytes(footer);
        contents.write(footer);
        Path file = directory.resolve("rsa.dat");
        Files.write(file, contents.toByteArray());

        DatFile dat = new DatFile(file.toString());
        dat.decrypt(new RSADatCrypter("test", 413, modulus, publicKey.getPublicExponent().toString(16), true));

        assertArrayEquals(expected, dat.getBuff().array());
    }
}
