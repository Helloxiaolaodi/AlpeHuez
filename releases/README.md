# Release Archive

Versioned AlpeHuez Windows artifacts are stored under `releases/v<version>/`.
Each folder keeps the portable release exe, installers, and a `SHA256SUMS.txt`.

Current archive:

| Version | Folder | Artifacts |
| --- | --- | --- |
| 0.3.0 | `v0.3.0/` | portable exe, NSIS setup, Android APK (arm64) |
| 0.2.0 | `v0.2.0/` | portable exe, NSIS setup |
| 0.1.0 | `v0.1.0/` | portable exe, NSIS setup, MSI |

To build and archive a future version, run from the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\release.ps1 -Version 0.3.0
```

The script updates `tauri.conf.json`, `Cargo.toml`, and `Cargo.lock`, builds the
NSIS installer, archives the artifacts, and refreshes `releases/latest.json`.
