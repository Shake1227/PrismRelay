# Release signing

Current releases do not use a developer signing certificate or notarization. macOS bundles use an ad-hoc signature to seal the app, icon, and license resources. This checks bundle integrity and does not verify developer identity. Gatekeeper rejects the downloaded app until the user grants permission through Privacy & Security. The initial-launch steps are in the README and release notes.

The current installer smoke check mounts the DMG and verifies architecture, resources, camera entitlement, and signature integrity. Passing that check does not mean Apple has approved the app. A future notarized release must also pass `xcrun stapler validate` and `spctl --assess --type execute` for the mounted app.

For a future signed release, configure Tauri's supported signing environment in the protected release job:

- macOS: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`.
- Windows: a protected certificate and password, plus Tauri's Windows `signCommand` or a post-build Authenticode signing step.

Store credentials as GitHub Actions secrets. Keep certificates and keys out of the repository. Sign the app before packaging MSI, setup EXE, and DMG installers, then calculate SHA-256 checksums. A signed release requires successful certificate verification and macOS notarization.

References: [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/) and [Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/).
