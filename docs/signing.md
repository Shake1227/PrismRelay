# Release signing

Current releases do not use a developer signing certificate or notarization. macOS app bundles may receive the ordinary local ad-hoc signature required by the toolchain; this is not developer identity verification.

For a future signed release, configure Tauri's supported signing environment in the protected release job:

- macOS: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`.
- Windows: a protected certificate and password, plus Tauri's Windows `signCommand` or a post-build Authenticode signing step.

Store credentials as GitHub Actions secrets. Keep certificates and keys out of the repository. Sign before producing ZIP archives and SHA-256 checksums. Never make a signed-release claim unless certificate verification and notarization succeed.

References: [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/) and [Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/).
