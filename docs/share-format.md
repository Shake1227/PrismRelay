# Prism Relay share format

Prism Relay 0.1 uses a self-contained `PRS1:` code. No account, upload service, or database is required. A `.prism` file contains the same UTF-8 code shown in the app.

## Version 1 pipeline

1. Validate every selected setting against the same default-deny rules used by the scanner and importer.
2. Replace local profile names with `profile-1`, `profile-2`, and so on, independently for each source.
3. Keep only the source, file kind, anonymous profile, setting pointer, and approved value for each record. Local IDs, labels, categories, groups, directory names, and file paths are absent from the payload.
4. Serialize the envelope as compact MessagePack arrays.
5. Compress with Zstandard level 3.
6. Prepend the 32-byte SHA-256 digest of the compressed bytes.
7. Encode the digest and compressed bytes using unpadded Base64URL, then prepend `PRS1:`.

The exact layout is:

```text
PRS1:base64url(sha256(compressed_payload) || compressed_payload)
```

The checksum detects accidental damage. It does not authenticate the sender: anyone can create a valid checksum. Decoding therefore validates the full payload and every setting independently before making an import preview available.

## Payload schema

The semantic envelope has these fields, in this order:

```text
[formatVersion, createdAt, applicationVersion, metadata, settings]
metadata = [minecraftVersion, lunarVersion, platform]
setting = [source, fileKind, profile, pointer, value]
```

`formatVersion` is the integer `1`. `createdAt` is an RFC 3339 timestamp. The application version is a bounded version string starting with a digit. Game versions contain two to four numeric components or a Minecraft snapshot identifier such as `24w14a`. Local version-directory suffixes are removed during export so custom profile names do not enter metadata. Optional versions are MessagePack `nil` when unavailable. `platform` is `windows`, `macos`, or `linux`.

`source` is `minecraft` or `lunar`. Minecraft uses `options` as its file kind and an options key as its pointer. Values retain their original string representation. Lunar uses an approved file kind and a JSON Pointer into an observed, allowlisted schema. Lunar values retain their scalar JSON types.

The ordered array layout is part of format version 1; changing it requires a new format version and prefix. There is no compression negotiation inside version 1. A future short sharing ID would be a separate transport, with separate network and privacy controls.

On decode, setting labels and categories are regenerated from the approved pointer. IDs are regenerated with SHA-256 over four UTF-8 components: source, file kind, anonymous profile, and pointer. Each component is preceded by its byte length as an unsigned 32-bit little-endian integer. These IDs identify settings within the decoded code. Import maps approved pointers to an explicitly selected local target profile; shared profile names never become filesystem paths.

## Limits and privacy

| Limit | Version 1 |
| --- | ---: |
| Share-code input | 2 MiB |
| Decompressed payload | 8 MiB |
| Zstandard decoding window | 8 MiB |
| Selected settings | 1–10,000 |
| Per-value conservative serialization budget | 64 KiB |
| Per-value nesting depth | 16 |
| MessagePack nesting depth | 32 |

The decoder bounds input before Base64 allocation and reads no more than 8 MiB plus one detection byte. It limits the settings array while deserializing and accepts only bounded scalar values; nested arrays or objects are rejected before their contents are allocated. Unknown versions, malformed Base64, checksum failures, invalid MessagePack, trailing uncompressed bytes, duplicate records, non-anonymous profiles, and unapproved settings are rejected. A valid checksum does not bypass any limit or whitelist.

Codes contain selected settings and version/platform metadata, rather than the original configuration files. Account files, authentication fields, tokens, server addresses, paths, logs, and unknown fields are excluded. If Resource Packs are selected, approved pack identifiers can include custom pack filenames. Deselect that category when those filenames should remain private. Codes are not encrypted; share them only with the intended recipients.

## Compression comparison

The reproducible benchmark is `src-tauri/examples/codec_benchmark.rs`:

```sh
cd src-tauri
cargo run --release --example codec_benchmark
```

It compares JSON and compact MessagePack representations of the same minimal envelope, each compressed with Zstandard level 3 or Brotli quality 5. The fixtures are synthetic and contain only approved setting names and values, with 24, 120, and 600 records. They include Minecraft video, mouse, audio, and controls plus observed Lunar HUD pointers. They contain no real user configuration or account data.

The timing includes serialization, compression, checksum, and Base64 framing for encode; Base64, checksum verification, decompression, deserialization, and equality verification for decode. Each result averages 200 iterations. The benchmark deserializes a mirror of the payload schema to compare the pipelines; the shipping decoder's whitelist checks and additional structural limits are outside the timing. The setting safety checks run once when constructing each benchmark fixture.

Measured on 2026-09-30 with an Apple M4, macOS 26.5.2, Rust 1.98.1, `rmp-serde` 1.3.1, `zstd` 0.13.3, and `brotli` 8.0.4 in release mode. Timings vary by hardware and configuration. Byte counts are deterministic for these fixtures and the fixed benchmark timestamp.

| Records | Representation | Compression | Raw bytes | Compressed bytes | Code characters | Encode µs | Decode µs |
| ---: | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 24 | JSON | Zstd 3 | 2,663 | 553 | 785 | 31.5 | 23.0 |
| 24 | JSON | Brotli 5 | 2,663 | 447 | 644 | 63.1 | 24.6 |
| 24 | MessagePack | Zstd 3 | 1,151 | 456 | 656 | 20.2 | 10.3 |
| 24 | MessagePack | Brotli 5 | 1,151 | 394 | 573 | 35.8 | 15.3 |
| 120 | JSON | Zstd 3 | 12,603 | 1,122 | 1,544 | 31.5 | 41.1 |
| 120 | JSON | Brotli 5 | 12,603 | 908 | 1,259 | 212.2 | 43.9 |
| 120 | MessagePack | Zstd 3 | 5,559 | 894 | 1,240 | 20.4 | 24.6 |
| 120 | MessagePack | Brotli 5 | 5,559 | 729 | 1,020 | 167.5 | 31.0 |
| 600 | JSON | Zstd 3 | 62,687 | 4,082 | 5,491 | 93.5 | 177.4 |
| 600 | JSON | Brotli 5 | 62,687 | 3,059 | 4,127 | 310.1 | 188.5 |
| 600 | MessagePack | Zstd 3 | 27,983 | 3,293 | 4,439 | 57.9 | 116.7 |
| 600 | MessagePack | Brotli 5 | 27,983 | 2,406 | 3,256 | 232.0 | 128.2 |

MessagePack cuts the uncompressed size by roughly 56% compared with JSON for these fixtures. Brotli produces the smallest codes, while Zstd encodes the MessagePack fixtures about 1.8–8.2 times faster and decodes every fixture faster. Version 1 chooses MessagePack with Zstd level 3 for predictable interactive performance and a decoder with an explicit memory-window bound. A typical 120-setting fixture occupies 1,240 characters including the checksum and prefix. Larger selections should use a `.prism` file when they exceed the app's QR limit.
