# Fuji Recipe Manager

[繁體中文](README.zh-TW.md) | English

Fuji Recipe Manager is a local-first desktop application for managing Fujifilm film-simulation recipes on macOS and Windows. It is built with Tauri 2, React, TypeScript, and Rust.

The project is intentionally safety-led: recipes can always be created, compared, imported, and exported locally; camera actions advance only after a model and firmware have passed a staged hardware validation record.

> **Current status: hardware-backed alpha.** Experimental X-M5 firmware 1.30 Recipe writes create a durable local snapshot first, write only the verified fields, read every field back, and verify rollback on failure. Other Fujifilm models remain probe-only.

## Design direction

The workflow takes inspiration from [Latent](https://github.com/formray/latent): preserve a local recipe library, make camera backup and read-back verification prerequisites for writes, record capability by camera model and firmware, and handle macOS PTP-ownership conflicts clearly. This is an independent native desktop implementation; it does not copy Latent source code.

## What works today

- A Tauri 2 desktop shell and responsive React recipe workspace.
- Local SQLite recipe library in the desktop app, with browser-development fallback storage.
- Recipe creation, editing, search, tags, favourites, JSON/FRecipe import and export, and common Recipe text parsing.
- Optional original-author and source-URL attribution stored with every local Recipe. URLs are never scraped or used to copy a Recipe into the app.
- USB discovery through Rust and `nusb` across Fujifilm X Series, X100, GFX, FinePix, and unknown Fujifilm bodies.
- An X-M5 profile with four custom slots and a physical USB record: `04CB:030C`, firmware `1.30`.
- Read-only PTP DeviceInfo probing with serial numbers deliberately excluded from logs.
- An opt-in PTP property-descriptor probe that opens and closes a standard session without selecting a slot or sending a write command.
- A hardware-verified X-M5 preset-name write/read-back/restore CLI test. Firmware 1.30 accepts printable ASCII names; Unicode names are proactively blocked after the camera rejected them with PTP `201C`.
- Reversible X-M5 C2 write/read-back/restore tests for dynamic range and the signed highlight, shadow, color, sharpness, and clarity properties.
- Reversible X-M5 C2 tests for film simulation, Color Chrome, Chrome FX Blue, white balance, WB shifts, high ISO NR, and Grain. X-M5 Grain Off uses command value `01 00` and reads back canonically as `06 00`; this normalization is encoded and unit-tested.
- Experimental GUI writes for X-M5 only: the target custom-setting name (`D18D`) and Recipe fields are captured in a pre-write SQLite backup and write journal, every value is read back, and failed writes run a verified rollback. The previous active slot is restored after each operation.
- A recovery-backup list and explicit, verified X-M5 restore action.

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

```bash
npm install
npm run tauri dev
```

Create a local macOS development bundle:

```bash
npm run tauri -- build
```

The local bundle is written under `target/release/bundle/macos/`. It is not code-signed or notarized. Build Windows installers on a Windows runner after the Windows USB/PTP checklist passes.

## Read-only camera probes

Set the camera to its documented USB/PTP-compatible mode, connect it directly by USB, and close Image Capture, Photos, X RAW Studio, and tethering software before probing.

```bash
# USB discovery only
cargo run -p fuji-test

# Standard PTP GetDeviceInfo; no PTP session and no camera setting access
cargo run -p fuji-test -- ptp-info

# Explicitly inspect one property descriptor (hex). This opens then closes a
# standard PTP session; it does not select a custom slot or write a setting.
cargo run -p fuji-test -- ptp-descriptor D18C

# Read one property's raw value. The bytes remain un-interpreted until their
# encoding is verified for this exact camera model and firmware.
cargo run -p fuji-test -- ptp-read D18C
```

`D18C` is the Fujifilm custom-slot selector identifier recorded for the experimental X-M5 path. A descriptor result marked writable only describes camera capability; it never unlocks application writes by itself. The X-M5 returns a general error for its standard descriptor request, while its direct read-only value request succeeds; this is recorded as a model-specific protocol observation, not as write permission.

## Repository layout

```text
apps/fuji-test/             Read-only USB/PTP validation CLI
crates/camera-core/         Shared camera capability types
crates/ptp-core/            Standard PTP framing and safe descriptor parsing
crates/fuji-ptp/            Fujifilm vendor property identifiers
crates/usb-transport/       nusb backend and platform fallback boundary
crates/camera-xm5/          X-M5 capability declaration
crates/camera-fujifilm/     Fujifilm family/model recognition and access stages
src-tauri/                  Native Tauri shell and local SQLite library
src/                        React recipe workspace
docs/                       Hardware safety and release documentation
```

## Development checks

```bash
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run check
npm run build
```

See [development notes](docs/development.md), [release status](docs/release-status.md), and the [X-M5 Recipe settings reference](docs/settings-reference.md) for the physical-hardware gate and option source.
The model-by-model support policy is in the [camera capability matrix](docs/camera-capability-matrix.md).

## Privacy and trademarks

Recipes remain on the device unless you export them. The application does not upload camera settings or RAF files.

Fuji Recipe Manager is independent and is not affiliated with or endorsed by Fujifilm. Fujifilm and film-simulation names are trademarks of their respective owners.
