use crate::model::Setting;
use crate::safety::canonicalize_setting;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{Cursor, Read};

pub const PREFIX: &str = "PRS1:";
pub const MAX_CODE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_UNCOMPRESSED_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SETTINGS: usize = 10_000;
const MAX_VALUE_BYTES: usize = 64 * 1024;
const MAX_VALUE_DEPTH: usize = 16;
const CHECKSUM_BYTES: usize = 32;
const INVALID_CODE: &str = "This share code appears to be damaged or unsupported.";
const UNSAFE_SETTINGS: &str = "Some selected settings cannot be shared safely.";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShareMetadata {
    pub minecraft_version: Option<String>,
    pub lunar_version: Option<String>,
    pub platform: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShareEnvelope {
    pub format_version: u32,
    pub created_at: String,
    pub application_version: String,
    pub metadata: ShareMetadata,
    pub settings: Vec<Setting>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodingResult {
    pub code: String,
    pub compressed_bytes: usize,
    pub uncompressed_bytes: usize,
    pub setting_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireEnvelope {
    format_version: u32,
    created_at: String,
    application_version: String,
    metadata: ShareMetadata,
    #[serde(deserialize_with = "deserialize_settings")]
    settings: Vec<WireSetting>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireSetting {
    source: String,
    file_kind: String,
    profile: String,
    pointer: String,
    #[serde(deserialize_with = "deserialize_scalar")]
    value: Value,
}

fn deserialize_settings<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<WireSetting>, D::Error> {
    struct SettingsVisitor;
    impl<'de> Visitor<'de> for SettingsVisitor {
        type Value = Vec<WireSetting>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a bounded array of settings")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            if sequence
                .size_hint()
                .is_some_and(|length| length > MAX_SETTINGS)
            {
                return Err(de::Error::custom("setting count limit exceeded"));
            }
            let mut settings =
                Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(MAX_SETTINGS));
            while let Some(setting) = sequence.next_element::<WireSetting>()? {
                if settings.len() >= MAX_SETTINGS {
                    return Err(de::Error::custom("setting count limit exceeded"));
                }
                settings.push(setting);
            }
            Ok(settings)
        }
    }
    deserializer.deserialize_seq(SettingsVisitor)
}

fn deserialize_scalar<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Value, D::Error> {
    struct ScalarVisitor;
    impl<'de> Visitor<'de> for ScalarVisitor {
        type Value = Value;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a bounded scalar setting value")
        }

        fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
            Ok(Value::Bool(value))
        }

        fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
            Ok(Value::Number(value.into()))
        }

        fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
            Ok(Value::Number(value.into()))
        }

        fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
            serde_json::Number::from_f64(value)
                .map(Value::Number)
                .ok_or_else(|| E::custom("non-finite setting value"))
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
            if value.len() > (MAX_VALUE_BYTES - 2) / 6 {
                return Err(E::custom("setting value limit exceeded"));
            }
            Ok(Value::String(value.to_string()))
        }

        fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
            if value.len() > (MAX_VALUE_BYTES - 2) / 6 {
                return Err(E::custom("setting value limit exceeded"));
            }
            Ok(Value::String(value))
        }
    }
    deserializer.deserialize_any(ScalarVisitor)
}

impl From<&ShareEnvelope> for WireEnvelope {
    fn from(envelope: &ShareEnvelope) -> Self {
        Self {
            format_version: envelope.format_version,
            created_at: envelope.created_at.clone(),
            application_version: envelope.application_version.clone(),
            metadata: envelope.metadata.clone(),
            settings: envelope
                .settings
                .iter()
                .map(|setting| WireSetting {
                    source: setting.source.clone(),
                    file_kind: setting.file_kind.clone(),
                    profile: setting.profile.clone(),
                    pointer: setting.pointer.clone(),
                    value: setting.value.clone(),
                })
                .collect(),
        }
    }
}

impl From<WireEnvelope> for ShareEnvelope {
    fn from(envelope: WireEnvelope) -> Self {
        Self {
            format_version: envelope.format_version,
            created_at: envelope.created_at,
            application_version: envelope.application_version,
            metadata: envelope.metadata,
            settings: envelope
                .settings
                .into_iter()
                .map(|setting| Setting {
                    id: String::new(),
                    label: String::new(),
                    source: setting.source,
                    category: String::new(),
                    group: String::new(),
                    file_kind: setting.file_kind,
                    profile: setting.profile,
                    pointer: setting.pointer,
                    value: setting.value,
                })
                .collect(),
        }
    }
}

pub fn encode(
    settings: Vec<Setting>,
    mut metadata: ShareMetadata,
) -> Result<EncodingResult, String> {
    if metadata.platform.len() > 16 {
        return Err(UNSAFE_SETTINGS.to_string());
    }
    metadata.platform = metadata.platform.to_ascii_lowercase();
    metadata.minecraft_version = metadata
        .minecraft_version
        .as_deref()
        .map(normalize_game_version)
        .transpose()
        .map_err(|_| UNSAFE_SETTINGS.to_string())?;
    metadata.lunar_version = metadata
        .lunar_version
        .as_deref()
        .map(normalize_game_version)
        .transpose()
        .map_err(|_| UNSAFE_SETTINGS.to_string())?;
    validate_metadata(&metadata).map_err(|_| UNSAFE_SETTINGS.to_string())?;
    let settings = prepare_settings(settings)?;
    let envelope = ShareEnvelope {
        format_version: 1,
        created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        application_version: env!("CARGO_PKG_VERSION").to_string(),
        metadata,
        settings,
    };
    encode_envelope(&envelope)
}

pub fn decode(code: &str) -> Result<ShareEnvelope, String> {
    decode_checked(code).map_err(|_| INVALID_CODE.to_string())
}

fn prepare_settings(settings: Vec<Setting>) -> Result<Vec<Setting>, String> {
    if settings.is_empty() || settings.len() > MAX_SETTINGS {
        return Err("Select between 1 and 10,000 settings to create a share code.".to_string());
    }
    for setting in &settings {
        validate_record_size(setting).map_err(|_| UNSAFE_SETTINGS.to_string())?;
    }
    let profiles: BTreeSet<(String, String)> = settings
        .iter()
        .map(|setting| (setting.source.clone(), setting.profile.clone()))
        .collect();
    let mut counters = BTreeMap::<String, usize>::new();
    let profile_map: BTreeMap<(String, String), String> = profiles
        .into_iter()
        .map(|identity| {
            let counter = counters.entry(identity.0.clone()).or_default();
            *counter += 1;
            (identity, format!("profile-{counter}"))
        })
        .collect();
    let mut identities = BTreeSet::new();
    let mut prepared = Vec::with_capacity(settings.len());
    let mut budget = 0;
    for mut setting in settings {
        setting.profile = profile_map
            .get(&(setting.source.clone(), setting.profile.clone()))
            .ok_or_else(|| UNSAFE_SETTINGS.to_string())?
            .clone();
        let mut canonical =
            canonicalize_setting(&setting).map_err(|_| UNSAFE_SETTINGS.to_string())?;
        canonical.id = anonymous_id(&canonical);
        if !identities.insert(canonical.id.clone()) {
            return Err("The selection contains a duplicate setting.".to_string());
        }
        budget += record_budget(&canonical);
        if budget > MAX_UNCOMPRESSED_BYTES {
            return Err("This selection is too large for a share code.".to_string());
        }
        prepared.push(canonical);
    }
    prepared.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(prepared)
}

fn encode_envelope(envelope: &ShareEnvelope) -> Result<EncodingResult, String> {
    let serialized = rmp_serde::to_vec(&WireEnvelope::from(envelope))
        .map_err(|_| "These settings could not be encoded.".to_string())?;
    if serialized.len() > MAX_UNCOMPRESSED_BYTES {
        return Err("This selection is too large for a share code.".to_string());
    }
    let compressed = zstd::stream::encode_all(Cursor::new(&serialized), 3)
        .map_err(|_| "These settings could not be compressed.".to_string())?;
    let code = frame_payload(&compressed);
    if code.len() > MAX_CODE_BYTES {
        return Err("This selection is too large for a share code.".to_string());
    }
    Ok(EncodingResult {
        code,
        compressed_bytes: compressed.len(),
        uncompressed_bytes: serialized.len(),
        setting_count: envelope.settings.len(),
    })
}

fn frame_payload(compressed: &[u8]) -> String {
    let mut framed = Vec::with_capacity(CHECKSUM_BYTES + compressed.len());
    framed.extend_from_slice(&Sha256::digest(compressed));
    framed.extend_from_slice(compressed);
    format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(framed))
}

fn decode_checked(code: &str) -> Result<ShareEnvelope, ()> {
    if code.len() > MAX_CODE_BYTES {
        return Err(());
    }
    let encoded = code.trim().strip_prefix(PREFIX).ok_or(())?;
    if encoded.is_empty()
        || !encoded
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(());
    }
    let framed = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| ())?;
    if framed.len() <= CHECKSUM_BYTES {
        return Err(());
    }
    let (checksum, compressed) = framed.split_at(CHECKSUM_BYTES);
    if checksum != Sha256::digest(compressed).as_slice() {
        return Err(());
    }
    let mut decoder = zstd::stream::read::Decoder::new(Cursor::new(compressed)).map_err(|_| ())?;
    decoder.window_log_max(23).map_err(|_| ())?;
    let mut serialized = Vec::new();
    decoder
        .take((MAX_UNCOMPRESSED_BYTES + 1) as u64)
        .read_to_end(&mut serialized)
        .map_err(|_| ())?;
    if serialized.is_empty() || serialized.len() > MAX_UNCOMPRESSED_BYTES {
        return Err(());
    }
    let mut deserializer = rmp_serde::Deserializer::new(Cursor::new(&serialized));
    deserializer.set_max_depth(32);
    let wire_envelope = WireEnvelope::deserialize(&mut deserializer).map_err(|_| ())?;
    if deserializer.position() != serialized.len() as u64 {
        return Err(());
    }
    let mut envelope = ShareEnvelope::from(wire_envelope);
    if envelope.format_version != 1
        || envelope.settings.is_empty()
        || envelope.settings.len() > MAX_SETTINGS
        || envelope.created_at.len() > 64
        || DateTime::parse_from_rfc3339(&envelope.created_at).is_err()
        || !valid_version(&envelope.application_version)
    {
        return Err(());
    }
    validate_metadata(&envelope.metadata)?;
    let mut identities = BTreeSet::new();
    for setting in &mut envelope.settings {
        validate_record_size(setting)?;
        if !valid_anonymous_profile(&setting.profile) {
            return Err(());
        }
        let canonical = canonicalize_setting(setting).map_err(|_| ())?;
        setting.id = anonymous_id(setting);
        if !identities.insert(setting.id.clone()) {
            return Err(());
        }
        setting.label = canonical.label;
        setting.category = canonical.category;
        setting.group = canonical.group;
    }
    Ok(envelope)
}

fn validate_metadata(metadata: &ShareMetadata) -> Result<(), ()> {
    if !matches!(metadata.platform.as_str(), "windows" | "macos" | "linux")
        || metadata
            .minecraft_version
            .as_ref()
            .is_some_and(|version| !valid_game_version(version))
        || metadata
            .lunar_version
            .as_ref()
            .is_some_and(|version| !valid_game_version(version))
    {
        return Err(());
    }
    Ok(())
}

fn valid_version(version: &str) -> bool {
    !version.is_empty()
        && version.len() <= 64
        && version.as_bytes()[0].is_ascii_digit()
        && version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte))
}

fn normalize_game_version(version: &str) -> Result<String, ()> {
    if version.len() > 64 {
        return Err(());
    }
    if valid_game_version(version) {
        return Ok(version.to_string());
    }
    let core = version.split('-').next().ok_or(())?;
    if valid_game_version(core) {
        return Ok(core.to_string());
    }
    Err(())
}

fn valid_game_version(version: &str) -> bool {
    if version.len() > 64 {
        return false;
    }
    let parts: Vec<&str> = version.split('.').collect();
    if (2..=4).contains(&parts.len())
        && parts.iter().all(|part| {
            !part.is_empty() && part.len() <= 4 && part.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return true;
    }
    let bytes = version.as_bytes();
    bytes.len() == 6
        && bytes[0..2].iter().all(u8::is_ascii_digit)
        && bytes[2] == b'w'
        && bytes[3..5].iter().all(u8::is_ascii_digit)
        && bytes[5].is_ascii_lowercase()
}

fn valid_anonymous_profile(profile: &str) -> bool {
    profile
        .strip_prefix("profile-")
        .and_then(|number| number.parse::<usize>().ok())
        .is_some_and(|number| {
            number > 0 && number <= MAX_SETTINGS && profile == format!("profile-{number}")
        })
}

fn anonymous_id(setting: &Setting) -> String {
    let mut digest = Sha256::new();
    for component in [
        &setting.source,
        &setting.file_kind,
        &setting.profile,
        &setting.pointer,
    ] {
        digest.update((component.len() as u32).to_le_bytes());
        digest.update(component.as_bytes());
    }
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn validate_record_size(setting: &Setting) -> Result<(), ()> {
    if setting.id.len() > 2_048
        || setting.label.len() > 256
        || setting.source.len() > 32
        || setting.category.len() > 128
        || setting.group.len() > 256
        || setting.file_kind.len() > 64
        || setting.profile.len() > 1_024
        || setting.pointer.len() > 1_024
        || value_budget(&setting.value, 0)? > MAX_VALUE_BYTES
    {
        return Err(());
    }
    Ok(())
}

fn value_budget(value: &Value, depth: usize) -> Result<usize, ()> {
    if depth > MAX_VALUE_DEPTH {
        return Err(());
    }
    let budget = match value {
        Value::Null => 4,
        Value::Bool(_) => 5,
        Value::Number(_) => 32,
        Value::String(value) => value
            .len()
            .checked_mul(6)
            .and_then(|size| size.checked_add(2))
            .ok_or(())?,
        Value::Array(values) => {
            let mut budget: usize = 2;
            for value in values {
                budget = budget
                    .checked_add(value_budget(value, depth + 1)? + 1)
                    .ok_or(())?;
                if budget > MAX_VALUE_BYTES {
                    return Err(());
                }
            }
            budget
        }
        Value::Object(values) => {
            let mut budget: usize = 2;
            for (key, value) in values {
                let key_budget = key
                    .len()
                    .checked_mul(6)
                    .and_then(|size| size.checked_add(4))
                    .ok_or(())?;
                budget = budget
                    .checked_add(key_budget)
                    .and_then(|size| size.checked_add(value_budget(value, depth + 1).ok()?))
                    .ok_or(())?;
                if budget > MAX_VALUE_BYTES {
                    return Err(());
                }
            }
            budget
        }
    };
    Ok(budget)
}

fn record_budget(setting: &Setting) -> usize {
    setting.id.len()
        + setting.label.len()
        + setting.source.len()
        + setting.category.len()
        + setting.group.len()
        + setting.file_kind.len()
        + setting.profile.len()
        + setting.pointer.len()
        + value_budget(&setting.value, 0).unwrap_or(MAX_VALUE_BYTES)
        + 128
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> Setting {
        Setting {
            id: "minecraft:/Users/private-name/game:options:fov".to_string(),
            label: "Private user label".to_string(),
            source: "minecraft".to_string(),
            category: "Private category".to_string(),
            group: "Private group".to_string(),
            file_kind: "options".to_string(),
            profile: "Private player profile".to_string(),
            pointer: "fov".to_string(),
            value: json!("0.5"),
        }
    }

    fn metadata() -> ShareMetadata {
        ShareMetadata {
            minecraft_version: Some("1.21.4".to_string()),
            lunar_version: None,
            platform: "macos".to_string(),
        }
    }

    fn envelope() -> ShareEnvelope {
        decode(&encode(vec![fixture()], metadata()).unwrap().code).unwrap()
    }

    #[test]
    fn round_trip_preserves_safe_settings_and_removes_local_identity() {
        let result = encode(vec![fixture()], metadata()).unwrap();
        assert!(result.code.starts_with(PREFIX));
        assert_eq!(result.setting_count, 1);
        let decoded = decode(&result.code).unwrap();
        assert_eq!(decoded.settings[0].value, json!("0.5"));
        assert_eq!(decoded.settings[0].profile, "profile-1");
        let text = serde_json::to_string(&decoded).unwrap();
        assert!(!text.contains("Private"));
        assert!(!text.contains("private-name"));
        assert!(!text.contains("/Users/"));
    }

    #[test]
    fn malformed_checksum_and_invalid_base64_are_rejected() {
        let result = encode(vec![fixture()], metadata()).unwrap();
        let mut bytes = URL_SAFE_NO_PAD
            .decode(result.code.strip_prefix(PREFIX).unwrap())
            .unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        assert!(decode(&format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes))).is_err());
        assert!(decode("PRS1:%%%bad").is_err());
        assert!(decode("PRS1:").is_err());
        assert!(decode("PRS2:abcd").is_err());
        assert!(decode("unrecognized").is_err());
    }

    #[test]
    fn future_envelope_versions_and_trailing_data_are_rejected() {
        let mut envelope = envelope();
        envelope.format_version = 2;
        assert!(decode(&encode_envelope(&envelope).unwrap().code).is_err());
        envelope.format_version = 1;
        let mut serialized = rmp_serde::to_vec(&WireEnvelope::from(&envelope)).unwrap();
        serialized.extend_from_slice(b"unexpected");
        let compressed = zstd::stream::encode_all(Cursor::new(serialized), 3).unwrap();
        assert!(decode(&frame_payload(&compressed)).is_err());
    }

    #[test]
    fn unsafe_records_are_rejected_even_with_a_valid_checksum() {
        let mut setting = fixture();
        setting.pointer = "accessToken".to_string();
        setting.value = json!("secret-token");
        assert!(encode(vec![setting.clone()], metadata()).is_err());
        let mut envelope = envelope();
        setting.profile = "profile-1".to_string();
        setting.id = anonymous_id(&setting);
        envelope.settings = vec![setting];
        assert!(decode(&encode_envelope(&envelope).unwrap().code).is_err());
    }

    #[test]
    fn duplicate_empty_and_oversized_selections_are_rejected() {
        assert!(encode(Vec::new(), metadata()).is_err());
        assert!(encode(vec![fixture(), fixture()], metadata()).is_err());
        assert!(encode(vec![fixture(); MAX_SETTINGS + 1], metadata()).is_err());
        assert!(decode(&"X".repeat(MAX_CODE_BYTES + 1)).is_err());
        let mut setting = fixture();
        setting.value = json!("X".repeat(MAX_VALUE_BYTES));
        assert!(encode(vec![setting], metadata()).is_err());
    }

    #[test]
    fn decompression_bombs_are_rejected() {
        let compressed =
            zstd::stream::encode_all(Cursor::new(vec![0; MAX_UNCOMPRESSED_BYTES + 1]), 3).unwrap();
        assert!(decode(&frame_payload(&compressed)).is_err());
    }

    #[test]
    fn metadata_paths_and_noncanonical_profiles_are_rejected() {
        let mut metadata = metadata();
        metadata.minecraft_version = Some("/Users/private-name/game".to_string());
        assert!(encode(vec![fixture()], metadata).is_err());
        let mut envelope = envelope();
        envelope.settings[0].profile = "player-name".to_string();
        envelope.settings[0].id = anonymous_id(&envelope.settings[0]);
        assert!(decode(&encode_envelope(&envelope).unwrap().code).is_err());
    }

    #[test]
    fn version_directory_suffixes_are_not_shared() {
        let mut metadata = metadata();
        metadata.minecraft_version = Some("1.21.4-PrivateProfileName".to_string());
        metadata.platform = "macOS".to_string();
        let envelope = decode(&encode(vec![fixture()], metadata).unwrap().code).unwrap();
        assert_eq!(
            envelope.metadata.minecraft_version.as_deref(),
            Some("1.21.4")
        );
        assert_eq!(envelope.metadata.platform, "macos");
        let mut metadata = envelope.metadata;
        metadata.lunar_version = Some("123SecretToken".to_string());
        assert!(encode(vec![fixture()], metadata).is_err());
    }

    #[test]
    fn deep_values_are_rejected_before_serialization() {
        let mut value = json!(true);
        for _ in 0..MAX_VALUE_DEPTH + 1 {
            value = json!([value]);
        }
        let mut setting = fixture();
        setting.value = value;
        assert!(encode(vec![setting], metadata()).is_err());
    }

    #[test]
    fn incoming_arrays_and_excessive_record_counts_are_rejected() {
        let mut envelope = envelope();
        envelope.settings[0].value = json!([true, false]);
        assert!(decode(&encode_envelope(&envelope).unwrap().code).is_err());
        envelope.settings[0].value = json!("0.5");
        envelope.settings = vec![envelope.settings[0].clone(); MAX_SETTINGS + 1];
        assert!(decode(&encode_envelope(&envelope).unwrap().code).is_err());
    }
}
