# Prism Relay share format

Prism Relay exports self-contained `PRS3:` codes when verified Lunar HUD coordinates include a source viewport. Other selections use the shorter `PRS2:` format. Existing `PRS1:` and `PRS2:` codes still decode. No account, upload service, or database is required. A `.prism` file contains the same UTF-8 code shown in the app.

The compact format keeps setting values unchanged. Minecraft values keep their original strings, including numeric formatting such as `0.50`. Lunar values keep their scalar JSON types, integer colors, and floating-point values.

## Version 2 pipeline

1. Validate every selected setting against the scanner and importer's default-deny rules.
2. Replace local profile names with `profile-1`, `profile-2`, and so on, independently for each source.
3. Group records by file kind and anonymous profile, with pointers sorted inside each group.
4. Serialize compact MessagePack arrays. Try full pointers, common-prefix pointers, and frozen dictionary indices; compress each candidate and keep the shortest result.
5. Compress with Brotli quality 11 and a 4 MiB window (`lgwin = 22`).
6. Prepend the complete 32-byte SHA-256 digest of the compressed bytes.
7. Encode the digest and compressed bytes as unpadded Base64URL, then prepend `PRS2:`.

```text
PRS2:base64url(sha256(compressed_payload) || compressed_payload)
```

The checksum detects accidental damage. It does not authenticate the sender: anyone can create a valid checksum. The decoder therefore checks the whole payload and every setting before presenting an import preview.

## Version 2 payload

The ordered array layout is:

```text
[2, timestamp, applicationVersion, minecraftVersion, lunarVersion, platform, groups]
group = [fileKind, profile, settings]
setting = [pointer, value]
        | [negativeDictionaryIndex, value]
        | [prefixByteLength, pointerSuffix, value]
```

`timestamp` is a Unix timestamp in whole seconds. Decoding expands it to the RFC 3339 `createdAt` field used by the app. Optional game versions are MessagePack `nil` when unavailable. Platform codes are `0` for Windows, `1` for macOS, and `2` for Linux. A profile is an integer from 1 to 10,000, expanded to `profile-N` when decoding.

| File-kind code | Source | File kind |
| ---: | --- | --- |
| 0 | Minecraft | `options` |
| 1 | Lunar | `mods` |
| 2 | Lunar | `general` |
| 3 | Lunar | `controls` |
| 4 | Lunar | `performance` |

Full pointers are bounded UTF-8 strings. A negative integer `-(index + 1)` references the fixed `POINTER_DICTIONARY` in `src-tauri/src/codec.rs`. The table has 611 public setting pointers and is part of the PRS2 specification. Its order and contents must never change, even if the scanner's allowlist changes. Its SHA-256 fingerprint, hashing each pointer followed by a newline, is:

```text
099b6870a9e420ca9d116422548c26524845439e4fccc7eb916a25485bdd38a5
```

Regression tests check the fingerprint, table length, and known indices. Later approved pointers absent from the table can use full strings or common-prefix records. A different dictionary requires a different format version and prefix; regenerating indices from the current scanner schema would break existing codes.

A positive prefix length references that many bytes of the previous expanded pointer in the same group, followed by the suffix. Lengths must be within the previous pointer and end at a UTF-8 character boundary. A group starts without a previous pointer. Dictionary and full-pointer records update the previous pointer too. The current encoder uses a prefix only when at least three bytes match. Decoding accepts prefix lengths from 1 to 512 and expanded pointers no longer than 512 bytes. Values never use dictionary or prefix references.

The public `ShareEnvelope` still exposes full pointers, source names, anonymous profiles, RFC 3339 creation time, and string platform names. Labels and categories are regenerated from approved pointers. Local IDs, display labels, directory names, paths, and original profile names are absent from both wire formats.

## Version 3 HUD metadata

PRS3 uses the same Brotli compression, complete SHA-256 framing, grouped setting records, and frozen pointer dictionary as PRS2. Its array adds one field:

```text
PRS3:base64url(sha256(compressed_payload) || compressed_payload)
[3, timestamp, applicationVersion, minecraftVersion, lunarVersion, platform, groups, hudViewport]
hudViewport = [width, height] | nil
```

Width and height are finite effective HUD coordinates from 16 to 32,768. They describe the maximized standard window client area after Minecraft GUI scale and Lunar/Retina scaling. They contain no monitor name, display identifier, or filesystem path. The native exporter measures them locally rather than trusting caller-provided metadata. If the source viewport cannot be verified, it exports PRS2 with exact coordinate values and no size metadata. A selection with one source viewport can contain verified root HUD coordinates from only one Lunar profile.

The decoder requires exactly eight fields and rejects inconsistent prefix/version pairs. HUD metadata is validated before presentation. Import preserves the wire values and applies viewport ratios only to selected, verified root `x`/`y` coordinates during planning; the adjusted values appear in the diff before application. Codes without source geometry retain an explicit option to import exact offsets. See [the HUD schema notes](lunar-schema.md#hud-layout-adaptation-in-103) for supported components and scale semantics.

## Version 1 compatibility

Legacy codes keep their original Zstandard level 3 pipeline:

```text
PRS1:base64url(sha256(compressed_payload) || compressed_payload)
[1, createdAt, applicationVersion, metadata, settings]
metadata = [minecraftVersion, lunarVersion, platform]
setting = [source, fileKind, profile, pointer, value]
```

The prefix selects the decoder and the payload must contain the matching version. Existing codes can be imported without conversion. Re-exporting uses PRS2 or PRS3 according to the selected HUD metadata. Unknown prefixes and versions are rejected.

IDs are regenerated with SHA-256 over four UTF-8 components: source, file kind, anonymous profile, and full pointer. Each component is preceded by its byte length as an unsigned 32-bit little-endian integer. Import maps approved pointers to an explicitly selected local target profile; shared profiles never become filesystem paths.

The application version is a bounded version string starting with a digit. Game versions contain one to four numeric components (for example `26` or `1.21.4`), or a Minecraft snapshot identifier such as `24w14a`. Local version-directory suffixes are removed during export so custom profile names do not enter metadata.

## Limits and privacy

| Limit | All versions |
| --- | ---: |
| Share-code input | 2 MiB |
| Decompressed payload | 8 MiB |
| Maximum decoding window | 8 MiB |
| Selected settings | 1–10,000 |
| Per-value conservative serialization budget | 64 KiB |
| Per-value nesting depth | 16 |
| MessagePack nesting depth | 32 |

The decoder bounds input before Base64 allocation. Zstandard uses an explicit window limit. Brotli's window header is checked before allocating the decoder; windows above 8 MiB and extended large-window headers are rejected. Brotli output is read in bounded chunks and stops when the 8 MiB output limit would be exceeded. The cumulative setting count is checked while reading groups.

All formats accept only bounded scalar setting values. Nested arrays and objects are rejected before allocating their contents. The decoder rejects malformed Base64, checksum failures, invalid MessagePack, trailing uncompressed bytes, duplicate records or groups, invalid profile and dictionary indices, invalid prefix boundaries, and unapproved settings. PRS2 and PRS3 also reject trailing compressed bytes. A valid checksum does not bypass limits or the allowlist.

Codes contain selected settings, version/platform metadata, and optional HUD viewport dimensions. Account files, authentication fields, tokens, server addresses, paths, logs, and unknown fields are excluded. Approved Resource Pack identifiers can include custom pack filenames; deselect that category if those names should remain private. Codes are not encrypted.

## Discord sharing

The app uses 2,000 characters as the ordinary Discord message budget. Representative selections fit, but their size depends on the settings, values, and number of profiles. There is no promise that every selection fits in one message. When a code is longer than the message budget, attach its `.prism` file to Discord. Splitting or truncating the code changes its contents and can make it unreadable.

## Compression comparison

Run the synthetic benchmark with:

```sh
cd src-tauri
cargo run --release --example codec_benchmark
```

The benchmark compares JSON and MessagePack, Zstandard level 3 and Brotli qualities 5, 9, and 11, grouped records, common-prefix pointers, and dictionary pointers. The shipping pipeline tries all three MessagePack representations at quality 11 and picks the smallest compressed candidate. The complete checksum and prefix are included in every character count.

Fixtures contain only approved public setting names and invented values. `small` includes 15 Minecraft settings and nine Lunar HUD values. `multi-profile` repeats that selection across five profiles. `broad-120` has one profile with 15 Minecraft settings and 105 distinct Lunar leaves. `broad-all` has one profile with 15 Minecraft settings and all 465 approved Lunar leaves. The large profile fixture repeats the 24-setting selection across 25 profiles. These repetitions make compression easier; the broad fixtures show the cost of distinct setting pointers and varied values.

Encode timing includes serialization, candidate compression, checksum, and Base64 framing. Decode timing includes Base64, checksum verification, decompression, mirror-schema deserialization, and equality checks. The shipping row includes all three candidate encodes. The app's whitelist checks, identity regeneration, and additional structural validation are outside these timing measurements; the benchmark also passes each final code through the real decoder and verifies its settings. Each timing averages 50 iterations with a fixed timestamp.

Measured on 2026-09-30 with an Apple M4, macOS 26.5.2, Rust 1.98.1, `rmp-serde` 1.3.1, `zstd` 0.13.3, and `brotli` 8.0.4 in release mode. Timing varies with hardware and system load. The example prints every measured combination; these rows show the main comparison:

| Fixture | Records | Representation | Compression | Raw bytes | Compressed bytes | Code characters | Encode µs | Decode µs |
| --- | ---: | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Small | 24 | Legacy JSON | Zstd 3 | 2,664 | 555 | 788 | 16.3 | 13.4 |
| Small | 24 | Legacy MessagePack | Zstd 3 | 1,152 | 460 | 661 | 13.8 | 9.2 |
| Small | 24 | Legacy MessagePack | Brotli 5 | 1,152 | 393 | 572 | 38.6 | 14.5 |
| Small | 24 | Grouped MessagePack | Brotli 5 | 525 | 322 | 477 | 31.9 | 12.4 |
| Small | 24 | Grouped MessagePack | Brotli 11 | 525 | 307 | 457 | 1,100.1 | 12.4 |
| Small | 24 | PRS2 shipping | Brotli 11 | 287 | 224 | 347 | 3,004.3 | 15.8 |
| Five profiles | 120 | Legacy MessagePack | Zstd 3 | 5,564 | 896 | 1,243 | 19.6 | 25.6 |
| Five profiles | 120 | PRS2 shipping | Brotli 11 | 1,343 | 246 | 376 | 9,563.4 | 32.9 |
| Distinct settings | 120 | Legacy JSON | Zstd 3 | 14,146 | 1,994 | 2,707 | 33.1 | 42.0 |
| Distinct settings | 120 | Legacy MessagePack | Zstd 3 | 6,898 | 1,821 | 2,476 | 25.0 | 28.7 |
| Distinct settings | 120 | Legacy MessagePack | Brotli 5 | 6,898 | 1,589 | 2,167 | 172.5 | 41.8 |
| Distinct settings | 120 | Grouped MessagePack | Brotli 5 | 4,212 | 1,369 | 1,873 | 173.8 | 33.6 |
| Distinct settings | 120 | Grouped MessagePack | Brotli 9 | 4,212 | 1,364 | 1,867 | 607.6 | 34.7 |
| Distinct settings | 120 | Grouped MessagePack | Brotli 11 | 4,212 | 1,239 | 1,700 | 3,190.5 | 31.2 |
| Distinct settings | 120 | Prefix MessagePack | Brotli 11 | 2,514 | 1,121 | 1,543 | 2,836.2 | 37.9 |
| Distinct settings | 120 | PRS2 shipping | Brotli 11 | 1,081 | 582 | 824 | 7,055.4 | 36.1 |
| All Lunar leaves | 480 | Legacy MessagePack | Zstd 3 | 30,592 | 6,520 | 8,741 | 74.3 | 111.6 |
| All Lunar leaves | 480 | Grouped MessagePack | Brotli 11 | 20,346 | 4,308 | 5,792 | 12,644.5 | 100.7 |
| All Lunar leaves | 480 | Prefix MessagePack | Brotli 11 | 10,254 | 3,714 | 5,000 | 6,388.7 | 113.7 |
| All Lunar leaves | 480 | PRS2 shipping | Brotli 11 | 4,499 | 1,630 | 2,221 | 23,766.4 | 109.7 |
| 25 profiles | 600 | Legacy MessagePack | Zstd 3 | 28,008 | 3,293 | 4,439 | 57.1 | 121.0 |
| 25 profiles | 600 | PRS2 shipping | Brotli 11 | 6,625 | 329 | 487 | 43,433.0 | 139.7 |

PRS2 reduces the distinct 120-setting fixture from 2,476 to 824 characters, about 67%, bringing it below the ordinary Discord message budget. The broad 480-setting fixture falls from 8,741 to 2,221 characters, about 75%, and should use a file attachment. Brotli quality 11 costs more encode time than Zstandard or lower Brotli qualities, but the measured fixtures remain in milliseconds. Export runs on a background worker to keep the interface responsive. Lower qualities and full-pointer alternatives remain benchmarked so future decisions can be checked against the same fixtures.
