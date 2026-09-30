# Prism Relay 0.1.0

Initial public release of a local settings sharing utility for Lunar Client and Minecraft.

- Japanese interface with an original glass design, light/dark/system themes and reduced motion support.
- Automatic configuration discovery, per-profile selection, searchable setting trees and custom presets.
- Compressed PRS1 share codes with SHA-256 corruption checks, clipboard/file sharing and QR export for short codes.
- Import selection, destination profiles, preview and before/after differences.
- Automatic backups, atomic file replacement, restore and interrupted transaction recovery.
- Read-only handling for unrecognized settings. Existing unknown fields remain intact.
- Manual update checks only; no analytics or telemetry.
- Portable Windows x64 and ARM64 archives, plus macOS Apple Silicon and Intel app archives.

Download the ZIP for your platform and extract it. Choose the x64 or ARM64 archive matching your Windows PC, then run `PrismRelay.exe`; Microsoft WebView2 is required and is included with Windows 11. On macOS, open `Prism Relay.app` directly from the extracted folder. No installer is used.

Binaries are unsigned and macOS builds are not notarized. Your operating system may request approval to open the app. No signing certificate has been fabricated. See README for launch instructions and limitations.

This release supports the verified Lunar profile schema described in `docs/lunar-schema.md`. Waypoints, macros, account information, server addresses and unverified fields are not shared. HUD coordinates are copied in the format stored by Lunar; automatic resolution scaling is not enabled. `optionsof.txt` is discovered but is read-only in this release. A share code contains settings rather than resource-pack contents. Review custom resource-pack names before sharing.

SHA256SUMS.txt covers all platform archives. GPL-3.0-or-later. Unofficial and independent of Lunar Client, Minecraft, Microsoft and Mojang.
