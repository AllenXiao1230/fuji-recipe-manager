# Release status

Updated: 2026-09-06

## Current support boundary

Only `04CB:030C` / `FUJIFILM X-M5` / firmware `1.30` has an experimental
Recipe-write path. The bundled capability record is schema version 2 and
separates four states: `write_verified`, `read_detected_unverified`,
`write_rejected`, and `blocked_unknown`. A local capability matrix or a
successful read-only PTP query cannot change this boundary.

The X-M5 1.30 writer supports only the explicitly verified custom-slot fields
and value encodings listed in the [camera capability matrix](camera-capability-matrix.md).
Long Exposure NR (`D1A3`) and monochrome warm/cool or magenta/green
(`D193`/`D194`) remain locked after `201C` rejections. Reserved preset fields
`D191`/`D1A5`, eight vendor global properties, two standard PTP global
properties (`5005`/`5015`), and the fixed unknown-vendor list are read-only
research evidence, never Recipe writes.

The CLI and Capability Matrix page can run a fixed-scope audit of `D191`,
`D1A5`, and unknown vendor properties. It uses only DeviceInfo,
GetDevicePropValue, and GetDevicePropDesc; it neither selects a C slot nor
sends SetDevicePropValue. See the [research procedure](xm5-unverified-property-research.md).

## Latest X-M5 firmware 1.30 hardware verification

The read-only property scan reported 61 DeviceInfo-advertised properties and
58 readable values. Reversible C4 tests verified the existing Recipe write
set (61/61 cases), plus image size (captured-value write), image quality
(captured-value write), Smooth Skin Effect (Off/Weak/Strong), Color Space
(sRGB/Adobe RGB), and Color Temperature (2500 K/6500 K/10000 K). Every
successful C4 case read back and restored its captured value, and the active
slot was restored to C1.

Long Exposure NR (`D1A3`) and Monochromatic Color Warm/Cool and
Green/Magenta (`D193`/`D194`) were rejected with PTP `201C` for every tested
candidate encoding, so they remain write-locked. Eight vendor global
properties plus the two standard PTP properties `5005` and `5015` accepted a
write of their current value and read back unchanged. This confirms only their
transport path; alternate-value encodings and global-setting safety are not
unlocked.

The 2026-09-02 retest repeated those outcomes on C4: Long Exposure NR Off and
On were both rejected and a follow-up read confirmed the captured value did
not change. With ACROS selected, both monochrome properties rejected each
direct and ×10 candidate at both test ends; captured values were restored.
The current-value-only global test passed 10/10 properties, but is evidence of
transport acceptance only and does not unlock alternate values.

The same physical X-M5 test session manually set C4 Image Size to `L 16:9`,
captured `D18E=08 00`, and then passed a dedicated write → read-back →
captured-value restore test with active-slot recovery. `L 16:9` is therefore
the second and only additional Image Size payload unlocked for firmware 1.30.

A subsequent C4 image-payload session set and captured `L 1:1` (`D18E=09 00`)
and `NORMAL` (`D18F=03 00`). Both values passed independent write → read-back
→ captured-value restore checks with active-slot recovery, and are now enabled
alongside the previously verified payloads.

The following C4 session captured `FINE+RAW` as `D18F=04 00` and passed the
same independent write → read-back → captured-value restore and active-slot
recovery check. It is now enabled; `NORMAL+RAW` and `RAW` remain unverified.

`NORMAL+RAW` was then captured as `D18F=05 00` on C4 and passed the same
reversible hardware test, including active-slot recovery. The final standalone
`RAW` quality was captured as `D18F=01 00` and also passed the same test. All
displayed Image Quality choices are now enabled for the X-M5 1.30 record.

`M 3:2` was then captured as `D18E=04 00` on C4 and passed the same reversible
write → read-back → captured-value restore and active-slot recovery check. It
is now enabled alongside the three previously verified L-size payloads.

`M 16:9` was then captured as `D18E=05 00` on C4 and passed the same reversible
hardware test. The remaining crop-dependent choices require a separate
drive-mode capability before they can be verified or enabled.

`M 1:1` was then captured as `D18E=06 00` on C4 and passed the same reversible
hardware test. It is now enabled alongside the other verified M-size payloads.

`S 3:2` was then captured as `D18E=01 00` on C4 and passed the same reversible
hardware test. It is now enabled alongside the verified M- and L-size payloads.

`S 16:9` was then captured as `D18E=02 00` on C4 and passed the same reversible
hardware test. It is now enabled alongside the other verified standard sizes.

`S 1:1` was then captured as `D18E=03 00` on C4 and passed the same reversible
hardware test. All nine standard S/M/L Image Size choices are now enabled.

The app now uses Fujifilm’s official names for 1.25× crop sizes: `P 3:2`,
`P 16:9`, and `P 1:1`. Older local values named `M … (1.25× crop)` are migrated
on load. They remain mode-dependent and write-blocked until the corresponding
Sports Finder or high-speed-burst prerequisite is represented and verified.

A follow-up C4 reversible enum run passed 15/15 candidate values: Dynamic
Range Auto, the twelve remaining non-Auto Film Simulations, and White Balance
White Priority / Ambience Priority. Each test wrote its target value, read it
back, restored the captured C4 value, and restored the active C slot. These
values are now enabled in the X-M5 codec and GUI preflight.

## Implemented

- Cross-platform Tauri 2 + React desktop shell.
- Native Rust USB device enumeration via `nusb`.
- Fujifilm-wide USB recognition across X Series, X100, GFX, FinePix, and unlisted Fujifilm bodies.
- Local recipe editor, search, tags, favourites, import/export, and persistent SQLite library.
- SQLite schema migration (`user_version`), WAL/busy-timeout configuration, optimistic per-Recipe upserts, and deletion tombstones. Desktop localStorage is now a migration/fallback source rather than a second authority.
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
- Startup-visible interrupted-write recovery journal. Pending journals cannot be dismissed by a UI action; they become `recovered_manually` only after the associated backup restore completes with read-back verification.
- Camera mutations in the import, installed-preset, and Camera pages require a fresh DeviceInfo identity result with the stable `exact_experimental` capability state, rather than relying on USB product ID alone.
- Per-Recipe raw C-slot snapshot capture for the exact X-M5 record. It preserves readable `D18D…D1A5` values and records unreadable codes, while keeping every raw byte outside both the writer and recovery allow-list.
- Controlled CLI candidate capture for unverified Image Size / Image Quality values. It requires a value to be selected manually on a disposable C slot, reads only the reported raw value, restores the active C slot, and keeps the capability matrix locked pending a separate reversible verification.
- Fixed desktop sidebar with an independent workspace scroll area, local Recipe-to-C-slot labels, and installed-preset synchronization by exact preset name.
- Per-slot X-M5 clear action: creates the same durable backup and journal as an import, clears the name, writes only neutral values for the 14 verified Recipe properties, verifies every value, and never claims to factory-reset unverified camera settings.
- macOS `.app` bundle build verified locally. After removing build-output-only Finder/file-provider extended attributes, the App passed strict ad-hoc `codesign` verification; the repackaged `Fuji Recipe Manager_0.1.0_aarch64-adhoc.dmg` passed checksum verification and its mounted App passed the same check. It is deliberately not Developer ID-signed or notarized, and Gatekeeper rejects it as expected.
- Versioned capability-record resolution requires an exact USB ID, model, and firmware match. All other Fujifilm cameras and firmware versions now have a dedicated `ptp-scan-readonly` path and remain probe-only.
- Bundled X-M5 capability record schema version 2 explicitly tracks the internal slot selector `D18C`, reserved snapshot-only fields `D191`/`D1A5`, eight vendor-global fields, and two standard global fields. It corrects the previous mixed eight-versus-ten global-property narrative without widening write access.
- Fixed-scope CLI and GUI read-only audit for `D191`, `D1A5`, and the unknown-vendor property list. Each result preserves raw value and descriptor evidence in local notes; it cannot write a property, select a C slot, or unlock a Recipe field.
- `.FP1/.FP2/.FP3` local import/export with safe unmapped-field retention and serial-number removal. X RAW Studio round-trip compatibility is not yet a release claim.
- JPEG/RAF metadata-to-Recipe import with per-field recognised/unavailable results. It is read-only; the present development implementation requires locally installed ExifTool.
- RAF preview offline preflight and cleanup-oriented Rust transaction abstraction. No vendor camera transport adapter, RAF upload, conversion, or preview JPEG is implemented yet.
- Frontend Vitest coverage for Recipe normalization/text import, safe external URL handling, and protected-dialog Escape behavior; CI runs frontend tests, production build, and dependency audit. Tag builds create explicitly unsigned macOS/Windows bundle artifacts with per-platform SHA-256 manifests for signing pipelines.

## Still required before a production camera importer release

The following work deliberately remains gated on physical hardware for each model and firmware combination:

1. Confirm the USB product ID and PTP-compatible camera mode for each model/firmware.
2. Open a PTP session and collect read-only property descriptors.
3. Read complete C1-C4 property values, store sanitized fixtures, and validate full-slot backup/restore.
4. Map every human-readable setting to its X-M5 property and value encoding.
5. Map every Recipe parameter to a model- and firmware-verified property encoding, then exercise one disposable-slot Recipe-field write followed by byte-for-byte read-back verification.
6. Repeat the completed X-M5 GUI-write/recovery checklist and installer validation on Windows.
7. Bundle, license-review, and test the JPEG/RAF metadata adapter on macOS and Windows; do not release a build that depends on an arbitrary user-installed `exiftool`.
8. Validate generated `.FP1/.FP2/.FP3` profiles by importing them into the target X RAW Studio version and record every preserved/unmapped field.
9. Derive and hardware-validate a model-specific RAF upload/conversion/download/abort adapter before enabling camera-side RAF preview.

Until all applicable gates pass for a model, its write button must remain disabled. X-M5 is an experimental recovery-capable path, not a production-supported importer.

## Latest physical result

An X-M5 with firmware `1.30` was detected as `04CB:030C` with USB Still Image/PTP interface `06/01/01`. The opt-in standard DeviceInfo probe completed successfully with PTP response `0x2001`, returning 365 bytes through bulk OUT `0x01` and bulk IN `0x81`. A standard `GetDevicePropDesc(D18C)` session was rejected with `0x2002`; the application consumed its response and closed the session cleanly. A separate `GetDevicePropValue(D18C)` session completed successfully and returned the raw two-byte value `01 00`. The selector was then set to C1 and read back successfully. C2 reversible tests verified dynamic range, film simulation, Color Chrome, Chrome FX Blue, white-balance mode and shifts, high-ISO NR, highlight, shadow, color, sharpness, clarity, and grain. The current C4 run verified raw `+5` / `-5` PTP values for the user-facing `+0.5` Highlight / `-0.5` Shadow steps, with both original values restored. C4 printable-ASCII preset name `FRM-TEST` also wrote, read back, and restored successfully; a Unicode test name was rejected with `0x201C`, so firmware 1.30 name writes are now restricted to printable ASCII. X-M5 Grain Off is sent with command value `01 00` and reads back canonically as `06 00`; the model codec accepts that normalization and uses the command form when restoring a captured Off value. The release App then completed a 15-field GUI C4 write transaction (including a `+0.5` Highlight), generated a durable SQLite restore point, and restored that same restore point through the new in-app confirmation dialog. The App reported full read-back verification. Independent CLI probes confirmed C4 was restored to the empty preset name (`01 00 00`) and baseline Highlight (`00 00`); the active custom slot was finally reset to C1 (`01 00`). Windows validation is still outstanding.
