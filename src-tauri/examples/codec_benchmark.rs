use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use prism_relay::codec::{decode, encode, ShareEnvelope, ShareMetadata};
use prism_relay::model::Setting;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::time::Instant;

#[derive(Clone, Copy)]
enum Serialization {
    Json,
    MessagePack,
}

#[derive(Clone, Copy)]
enum Compression {
    Zstd,
    Brotli,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Payload {
    format_version: u32,
    created_at: String,
    application_version: String,
    metadata: ShareMetadata,
    settings: Vec<PayloadSetting>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct PayloadSetting {
    source: String,
    file_kind: String,
    profile: String,
    pointer: String,
    value: Value,
}

impl From<ShareEnvelope> for Payload {
    fn from(envelope: ShareEnvelope) -> Self {
        Self {
            format_version: envelope.format_version,
            created_at: envelope.created_at,
            application_version: envelope.application_version,
            metadata: envelope.metadata,
            settings: envelope
                .settings
                .into_iter()
                .map(|setting| PayloadSetting {
                    source: setting.source,
                    file_kind: setting.file_kind,
                    profile: setting.profile,
                    pointer: setting.pointer,
                    value: setting.value,
                })
                .collect(),
        }
    }
}

fn main() {
    println!("fixture,settings,serialization,compression,raw_bytes,compressed_bytes,code_chars,encode_us,decode_us");
    for (name, profiles) in [("small", 1), ("typical", 5), ("large", 25)] {
        let settings = fixture(profiles);
        let metadata = ShareMetadata {
            minecraft_version: Some("1.21.4".to_string()),
            lunar_version: None,
            platform: "windows".to_string(),
        };
        let mut envelope =
            Payload::from(decode(&encode(settings, metadata).unwrap().code).unwrap());
        envelope.created_at = "2026-09-30T00:00:00Z".to_string();
        for serialization in [Serialization::Json, Serialization::MessagePack] {
            for compression in [Compression::Zstd, Compression::Brotli] {
                measure(name, &envelope, serialization, compression);
            }
        }
    }
}

fn fixture(profiles: usize) -> Vec<Setting> {
    let minecraft = [
        ("fov", "0.5"),
        ("guiScale", "3"),
        ("renderDistance", "12"),
        ("simulationDistance", "8"),
        ("maxFps", "240"),
        ("enableVsync", "false"),
        ("gamma", "0.5"),
        ("mouseSensitivity", "0.42"),
        ("rawMouseInput", "true"),
        ("fullscreen", "false"),
        ("key_key.forward", "key.keyboard.w"),
        ("key_key.back", "key.keyboard.s"),
        ("key_key.attack", "key.mouse.left"),
        ("key_key.jump", "key.keyboard.space"),
        ("soundCategory_master", "0.8"),
    ];
    let lunar = [
        ("/FPS/enabled", json!(true)),
        ("/FPS/x", json!(0.25)),
        ("/FPS/y", json!(-0.6)),
        ("/COORDINATES/enabled", json!(true)),
        ("/COORDINATES/x", json!(0.4)),
        ("/KEYSTROKES/x", json!(0.3)),
        ("/KEYSTROKES/y", json!(0.3)),
        ("/CPS/enabled", json!(true)),
        ("/CPS/y", json!(0.25)),
    ];
    let mut settings = Vec::new();
    for index in 0..profiles {
        let profile = format!("synthetic-{index}");
        for (pointer, value) in minecraft {
            settings.push(record(
                "minecraft",
                "options",
                &profile,
                pointer,
                json!(value),
            ));
        }
        for (pointer, value) in &lunar {
            settings.push(record("lunar", "mods", &profile, pointer, value.clone()));
        }
    }
    settings
}

fn record(source: &str, file_kind: &str, profile: &str, pointer: &str, value: Value) -> Setting {
    Setting {
        id: format!("{source}:{profile}:{file_kind}:{pointer}"),
        label: pointer.to_string(),
        source: source.to_string(),
        category: String::new(),
        group: String::new(),
        file_kind: file_kind.to_string(),
        profile: profile.to_string(),
        pointer: pointer.to_string(),
        value,
    }
}

fn serialize(envelope: &Payload, serialization: Serialization) -> Vec<u8> {
    match serialization {
        Serialization::Json => serde_json::to_vec(envelope).unwrap(),
        Serialization::MessagePack => rmp_serde::to_vec(envelope).unwrap(),
    }
}

fn compress(raw: &[u8], compression: Compression) -> Vec<u8> {
    match compression {
        Compression::Zstd => zstd::stream::encode_all(Cursor::new(raw), 3).unwrap(),
        Compression::Brotli => {
            let mut reader = brotli::CompressorReader::new(Cursor::new(raw), 4096, 5, 22);
            let mut compressed = Vec::new();
            reader.read_to_end(&mut compressed).unwrap();
            compressed
        }
    }
}

fn decompress(compressed: &[u8], compression: Compression) -> Vec<u8> {
    match compression {
        Compression::Zstd => zstd::stream::decode_all(Cursor::new(compressed)).unwrap(),
        Compression::Brotli => {
            let mut reader = brotli::Decompressor::new(Cursor::new(compressed), 4096);
            let mut raw = Vec::new();
            reader.read_to_end(&mut raw).unwrap();
            raw
        }
    }
}

fn frame(compressed: &[u8]) -> String {
    let mut bytes = Sha256::digest(compressed).to_vec();
    bytes.extend_from_slice(compressed);
    format!("PRS1:{}", URL_SAFE_NO_PAD.encode(bytes))
}

fn unframe(code: &str) -> Vec<u8> {
    let bytes = URL_SAFE_NO_PAD
        .decode(code.strip_prefix("PRS1:").unwrap())
        .unwrap();
    let (checksum, compressed) = bytes.split_at(32);
    assert_eq!(checksum, Sha256::digest(compressed).as_slice());
    compressed.to_vec()
}

fn measure(name: &str, envelope: &Payload, serialization: Serialization, compression: Compression) {
    let raw = serialize(envelope, serialization);
    let compressed = compress(&raw, compression);
    let code = frame(&compressed);
    let repetitions = 200;
    let start = Instant::now();
    for _ in 0..repetitions {
        let serialized = serialize(envelope, serialization);
        let compressed = compress(&serialized, compression);
        std::hint::black_box(frame(&compressed));
    }
    let encode_us = start.elapsed().as_secs_f64() * 1_000_000.0 / repetitions as f64;
    let start = Instant::now();
    for _ in 0..repetitions {
        let compressed = unframe(&code);
        let raw = decompress(&compressed, compression);
        let decoded: Payload = match serialization {
            Serialization::Json => serde_json::from_slice(&raw).unwrap(),
            Serialization::MessagePack => rmp_serde::from_slice(&raw).unwrap(),
        };
        assert_eq!(&decoded, envelope);
        std::hint::black_box(decoded);
    }
    let decode_us = start.elapsed().as_secs_f64() * 1_000_000.0 / repetitions as f64;
    let serialization_name = match serialization {
        Serialization::Json => "json",
        Serialization::MessagePack => "messagepack",
    };
    let compression_name = match compression {
        Compression::Zstd => "zstd-3",
        Compression::Brotli => "brotli-5",
    };
    println!(
        "{name},{},{serialization_name},{compression_name},{},{},{},{encode_us:.1},{decode_us:.1}",
        envelope.settings.len(),
        raw.len(),
        compressed.len(),
        code.len()
    );
}
