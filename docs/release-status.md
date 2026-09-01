# Release status

## Implemented

- Cross-platform Tauri 2 + React desktop shell.
- Native Rust USB device enumeration via `nusb`.
- Fujifilm-wide USB recognition across X Series, X100, GFX, FinePix, and unlisted Fujifilm bodies.
- Local recipe editor, search, tags, favourites, import/export, and persistent SQLite library.
- Portable versioned recipe JSON (`schemaVersion: 1`).
- Read-only camera discovery UI and `fuji-test` CLI.
- Standards-based, opt-in `GetDeviceInfo` PTP probe; it is read-only and does not open a PTP session.
- Opt-in standard `GetDevicePropDesc` and `GetDevicePropValue` probes with explicit session open/close and strict response validation. They are not automatically invoked, cannot select a slot, and send no PTP setting-write operation.
- X-M5 firmware 1.30 custom-slot selector (`D18C`) write/read-back verification for C1.
- X-M5 firmware 1.30 reversible C4 preset-name (`D18D`) write/read-back/restore verification using `FRM-TEST`; printable ASCII works, while Unicode was rejected with PTP `0x201C` and is blocked by the UI and backend.
- X-M5 firmware 1.30 reversible Recipe-field tests with byte-for-byte restoration: dynamic range (`D190`) and highlight/shadow/color/sharpness/clarity (`D19D`–`D1A0`, `D1A2`). The current C4 hardware run also verified the new Highlight `+0.5` and Shadow `-0.5` encodings and restoration.
- X-M5 firmware 1.30 reversible C2 tests for film simulation (`D192`), Color Chrome (`D196`), Chrome FX Blue (`D197`), white-balance mode and shifts (`D199`, `D19A`, `D19B`), high-ISO NR (`D1A1`), and grain (`D195`).
- Physical X-M5 GUI write transaction verified end-to-end: a 15-field C4 Recipe write created a durable pre-write SQLite backup and journal, read every written field back, then restored the backup through the in-app confirmation dialog with read-back verification.
- Recovery-backup list and explicit X-M5 restore action with an in-app confirmation dialog and read-back verification.
- macOS `.app` bundle build verified locally.

## Still required before a production camera importer release

The following work deliberately remains gated on physical hardware for each model and firmware combination:

1. Confirm the USB product ID and PTP-compatible camera mode for each model/firmware.
2. Open a PTP session and collect read-only property descriptors.
3. Read complete C1-C4 property values, store sanitized fixtures, and validate full-slot backup/restore.
4. Map every human-readable setting to its X-M5 property and value encoding.
5. Map every Recipe parameter to a model- and firmware-verified property encoding, then exercise one disposable-slot Recipe-field write followed by byte-for-byte read-back verification.
6. Repeat the completed X-M5 GUI-write/recovery checklist and installer validation on Windows.

Until all six gates pass for a model, its write button must remain disabled. X-M5 is an experimental recovery-capable path, not a production-supported importer.

## Latest physical result

An X-M5 with firmware `1.30` was detected as `04CB:030C` with USB Still Image/PTP interface `06/01/01`. The opt-in standard DeviceInfo probe completed successfully with PTP response `0x2001`, returning 365 bytes through bulk OUT `0x01` and bulk IN `0x81`. A standard `GetDevicePropDesc(D18C)` session was rejected with `0x2002`; the application consumed its response and closed the session cleanly. A separate `GetDevicePropValue(D18C)` session completed successfully and returned the raw two-byte value `01 00`. The selector was then set to C1 and read back successfully. C2 reversible tests verified dynamic range, film simulation, Color Chrome, Chrome FX Blue, white-balance mode and shifts, high-ISO NR, highlight, shadow, color, sharpness, clarity, and grain. The current C4 run verified raw `+5` / `-5` PTP values for the user-facing `+0.5` Highlight / `-0.5` Shadow steps, with both original values restored. C4 printable-ASCII preset name `FRM-TEST` also wrote, read back, and restored successfully; a Unicode test name was rejected with `0x201C`, so firmware 1.30 name writes are now restricted to printable ASCII. X-M5 Grain Off is sent with command value `01 00` and reads back canonically as `06 00`; the model codec accepts that normalization and uses the command form when restoring a captured Off value. The release App then completed a 15-field GUI C4 write transaction (including a `+0.5` Highlight), generated a durable SQLite restore point, and restored that same restore point through the new in-app confirmation dialog. The App reported full read-back verification. Independent CLI probes confirmed C4 was restored to the empty preset name (`01 00 00`) and baseline Highlight (`00 00`); the active custom slot was finally reset to C1 (`01 00`). Windows validation is still outstanding.
