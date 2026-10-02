use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use prism_relay::codec::{decode, encode, ShareEnvelope, ShareMetadata};
use prism_relay::model::Setting;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
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
    Brotli(u32),
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct CompactPayload {
    format_version: u32,
    timestamp: i64,
    application_version: String,
    minecraft_version: Option<String>,
    lunar_version: Option<String>,
    platform: u8,
    groups: Vec<CompactGroup>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct CompactGroup {
    kind: u8,
    profile: u16,
    settings: Vec<CompactSetting>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
enum CompactSetting {
    Full((String, Value)),
    Prefix((u16, String, Value)),
    Dictionary((i16, Value)),
}

impl From<&Payload> for CompactPayload {
    fn from(payload: &Payload) -> Self {
        let mut groups = BTreeMap::<(u8, u16), Vec<CompactSetting>>::new();
        for setting in &payload.settings {
            let kind = match (setting.source.as_str(), setting.file_kind.as_str()) {
                ("minecraft", "options") => 0,
                ("lunar", "mods") => 1,
                ("lunar", "general") => 2,
                ("lunar", "controls") => 3,
                ("lunar", "performance") => 4,
                _ => unreachable!(),
            };
            let profile = setting
                .profile
                .strip_prefix("profile-")
                .unwrap()
                .parse()
                .unwrap();
            groups
                .entry((kind, profile))
                .or_default()
                .push(CompactSetting::Full((
                    setting.pointer.clone(),
                    setting.value.clone(),
                )));
        }
        Self {
            format_version: 2,
            timestamp: chrono::DateTime::parse_from_rfc3339(&payload.created_at)
                .unwrap()
                .timestamp(),
            application_version: payload.application_version.clone(),
            minecraft_version: payload.metadata.minecraft_version.clone(),
            lunar_version: payload.metadata.lunar_version.clone(),
            platform: 0,
            groups: groups
                .into_iter()
                .map(|((kind, profile), mut settings)| {
                    settings.sort_by(|left, right| {
                        let (CompactSetting::Full((left, _)), CompactSetting::Full((right, _))) =
                            (left, right)
                        else {
                            unreachable!()
                        };
                        left.cmp(right)
                    });
                    CompactGroup {
                        kind,
                        profile,
                        settings,
                    }
                })
                .collect(),
        }
    }
}

fn prefix_payload(payload: &CompactPayload) -> CompactPayload {
    let mut result = payload.clone();
    for group in &mut result.groups {
        let mut previous = String::new();
        for setting in &mut group.settings {
            let CompactSetting::Full((pointer, value)) = setting else {
                unreachable!()
            };
            let full = pointer.clone();
            let mut prefix = previous
                .bytes()
                .zip(full.bytes())
                .take_while(|(a, b)| a == b)
                .count();
            while !full.is_char_boundary(prefix) {
                prefix -= 1;
            }
            if prefix >= 3 {
                *setting = CompactSetting::Prefix((
                    prefix as u16,
                    full[prefix..].to_string(),
                    value.clone(),
                ));
            }
            previous = full;
        }
    }
    result
}

fn dictionary_payload(payload: &CompactPayload) -> CompactPayload {
    let dictionary = prism_relay::codec::POINTER_DICTIONARY
        .iter()
        .enumerate()
        .map(|(index, pointer)| (*pointer, index as i16))
        .collect::<BTreeMap<_, _>>();
    let mut result = payload.clone();
    for group in &mut result.groups {
        for setting in &mut group.settings {
            let CompactSetting::Full((pointer, value)) = setting else {
                unreachable!()
            };
            if let Some(index) = dictionary.get(pointer.as_str()) {
                *setting = CompactSetting::Dictionary((-1 - *index, value.clone()));
            }
        }
    }
    result
}

fn main() {
    println!("fixture,settings,representation,compression,raw_bytes,compressed_bytes,code_chars,encode_us,decode_us");
    let fixtures = [
        ("small", fixture(1)),
        ("multi-profile", fixture(5)),
        ("broad-120", broad_fixture(120)),
        ("broad-all", broad_fixture(10_000)),
        ("large-multi-profile", fixture(25)),
    ];
    for (name, settings) in fixtures {
        let metadata = ShareMetadata {
            hud_viewport: None,
            minecraft_version: Some("1.21.4".to_string()),
            lunar_version: None,
            platform: "windows".to_string(),
        };
        let mut envelope =
            Payload::from(decode(&encode(settings, metadata).unwrap().code).unwrap());
        envelope.format_version = 1;
        envelope.created_at = "2026-09-30T00:00:00Z".to_string();
        let count = envelope.settings.len();
        for serialization in [Serialization::Json, Serialization::MessagePack] {
            for compression in [Compression::Zstd, Compression::Brotli(5)] {
                measure(name, count, "legacy", &envelope, serialization, compression);
            }
        }
        let compact = CompactPayload::from(&envelope);
        for compression in [
            Compression::Zstd,
            Compression::Brotli(5),
            Compression::Brotli(9),
            Compression::Brotli(11),
        ] {
            measure(
                name,
                count,
                "grouped",
                &compact,
                Serialization::MessagePack,
                compression,
            );
        }
        let prefixed = prefix_payload(&compact);
        measure(
            name,
            count,
            "prefix-grouped",
            &prefixed,
            Serialization::MessagePack,
            Compression::Brotli(11),
        );
        let dictionary = dictionary_payload(&compact);
        measure(
            name,
            count,
            "dictionary",
            &dictionary,
            Serialization::MessagePack,
            Compression::Brotli(11),
        );
        let code = measure_adaptive(name, count, &compact);
        let decoded = decode(&code).unwrap();
        assert_eq!(decoded.settings.len(), count);
        for setting in &decoded.settings {
            let original = envelope
                .settings
                .iter()
                .find(|other| {
                    other.source == setting.source
                        && other.file_kind == setting.file_kind
                        && other.profile == setting.profile
                        && other.pointer == setting.pointer
                })
                .unwrap();
            assert_eq!(setting.value, original.value);
        }
    }
}

fn broad_fixture(limit: usize) -> Vec<Setting> {
    let mut records = fixture(1)
        .into_iter()
        .filter(|setting| setting.source == "minecraft")
        .collect::<Vec<_>>();
    let schema: BTreeMap<String, Vec<String>> =
        serde_json::from_str(include_str!("../src/lunar_schema.json")).unwrap();
    for (index, (key, types)) in schema.into_iter().enumerate() {
        if records.len() >= limit {
            break;
        }
        let (file_kind, pointer) = key.split_once(':').unwrap();
        let value = match types[0].as_str() {
            "bool" => json!(index % 3 != 0),
            "number" => {
                if pointer.ends_with("/value") {
                    json!(-16777216_i64 + index as i64 * 251)
                } else {
                    json!((index % 83) as f64 / 128.0 - 0.25)
                }
            }
            "numericString" => json!(format!("{}.50", index % 10)),
            "position" => json!(if index % 2 == 0 {
                "TOP_LEFT"
            } else {
                "BOTTOM_RIGHT"
            }),
            "keybind" => json!("KEY_R"),
            _ => unreachable!(),
        };
        records.push(record(
            "lunar",
            file_kind,
            "synthetic-default",
            pointer,
            value,
        ));
    }
    records
}

fn fixture(profiles: usize) -> Vec<Setting> {
    let minecraft = [
        ("fov", "0.50"),
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

fn serialize<T: Serialize>(envelope: &T, serialization: Serialization) -> Vec<u8> {
    match serialization {
        Serialization::Json => serde_json::to_vec(envelope).unwrap(),
        Serialization::MessagePack => rmp_serde::to_vec(envelope).unwrap(),
    }
}

fn compress(raw: &[u8], compression: Compression) -> Vec<u8> {
    match compression {
        Compression::Zstd => zstd::stream::encode_all(Cursor::new(raw), 3).unwrap(),
        Compression::Brotli(quality) => {
            let mut reader = brotli::CompressorReader::new(Cursor::new(raw), 4096, quality, 22);
            let mut compressed = Vec::new();
            reader.read_to_end(&mut compressed).unwrap();
            compressed
        }
    }
}

fn decompress(compressed: &[u8], compression: Compression) -> Vec<u8> {
    match compression {
        Compression::Zstd => zstd::stream::decode_all(Cursor::new(compressed)).unwrap(),
        Compression::Brotli(_) => {
            let mut reader = brotli::Decompressor::new(Cursor::new(compressed), 4096);
            let mut raw = Vec::new();
            reader.read_to_end(&mut raw).unwrap();
            raw
        }
    }
}

fn frame(compressed: &[u8], prefix: &str) -> String {
    let mut bytes = Sha256::digest(compressed).to_vec();
    bytes.extend_from_slice(compressed);
    format!("{prefix}{}", URL_SAFE_NO_PAD.encode(bytes))
}

fn unframe(code: &str, prefix: &str) -> Vec<u8> {
    let bytes = URL_SAFE_NO_PAD
        .decode(code.strip_prefix(prefix).unwrap())
        .unwrap();
    let (checksum, compressed) = bytes.split_at(32);
    assert_eq!(checksum, Sha256::digest(compressed).as_slice());
    compressed.to_vec()
}

fn measure<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(
    name: &str,
    count: usize,
    representation: &str,
    envelope: &T,
    serialization: Serialization,
    compression: Compression,
) {
    let raw = serialize(envelope, serialization);
    let compressed = compress(&raw, compression);
    let prefix = if representation == "legacy" {
        "PRS1:"
    } else {
        "PRS2:"
    };
    let code = frame(&compressed, prefix);
    let repetitions = 50;
    let start = Instant::now();
    for _ in 0..repetitions {
        let serialized = serialize(envelope, serialization);
        let compressed = compress(&serialized, compression);
        std::hint::black_box(frame(&compressed, prefix));
    }
    let encode_us = start.elapsed().as_secs_f64() * 1_000_000.0 / repetitions as f64;
    let start = Instant::now();
    for _ in 0..repetitions {
        let compressed = unframe(&code, prefix);
        let raw = decompress(&compressed, compression);
        let decoded: T = match serialization {
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
        Compression::Zstd => "zstd-3".to_string(),
        Compression::Brotli(quality) => format!("brotli-{quality}"),
    };
    println!("{name},{count},{representation}-{serialization_name},{compression_name},{},{},{},{encode_us:.1},{decode_us:.1}", raw.len(), compressed.len(), code.len());
}

fn best_compact(payload: &CompactPayload) -> (CompactPayload, Vec<u8>, Vec<u8>) {
    let prefixed = prefix_payload(payload);
    let dictionary = dictionary_payload(payload);
    [dictionary, payload.clone(), prefixed]
        .into_iter()
        .map(|candidate| {
            let raw = rmp_serde::to_vec(&candidate).unwrap();
            let compressed = compress(&raw, Compression::Brotli(11));
            (candidate, raw, compressed)
        })
        .min_by_key(|(_, _, compressed)| compressed.len())
        .unwrap()
}

fn measure_adaptive(name: &str, count: usize, payload: &CompactPayload) -> String {
    let (selected, raw, compressed) = best_compact(payload);
    let code = frame(&compressed, "PRS2:");
    let repetitions = 50;
    let start = Instant::now();
    for _ in 0..repetitions {
        let (_, _, compressed) = best_compact(payload);
        std::hint::black_box(frame(&compressed, "PRS2:"));
    }
    let encode_us = start.elapsed().as_secs_f64() * 1_000_000.0 / repetitions as f64;
    let start = Instant::now();
    for _ in 0..repetitions {
        let compressed = unframe(&code, "PRS2:");
        let raw = decompress(&compressed, Compression::Brotli(11));
        let decoded: CompactPayload = rmp_serde::from_slice(&raw).unwrap();
        assert_eq!(decoded, selected);
        std::hint::black_box(decoded);
    }
    let decode_us = start.elapsed().as_secs_f64() * 1_000_000.0 / repetitions as f64;
    println!(
        "{name},{count},adaptive-messagepack,brotli-11,{},{},{},{encode_us:.1},{decode_us:.1}",
        raw.len(),
        compressed.len(),
        code.len()
    );
    code
}
