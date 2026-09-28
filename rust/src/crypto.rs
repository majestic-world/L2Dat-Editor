//! Native Lineage II DAT envelopes and the unchanged XML key catalog.
//!
//! RSA has a length-prefixed zlib stream and can reject incorrect keys. Legacy
//! XOR/ECB formats are unauthenticated: without a descriptor or a known file
//! signature, a successful transform cannot establish that its key was correct.
use std::fs;
use std::io::Write;
use std::path::Path;
use std::thread;

use anyhow::{Context, Result, anyhow, bail, ensure};
use blowfish::BlowfishLE;
use cipher::{BlockDecrypt, BlockEncrypt, KeyInit, generic_array::GenericArray};
use des::Des;
use flate2::{Compression, Decompress, FlushDecompress, Status, write::ZlibEncoder};
use num_bigint::BigUint;

const HEADER_PREFIX: &[u8] = b"L\0i\0n\0e\0a\0g\0e\02\0V\0e\0r\0";
const HEADER_LEN: usize = 28;
const FOOTER_LEN: usize = 20;
const RSA_BLOCK: usize = 128;
const RSA_PAYLOAD: usize = 124;
const RSA_MIN_BLOCKS_PER_WORKER: usize = 128;
const RSA_MAX_WORKERS: usize = 8;

pub struct Decoded {
    pub bytes: Vec<u8>,
    pub key_name: Option<String>,
    pub use_structure: bool,
}

pub struct CryptoCatalog {
    keys: Vec<Key>,
    add_footer: bool,
}

struct Key {
    name: String,
    code: u16,
    decrypt: bool,
    use_structure: bool,
    algorithm: Algorithm,
}

enum Algorithm {
    Xor(u8),
    Blowfish(Box<BlowfishLE>),
    Des(Des),
    Rsa { modulus: BigUint, exponent: BigUint },
}

impl CryptoCatalog {
    pub fn load(data_dir: &Path) -> Result<Self> {
        let full = data_dir.join("config/cryptVersion_Full.xml");
        let path = if full.exists() {
            full
        } else {
            data_dir.join("config/cryptVersion.xml")
        };
        let xml = fs::read_to_string(&path)
            .with_context(|| format!("Reading key catalog {}", path.display()))?;
        let mut catalog = Self::from_xml(&xml)?;
        let debug_path = data_dir.join("config/config_debug.ini");
        match fs::read_to_string(&debug_path) {
            Ok(config) => {
                for line in config.lines() {
                    if let Some((name, value)) = line.split_once('=') {
                        if name.trim() == "DAT_ADD_END_BYTES" {
                            catalog.add_footer = value.trim().eq_ignore_ascii_case("true");
                        }
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("Reading DAT footer configuration"),
        }
        Ok(catalog)
    }

    fn from_xml(xml: &str) -> Result<Self> {
        let document = roxmltree::Document::parse(xml).context("Parsing key catalog XML")?;
        ensure!(
            document.root_element().has_tag_name("keys"),
            "Expected keys catalog root"
        );
        let mut keys: Vec<Key> = Vec::new();
        for node in document
            .root_element()
            .children()
            .filter(|node| node.has_tag_name("key"))
        {
            let attribute = |name| {
                node.attribute(name)
                    .ok_or_else(|| anyhow!("Key missing {name} attribute"))
            };
            let name = attribute("name")?.to_owned();
            let code: u16 = attribute("code")?.parse().context("Invalid DAT version")?;
            ensure!(code <= 999, "DAT version must have three decimal digits");
            let decrypt = attribute("decrypt")?.eq_ignore_ascii_case("true");
            let use_structure = attribute("useStructure")?.eq_ignore_ascii_case("true");
            let algorithm = match attribute("type")?.to_ascii_lowercase().as_str() {
                "xor" => Algorithm::Xor(attribute("key")?.parse::<i32>()? as u8),
                "blowfish" => {
                    // XML backslash-zero is two literal key bytes, just as Java getBytes().
                    let cipher = BlowfishLE::new_from_slice(attribute("key")?.as_bytes())
                        .map_err(|_| anyhow!("Invalid Blowfish key length for {name}"))?;
                    Algorithm::Blowfish(Box::new(cipher))
                }
                "des" => {
                    let mut folded = [0u8; 8];
                    for (index, byte) in attribute("key")?.bytes().enumerate() {
                        folded[index % 8] ^= byte;
                    }
                    Algorithm::Des(Des::new(GenericArray::from_slice(&folded)))
                }
                "rsa" => {
                    let modulus = BigUint::parse_bytes(attribute("modulus")?.as_bytes(), 16)
                        .ok_or_else(|| anyhow!("Invalid RSA modulus for {name}"))?;
                    let exponent = BigUint::parse_bytes(attribute("exp")?.as_bytes(), 16)
                        .ok_or_else(|| anyhow!("Invalid RSA exponent for {name}"))?;
                    ensure!(
                        modulus.bits() > 1016 && modulus.bits() <= 1024,
                        "RSA key {name} must use 128-byte blocks"
                    );
                    ensure!(
                        exponent != BigUint::default(),
                        "RSA key {name} has zero exponent"
                    );
                    Algorithm::Rsa { modulus, exponent }
                }
                other => bail!("Unsupported crypto algorithm {other} for {name}"),
            };
            let key = Key {
                name,
                code,
                decrypt,
                use_structure,
                algorithm,
            };
            // LinkedHashMap.put in Java replaces duplicates without moving their position.
            if let Some(index) = keys
                .iter()
                .position(|old| old.name == key.name && old.decrypt == key.decrypt)
            {
                keys[index] = key;
            } else {
                keys.push(key);
            }
        }
        ensure!(!keys.is_empty(), "Key catalog contains no keys");
        Ok(Self {
            keys,
            add_footer: true,
        })
    }

    pub fn encrypt_names(&self) -> Vec<String> {
        self.keys
            .iter()
            .filter(|key| !key.decrypt)
            .map(|key| key.name.clone())
            .collect()
    }

    pub fn encryption_uses_structure(&self, name: &str) -> Result<bool> {
        self.keys
            .iter()
            .find(|key| !key.decrypt && key.name == name)
            .map(|key| key.use_structure)
            .ok_or_else(|| anyhow!("Unknown encryption key {name}"))
    }

    pub fn encryption_for_source(&self, source: &str) -> Result<String> {
        if self
            .keys
            .iter()
            .any(|key| !key.decrypt && key.name == source)
        {
            return Ok(source.to_owned());
        }
        bail!(
            "No encryption key is available for source {source}; select an explicit encryption key"
        )
    }

    pub fn decrypt(&self, bytes: &[u8], filename: &str) -> Result<Decoded> {
        let Some(code) = version(bytes)? else {
            return Ok(Decoded {
                bytes: bytes.to_vec(),
                key_name: None,
                // A plaintext DAT may still contain binary schema records. The
                // application resolves its descriptor; absence of crypto is not text.
                use_structure: !is_text_file(filename),
            });
        };
        let mut errors = Vec::new();
        for key in self
            .keys
            .iter()
            .filter(|key| key.decrypt && key.code == code)
        {
            let result = payload(bytes, matches!(key.algorithm, Algorithm::Rsa { .. }))
                .and_then(|data| key.transform(data, filename, false));
            match result {
                Ok(decoded) => {
                    return Ok(Decoded {
                        bytes: decoded,
                        key_name: Some(key.name.clone()),
                        use_structure: key.use_structure,
                    });
                }
                Err(error) => errors.push(format!("{}: {error:#}", key.name)),
            }
        }
        if errors.is_empty() {
            bail!("No decryption key for DAT version {code:03}");
        }
        bail!(
            "Could not decrypt DAT version {code:03}: {}",
            errors.join("; ")
        )
    }

    pub fn encrypt(&self, bytes: &[u8], filename: &str, key_name: &str) -> Result<Vec<u8>> {
        let key = self
            .keys
            .iter()
            .find(|key| !key.decrypt && key.name == key_name)
            .ok_or_else(|| anyhow!("Unknown encryption key {key_name}"))?;
        let encrypted = key.transform(bytes, filename, true)?;
        let mut output = Vec::with_capacity(
            HEADER_LEN + encrypted.len() + if self.add_footer { FOOTER_LEN } else { 0 },
        );
        for unit in format!("Lineage2Ver{:03}", key.code).encode_utf16() {
            output.extend_from_slice(&unit.to_le_bytes());
        }
        output.extend_from_slice(&encrypted);
        if self.add_footer {
            // Java emits this sentinel without populating CRC/version fields.
            output.resize(output.len() + FOOTER_LEN, 0);
            *output.last_mut().expect("DAT envelope is nonempty") = 100;
        }
        Ok(output)
    }
}

impl Key {
    fn transform(&self, bytes: &[u8], _filename: &str, encrypt: bool) -> Result<Vec<u8>> {
        match &self.algorithm {
            // Preserve this editor's XML-key behavior for every XOR version,
            // including 121: its Java implementation does not derive filename keys.
            Algorithm::Xor(key) => Ok(bytes.iter().map(|byte| byte ^ key).collect()),
            Algorithm::Blowfish(cipher) => Ok(block_transform(cipher.as_ref(), bytes, encrypt)),
            Algorithm::Des(cipher) => Ok(block_transform(cipher, bytes, encrypt)),
            Algorithm::Rsa { modulus, exponent } => {
                if encrypt {
                    rsa_encrypt(bytes, modulus, exponent)
                } else {
                    rsa_decrypt(bytes, modulus, exponent)
                }
            }
        }
    }
}

fn is_text_file(filename: &str) -> bool {
    let extension = filename.rsplit('.').next().unwrap_or_default();
    ["ini", "txt", "htm", "html"]
        .iter()
        .any(|expected| extension.eq_ignore_ascii_case(expected))
}

fn version(bytes: &[u8]) -> Result<Option<u16>> {
    if !bytes.starts_with(HEADER_PREFIX) {
        // Do not reinterpret a truncated recognizable encrypted header as plaintext.
        if bytes.len() >= 2 && bytes.len() < HEADER_PREFIX.len() && HEADER_PREFIX.starts_with(bytes)
        {
            bail!("Truncated DAT encryption header");
        }
        return Ok(None);
    }
    ensure!(bytes.len() >= HEADER_LEN, "Truncated DAT encryption header");
    let mut code = 0u16;
    for pair in bytes[HEADER_PREFIX.len()..HEADER_LEN].chunks_exact(2) {
        ensure!(
            pair[1] == 0 && pair[0].is_ascii_digit(),
            "Invalid DAT encryption version"
        );
        code = code * 10 + u16::from(pair[0] - b'0');
    }
    Ok(Some(code))
}

fn payload(bytes: &[u8], rsa: bool) -> Result<&[u8]> {
    let body = &bytes[HEADER_LEN..];
    if rsa {
        if body.len() >= FOOTER_LEN && (body.len() - FOOTER_LEN) % RSA_BLOCK == 0 {
            validate_footer(bytes)?;
            return Ok(&body[..body.len() - FOOTER_LEN]);
        }
        ensure!(
            body.len() % RSA_BLOCK == 0,
            "Truncated RSA block or DAT footer"
        );
        return Ok(body);
    }
    // Legacy DATs may have no footer. Only the exact Java sentinel or a
    // checksum-bearing recognized footer can be distinguished from payload.
    if body.len() >= FOOTER_LEN {
        let footer = &body[body.len() - FOOTER_LEN..];
        let sentinel = footer[..19].iter().all(|byte| *byte == 0) && footer[19] == 100;
        let checksum_footer =
            footer[..4] == [0; 4] && footer[16..19] == [0; 3] && footer[19] == 100;
        if sentinel || checksum_footer {
            validate_footer(bytes)?;
            return Ok(&body[..body.len() - FOOTER_LEN]);
        }
    }
    Ok(body)
}

fn validate_footer(bytes: &[u8]) -> Result<()> {
    let end = bytes.len() - FOOTER_LEN;
    let footer = &bytes[end..];
    let expected = u32::from_le_bytes(footer[12..16].try_into().expect("20-byte footer"));
    if expected != 0 {
        ensure!(
            crc32fast::hash(&bytes[..end]) == expected,
            "DAT footer CRC32 mismatch"
        );
    }
    Ok(())
}

fn block_transform<C>(cipher: &C, bytes: &[u8], encrypt: bool) -> Vec<u8>
where
    C: BlockEncrypt + BlockDecrypt + cipher::BlockSizeUser<BlockSize = cipher::consts::U8>,
{
    let mut result = bytes.to_vec();
    // No PKCS padding: preserve the final incomplete block byte-for-byte,
    // matching DESDatCrypter. BlowfishEngine also operates on raw 8-byte blocks.
    for chunk in result.chunks_exact_mut(8) {
        let block = GenericArray::from_mut_slice(chunk);
        if encrypt {
            cipher.encrypt_block(block);
        } else {
            cipher.decrypt_block(block);
        }
    }
    result
}

fn rsa_block(
    input: &[u8; RSA_BLOCK],
    modulus: &BigUint,
    exponent: &BigUint,
) -> Result<[u8; RSA_BLOCK]> {
    let value = BigUint::from_bytes_be(input);
    ensure!(&value < modulus, "RSA block exceeds the key modulus");
    let transformed = value.modpow(exponent, modulus).to_bytes_be();
    let mut output = [0u8; RSA_BLOCK];
    ensure!(
        transformed.len() <= RSA_BLOCK,
        "RSA result exceeds the block size"
    );
    output[RSA_BLOCK - transformed.len()..].copy_from_slice(&transformed);
    Ok(output)
}

fn rsa_encrypt(bytes: &[u8], modulus: &BigUint, exponent: &BigUint) -> Result<Vec<u8>> {
    let length =
        u32::try_from(bytes.len()).context("DAT payload exceeds the 32-bit length field")?;
    let mut compressed = Vec::new();
    compressed.extend_from_slice(&length.to_le_bytes());
    let mut encoder = ZlibEncoder::new(compressed, Compression::default());
    encoder
        .write_all(bytes)
        .context("Compressing DAT payload")?;
    let compressed = encoder.finish().context("Finishing DAT compression")?;
    let blocks = compressed.len().div_ceil(RSA_PAYLOAD);
    let workers = (blocks / RSA_MIN_BLOCKS_PER_WORKER).clamp(1, RSA_MAX_WORKERS);
    let workers = if workers > 1 {
        workers.min(thread::available_parallelism().map_or(1, |count| count.get()))
    } else {
        1
    };
    rsa_encrypt_blocks(&compressed, modulus, exponent, workers)
}

fn rsa_encrypt_blocks(
    compressed: &[u8],
    modulus: &BigUint,
    exponent: &BigUint,
    workers: usize,
) -> Result<Vec<u8>> {
    let blocks = compressed.len().div_ceil(RSA_PAYLOAD);
    let mut output = vec![0u8; blocks * RSA_BLOCK];
    if workers == 1 || blocks <= 1 {
        rsa_encrypt_into(compressed, &mut output, modulus, exponent)?;
        return Ok(output);
    }
    let blocks_per_worker = blocks.div_ceil(workers);
    // Workers borrow disjoint output slices. No shared append, key copies or
    // reordering; a failed worker prevents the entire payload from being saved.
    thread::scope(|scope| -> Result<()> {
        let mut chunks = output
            .chunks_mut(blocks_per_worker * RSA_BLOCK)
            .zip(compressed.chunks(blocks_per_worker * RSA_PAYLOAD));
        let (first_output, first_input) = chunks.next().expect("nonempty RSA payload");
        let mut handles = Vec::with_capacity(workers - 1);
        for (output, input) in chunks {
            handles.push(
                thread::Builder::new()
                    .spawn_scoped(scope, move || {
                        rsa_encrypt_into(input, output, modulus, exponent)
                    })
                    .context("Starting RSA encryption worker")?,
            );
        }
        let mut result = rsa_encrypt_into(first_input, first_output, modulus, exponent);
        for handle in handles {
            let worker_result = handle
                .join()
                .map_err(|_| anyhow!("RSA encryption worker panicked"))
                .and_then(|result| result);
            if result.is_ok() {
                result = worker_result;
            }
        }
        result
    })?;
    Ok(output)
}

fn rsa_encrypt_into(
    compressed: &[u8],
    output: &mut [u8],
    modulus: &BigUint,
    exponent: &BigUint,
) -> Result<()> {
    for (chunk, output) in compressed
        .chunks(RSA_PAYLOAD)
        .zip(output.chunks_exact_mut(RSA_BLOCK))
    {
        let mut block = [0u8; RSA_BLOCK];
        block[..4].copy_from_slice(&(chunk.len() as u32).to_be_bytes());
        let start = RSA_BLOCK - chunk.len() - (RSA_PAYLOAD - chunk.len()) % 4;
        block[start..start + chunk.len()].copy_from_slice(chunk);
        output.copy_from_slice(&rsa_block(&block, modulus, exponent)?);
    }
    Ok(())
}

fn rsa_decrypt(bytes: &[u8], modulus: &BigUint, exponent: &BigUint) -> Result<Vec<u8>> {
    ensure!(
        !bytes.is_empty() && bytes.len() % RSA_BLOCK == 0,
        "RSA payload must contain complete 128-byte blocks"
    );
    let mut compressed = Vec::with_capacity(bytes.len());
    for chunk in bytes.chunks_exact(RSA_BLOCK) {
        let block = rsa_block(chunk.try_into().expect("128-byte chunk"), modulus, exponent)?;
        let size = u32::from_be_bytes(block[..4].try_into().expect("4-byte RSA length")) as usize;
        ensure!(
            (1..=RSA_PAYLOAD).contains(&size),
            "Invalid RSA block payload length {size}"
        );
        let start = RSA_BLOCK - size - (RSA_PAYLOAD - size) % 4;
        compressed.extend_from_slice(&block[start..start + size]);
    }
    ensure!(compressed.len() >= 4, "Missing decompressed DAT length");
    let expected =
        u32::from_le_bytes(compressed[..4].try_into().expect("4-byte DAT length")) as usize;
    inflate_exact(&compressed[4..], expected)
}

fn inflate_exact(compressed: &[u8], expected: usize) -> Result<Vec<u8>> {
    let mut inflater = Decompress::new(true);
    let mut result = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let input_offset = inflater.total_in() as usize;
        let output_offset = inflater.total_out();
        let status = inflater
            .decompress(
                &compressed[input_offset..],
                &mut buffer,
                FlushDecompress::None,
            )
            .context("Invalid DAT zlib stream")?;
        let written = (inflater.total_out() - output_offset) as usize;
        ensure!(
            result
                .len()
                .checked_add(written)
                .is_some_and(|length| length <= expected),
            "Inflated DAT exceeds declared size {expected}"
        );
        result.extend_from_slice(&buffer[..written]);
        if status == Status::StreamEnd {
            ensure!(
                inflater.total_in() as usize == compressed.len(),
                "Trailing data after DAT zlib stream"
            );
            ensure!(
                result.len() == expected,
                "Inflated DAT size {} differs from declared {expected}",
                result.len()
            );
            return Ok(result);
        }
        ensure!(
            written != 0 || inflater.total_in() as usize != input_offset,
            "Truncated DAT zlib stream"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> CryptoCatalog {
        CryptoCatalog::from_xml(include_str!("../../dist/data/config/cryptVersion.xml")).unwrap()
    }

    #[test]
    fn xor_known_answer_and_configured_121_key() {
        let key = Key {
            name: String::new(),
            code: 111,
            decrypt: false,
            use_structure: false,
            algorithm: Algorithm::Xor(172),
        };
        assert_eq!(
            key.transform(&[0, 1, 172, 255], "test.dat", true).unwrap(),
            [172, 173, 0, 83]
        );
        let key = Key {
            code: 121,
            algorithm: Algorithm::Xor(230),
            ..key
        };
        assert_eq!(
            key.transform(&[0, 1, 230, 255], "test.dat", true).unwrap(),
            [230, 231, 0, 25]
        );
        assert_eq!(
            key.transform(&[0, 1, 230, 255], "other.utx", true).unwrap(),
            [230, 231, 0, 25]
        );
    }

    #[test]
    fn blowfish_little_endian_known_answer() {
        let cipher = BlowfishLE::new_from_slice(&[0; 8]).unwrap();
        // Standard Blowfish zero-key vector, with each 32-bit word in LE order.
        let expected = [0x45, 0x97, 0xf9, 0x4e, 0x78, 0xdd, 0x98, 0x61];
        assert_eq!(block_transform(&cipher, &[0; 8], true), expected);
        assert_eq!(block_transform(&cipher, &expected, false), [0; 8]);
    }

    #[test]
    fn des_known_answer_and_unpadded_tail() {
        let cipher = Des::new(GenericArray::from_slice(&[
            0x13, 0x34, 0x57, 0x79, 0x9b, 0xbc, 0xdf, 0xf1,
        ]));
        let input = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 3, 4, 5];
        let encrypted = block_transform(&cipher, &input, true);
        assert_eq!(
            encrypted,
            [0x85, 0xe8, 0x13, 0x54, 0x0f, 0x0a, 0xb4, 0x05, 3, 4, 5]
        );
        assert_eq!(block_transform(&cipher, &encrypted, false), input);
    }

    #[test]
    fn all_available_encryption_keys_roundtrip() {
        let catalog = catalog();
        for name in catalog.encrypt_names() {
            for length in [0, 1, 7, 8, 9, 123, 124, 125, 511] {
                let input: Vec<u8> = (0..length)
                    .map(|index| (index * 73 + index / 7) as u8)
                    .collect();
                let encrypted = catalog.encrypt(&input, "Sample.dat", &name).unwrap();
                let decoded = catalog.decrypt(&encrypted, "Sample.dat").unwrap();
                assert_eq!(decoded.bytes, input, "{name}, length {length}");
                assert_eq!(decoded.key_name.as_deref(), Some(name.as_str()));
            }
        }
    }

    #[test]
    fn parallel_rsa_preserves_block_order_and_partial_tails() {
        let catalog = catalog();
        let key = catalog
            .keys
            .iter()
            .find(|key| !key.decrypt && key.name == "v413_encdec")
            .unwrap();
        let Algorithm::Rsa { modulus, exponent } = &key.algorithm else {
            panic!("Expected RSA key");
        };
        for length in [
            RSA_PAYLOAD - 1,
            RSA_PAYLOAD,
            RSA_PAYLOAD + 1,
            RSA_PAYLOAD * 7 + 1,
        ] {
            let input: Vec<u8> = (0..length).map(|index| (index * 37) as u8).collect();
            let serial = rsa_encrypt_blocks(&input, modulus, exponent, 1).unwrap();
            for workers in [2, 4] {
                assert_eq!(
                    rsa_encrypt_blocks(&input, modulus, exponent, workers).unwrap(),
                    serial
                );
            }
        }
        assert!(
            rsa_encrypt_blocks(
                &vec![1; RSA_PAYLOAD * 7],
                &BigUint::from(3233u32),
                &BigUint::from(17u32),
                4
            )
            .is_err(),
            "Worker errors must prevent returning a partial encrypted payload"
        );
    }

    #[test]
    fn large_rsa_payload_roundtrips_with_both_keys() {
        let catalog = catalog();
        let mut state = 0x9e3779b9u32;
        let input: Vec<u8> = (0..32769)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect();
        for name in ["v413_encdec", "v413_encdec_bonux"] {
            let encrypted = catalog.encrypt(&input, "sample.dat", name).unwrap();
            assert!(encrypted.len() > RSA_BLOCK * RSA_MIN_BLOCKS_PER_WORKER * 2);
            let decoded = catalog.decrypt(&encrypted, "sample.dat").unwrap();
            assert_eq!(decoded.bytes, input, "{name}");
            assert_eq!(decoded.key_name.as_deref(), Some(name));
        }
    }

    #[test]
    fn rsa_candidates_reject_wrong_key_and_truncation() {
        let catalog = catalog();
        let encrypted = catalog
            .encrypt(
                b"payload requiring the bonux key",
                "sample.dat",
                "v413_encdec_bonux",
            )
            .unwrap();
        assert_eq!(
            catalog
                .decrypt(&encrypted, "sample.dat")
                .unwrap()
                .key_name
                .as_deref(),
            Some("v413_encdec_bonux")
        );
        for length in [2, 22, 27, 28, 47, encrypted.len() - 1] {
            assert!(
                catalog.decrypt(&encrypted[..length], "sample.dat").is_err(),
                "prefix {length}"
            );
        }
        let wrong = catalog
            .keys
            .iter()
            .find(|key| key.decrypt && key.name == "v413_encdec")
            .unwrap();
        assert!(
            wrong
                .transform(
                    &encrypted[HEADER_LEN..encrypted.len() - FOOTER_LEN],
                    "sample.dat",
                    false
                )
                .is_err()
        );
    }

    #[test]
    fn rsa_big_endian_known_answer() {
        // Textbook RSA: 65^17 mod 3233 = 2790, with leading-zero block padding.
        let mut plain = [0u8; RSA_BLOCK];
        plain[RSA_BLOCK - 1] = 65;
        let encrypted = rsa_block(&plain, &BigUint::from(3233u32), &BigUint::from(17u32)).unwrap();
        let mut expected = [0u8; RSA_BLOCK];
        expected[RSA_BLOCK - 2..].copy_from_slice(&2790u16.to_be_bytes());
        assert_eq!(encrypted, expected);
        assert_eq!(
            rsa_block(&encrypted, &BigUint::from(3233u32), &BigUint::from(2753u32)).unwrap(),
            plain
        );
        assert!(
            rsa_block(
                &[255; RSA_BLOCK],
                &BigUint::from(3233u32),
                &BigUint::from(17u32)
            )
            .is_err()
        );
    }

    #[test]
    fn rsa_without_footer_and_crc_validation() {
        let mut catalog = catalog();
        catalog.add_footer = false;
        let encrypted = catalog
            .encrypt(b"no footer", "sample.dat", "v413_encdec")
            .unwrap();
        assert_eq!(
            catalog.decrypt(&encrypted, "sample.dat").unwrap().bytes,
            b"no footer"
        );
        catalog.add_footer = true;
        let mut encrypted = catalog
            .encrypt(b"CRC protected", "sample.dat", "v413_encdec")
            .unwrap();
        let end = encrypted.len() - FOOTER_LEN;
        let checksum = crc32fast::hash(&encrypted[..end]);
        encrypted[end + 12..end + 16].copy_from_slice(&checksum.to_le_bytes());
        assert_eq!(
            catalog.decrypt(&encrypted, "sample.dat").unwrap().bytes,
            b"CRC protected"
        );
        encrypted[HEADER_LEN] ^= 1;
        assert!(catalog.decrypt(&encrypted, "sample.dat").is_err());
    }

    #[test]
    fn zlib_requires_complete_stream_and_exact_length() {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"complete payload").unwrap();
        let compressed = encoder.finish().unwrap();
        assert_eq!(inflate_exact(&compressed, 16).unwrap(), b"complete payload");
        assert!(inflate_exact(&compressed, 15).is_err());
        assert!(inflate_exact(&compressed, 17).is_err());
        for length in 0..compressed.len() {
            assert!(inflate_exact(&compressed[..length], 16).is_err());
        }
        let mut extra = compressed;
        extra.push(0);
        assert!(inflate_exact(&extra, 16).is_err());
    }

    #[test]
    fn plaintext_structures_and_source_selection() {
        let catalog = catalog();
        let plain = catalog.decrypt(&[1, 0, 0, 0], "itemname-e.dat").unwrap();
        assert!(plain.use_structure);
        assert!(plain.key_name.is_none());
        assert_eq!(plain.bytes, [1, 0, 0, 0]);
        assert!(
            !catalog
                .decrypt(b"[section]", "l2.ini")
                .unwrap()
                .use_structure
        );
        assert!(catalog.encryption_for_source("v413_original").is_err());
        assert_eq!(
            catalog.encryption_for_source("v413_encdec_bonux").unwrap(),
            "v413_encdec_bonux"
        );
        assert!(catalog.encryption_for_source("missing").is_err());
    }
}
