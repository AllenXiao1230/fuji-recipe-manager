# Fuji Recipe Manager

[繁體中文](README.zh-TW.md) | English

Fuji Recipe Manager is a local-first desktop application for managing Fujifilm film-simulation recipes on macOS and Windows. It is built with Tauri 2, React, TypeScript, and Rust.

The project is intentionally safety-led: recipes can always be created, compared, imported, and exported locally; camera actions advance only after a model and firmware have passed a staged hardware validation record.

> **Current status: hardware-backed alpha.** Experimental X-M5 firmware 1.30 Recipe writes create a durable local snapshot first, write only the verified fields, read every field back, and verify rollback on failure. Other Fujifilm models remain probe-only.

## Design direction

The workflow takes inspiration from [Latent](https://github.com/formray/latent): preserve a local recipe library, make camera backup and read-back verification prerequisites for writes, record capability by camera model and firmware, and handle macOS PTP-ownership conflicts clearly. This is an independent native desktop implementation; it does not copy Latent source code.

## What works today

- A Tauri 2 desktop shell and responsive React recipe workspace.
- Local SQLite is the desktop Recipe-library authority. Browser storage is used only for browser development and one-time desktop migration; migrations, WAL, optimistic revisions, and deletion tombstones protect against delayed saves.
- A local SQLite capability-matrix database keyed by Fujifilm USB ID, model, and firmware. It records configurable property scope, observed PTP code, source type, safe attribution URL, evidence notes, and verification time without granting camera-write permission.
- A Camera Capability Catalog that separates recognised Fujifilm models from trusted model/firmware records and local research matrices; a recognised body remains probe-only until verification is complete.
- Recipe creation, editing, search, tags, favourites, JSON/FRecipe import and export, and common Recipe text parsing.
- Composable Recipe Library filters for film simulation, tag (including untagged), favourites, C1–C4 assignment, and camera compatibility. Search also covers each Recipe's name, description, tags, source author, and compatible cameras.
- Independent desktop scrolling for the Recipe Library column (search, filters, and cards) and the Recipe editor, so a long editor never displaces the library controls.
- A custom minimal camera-and-recipe-card app icon, generated as macOS `.icns`, Windows `.ico`, and platform PNG assets and explicitly included in the Tauri bundle.
- Optional original-author and source-URL attribution stored with every local Recipe. URLs are never scraped or used to copy a Recipe into the app.
- USB discovery through Rust and `nusb` across Fujifilm X Series, X100, GFX, FinePix, and unknown Fujifilm bodies.
- An X-M5 profile with four custom slots and a physical USB record: `04CB:030C`, firmware `1.30`.
- Read-only PTP DeviceInfo probing with serial numbers deliberately excluded from logs.
- An opt-in PTP property-descriptor probe that opens and closes a standard session without selecting a slot or sending a write command.
- A hardware-verified X-M5 preset-name write/read-back/restore CLI test. Firmware 1.30 accepts printable ASCII names; Unicode names are proactively blocked after the camera rejected them with PTP `201C`.
- Reversible X-M5 C4 write/read-back/restore tests across every currently writable Recipe field. Enumerated controls are tested across their supported values; numeric controls at minimum, neutral, and maximum.
- X-M5 Grain Off writes the retained Small/Large size first, then command value `01 00`; it reads back canonically as `06 00` (Small) or `07 00` (Large). This recovery sequence is encoded and unit-tested.
- X-M5 firmware 1.30 rejects Highlight and Shadow below `-2.0` with PTP `201C`; both controls therefore allow `-2.0` through `+4.0` in `0.5` steps.
- Experimental GUI writes for X-M5 only: the UI must first verify the connected DeviceInfo model and firmware, then captures the target custom-setting name (`D18D`) and Recipe fields in a pre-write SQLite backup and write journal. Every value is read back, failed writes run a verified rollback, and the previous active slot is restored after each operation.
- Optional per-Recipe raw C-slot preservation: readable X-M5 custom-slot bytes can be captured for audit/interchange, including rejected fields. They are strictly read-only metadata and can never be replayed by the camera writer or recovery system.
- Startup detection for interrupted-write journals, a recovery-backup list, and explicit, verified X-M5 restore actions. A journal is acknowledged only after a manual restore has read back successfully.
- A versioned, bundled capability record at `data/capabilities/fujifilm-xm5-1.30.json`; the import dialog lists every field as write verified, detected-but-unverified, camera-rejected (`201C`), or unknown/blocked.
- A fixed-scope `fuji-test ptp-audit-xm5-unverified` read-only audit for reserved `D191`/`D1A5` and unknown vendor properties; it records raw values and descriptor metadata without selecting a slot or writing the camera.
- Additional X-M5 1.30 write/read-back/restore support for Smooth Skin Effect, sRGB/Adobe RGB, Color Temperature (when White Balance is Color Temperature), and the specifically verified `S 3:2`, `S 16:9`, `S 1:1`, `M 3:2`, `M 16:9`, `M 1:1`, `L 3:2`, `L 16:9`, `L 1:1`, `RAW`, `FINE`, `NORMAL`, `FINE+RAW`, and `NORMAL+RAW` image payloads.
- All 20 non-Auto Film Simulations, Dynamic Range Auto, and White Balance White Priority / Ambience Priority also passed independent X-M5 C4 write/read-back/restore verification. Film Simulation Auto and White Balance Custom 1–3 remain locked.
- Long Exposure NR and monochrome warm/cool or magenta/green remain locked after physical `201C` rejections; detected global properties also remain read-only.
- X RAW Studio `.FP1`, `.FP2`, and `.FP3` profile import/export. Unmapped XML is retained with the local Recipe where safe; serial-number data is removed and the app reports unmapped fields.
- Desktop JPEG/RAF metadata import creates a local Recipe and marks every field as recognised or unavailable. Development uses ExifTool for Fujifilm MakerNotes; a distributable release must bundle and validate this adapter on each platform.
- RAF preview has a desktop offline preflight and a Rust transaction boundary for upload → temporary Recipe → conversion → JPEG download → cleanup. It deliberately has no camera transport adapter yet, so the preflight never opens a camera or claims to render a JPEG.

## Safety boundary

Only the X-M5 USB ID `04CB:030C` currently exposes an **experimental** Recipe-write action. A recognised model name or USB vendor ID does not prove that its Recipe-property encoding matches another Fujifilm generation. No other body is writable in the app.

Before a camera write can be enabled, the model/firmware path must pass all of the following:

1. Detect the USB/PTP interface and record DeviceInfo.
2. Read property descriptors and build a supported-field capability record.
3. Capture the target slot's verified Recipe fields to a local, restorable backup before any field is sent.
4. Translate a recipe only to fields confirmed for that model and firmware.
5. Write to a disposable slot only after explicit user confirmation.
6. Read the slot back, compare every field, retain a write journal, and provide verified restore/interruption recovery.

## Run the desktop app

Requires Node.js 22.12 or newer, Rust stable, and the Tauri platform prerequisites.

```bash
npm install
npm run tauri dev
```

Create the local macOS app bundle only (recommended for local hand-off testing):

```bash
npm run tauri -- build --bundles app
```

The resulting app is `target/release/bundle/macos/Fuji Recipe Manager.app`. It is not code-signed or notarized. Build Windows installers on a Windows runner after the Windows USB/PTP checklist passes.

An unsigned local build may require an explicit Gatekeeper “Open” action on macOS. A signed and notarized DMG is still required before normal distribution.

### Build a Windows `.exe`

On Windows, run [`build-windows-exe.bat`](build-windows-exe.bat). It requires Node.js 22.12+, npm, and Rust; runs locked dependency installation, TypeScript checks, frontend tests, then builds `target\\release\\Fuji Recipe Manager.exe` with Tauri. Install the Rust MSVC toolchain, Microsoft C++ Build Tools, and WebView2 Runtime first. The generated executable is not code-signed.

## Recipe Library navigation

Filters can be combined. For example, select **Classic Negative**, **Favourite**, and **Unassigned** to find starred Classic Negative Recipes that are not currently assigned to C1–C4. Use **Clear filters** to return to the complete local library.

On desktop, the left Recipe Library column and the right Recipe editor have separate scroll areas. The search and filter controls remain with the library while you browse or edit long Recipe details.

## Read-only camera probes

Set the camera to its documented USB/PTP-compatible mode, connect it directly by USB, and close Image Capture, Photos, X RAW Studio, and tethering software before probing.

```bash
# USB discovery only
cargo run -p fuji-test

# Standard PTP GetDeviceInfo; no PTP session and no camera setting access
cargo run -p fuji-test -- ptp-info

# Read-only scan for any Fujifilm model. It records identity and standard
# readable property values only; it never selects a C slot or writes a setting.
cargo run -p fuji-test -- ptp-scan-readonly

# Explicitly inspect one property descriptor (hex). This opens then closes a
# standard PTP session; it does not select a custom slot or write a setting.
cargo run -p fuji-test -- ptp-descriptor D18C

# Read one property's raw value. The bytes remain un-interpreted until their
# encoding is verified for this exact camera model and firmware.
cargo run -p fuji-test -- ptp-read D18C

# Controlled candidate capture for an Image Size / Quality value you selected
# manually on a disposable C slot. It reads one raw value and restores the
# previously active slot; it does not write the candidate value.
cargo run -p fuji-test -- ptp-capture-xm5-image-payload C4 image-size L_16_9

# Reversible verification after recording the raw candidate. The raw argument
# is hexadecimal; only a PASS with active-slot recovery may unlock the value.
cargo run -p fuji-test -- ptp-verify-xm5-image-payload C4 image-size L_16_9 0x0008

# Fixed-scope evidence collection for reserved and unknown X-M5 properties.
# It only reads values and standard descriptors; it never selects a C slot or
# sends SetDevicePropValue.
cargo run -p fuji-test -- ptp-audit-xm5-unverified

# Physical write validation on a disposable slot. This captures every value,
# writes/read-backs each test case, restores it, and restores the active slot.
cargo run -p fuji-test -- ptp-verify-xm5-recipe C4
```

`D18C` is the Fujifilm custom-slot selector identifier recorded for the experimental X-M5 path. A descriptor result marked writable only describes camera capability; it never unlocks application writes by itself. The X-M5 returns a general error for its standard descriptor request, while its direct read-only value request succeeds; this is recorded as a model-specific protocol observation, not as write permission.

### Review unverified properties safely

In the desktop app, open **Camera Capability Matrix**, choose the discovered
X-M5, then select **Run read-only unknown-property audit**. The result is
added to a local research matrix as evidence notes; it cannot promote a field
to write support. The audit is gated to the exact X-M5 1.30 record and covers
only `D191`, `D1A5`, and the fixed unknown-vendor list. Before collecting or
sharing a trace, follow the [X-M5 unverified-property research procedure](docs/xm5-unverified-property-research.md).

## Repository layout

```text
apps/fuji-test/             Read-only USB/PTP validation CLI
crates/camera-core/         Shared camera capability types
crates/ptp-core/            Standard PTP framing and safe descriptor parsing
crates/fuji-ptp/            Fujifilm vendor property identifiers
crates/usb-transport/       nusb backend and platform fallback boundary
crates/camera-xm5/          X-M5 capability declaration
data/capabilities/          Versioned USB ID + model + firmware capability records
crates/camera-fujifilm/     Fujifilm family/model recognition and access stages
crates/raf-preview/         Recoverable RAF preview transaction boundary (no camera adapter yet)
assets/branding/            Source artwork for the application icon
build-windows-exe.bat       Windows EXE build helper
src-tauri/                  Native Tauri shell and local SQLite library
src-tauri/icons/            Generated macOS, Windows, and platform icon assets
src/                        React recipe workspace
src/CapabilityMatrixPanel.tsx  Local camera capability-matrix database UI
docs/                       Hardware safety and release documentation
```

## Development checks

```bash
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run check
npm run test
npm run build
```

See [development notes](docs/development.md), [release status](docs/release-status.md), the [X-M5 Recipe settings reference](docs/settings-reference.md), and the [unverified-property research procedure](docs/xm5-unverified-property-research.md) for the physical-hardware gate and option source. The model-by-model support policy is in the [camera capability matrix](docs/camera-capability-matrix.md).

## Privacy and trademarks

Recipes remain on the device unless you export them. The application does not upload camera settings or RAF files.

Fuji Recipe Manager is independent and is not affiliated with or endorsed by Fujifilm. Fujifilm and film-simulation names are trademarks of their respective owners.
