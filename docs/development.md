# Development notes

## Hardware safety boundary

Phase 1 contains one experimental write path: X-M5 firmware 1.30 through USB ID `04CB:030C`. Before it sends a property, the app saves a durable SQLite snapshot and write journal; it reads each property back and verifies rollback if any write fails. No other model may call `SetDevicePropValue` until it has its own capability record and hardware checklist.

The UI must also complete a fresh read-only DeviceInfo identity probe before it
exposes a custom-slot mutation. A matching USB product ID alone is never a UI
or backend write grant. On launch, `writing` and `recovery_failed` journals are
listed for review; a journal is cleared only when its associated backup has
been manually restored with read-back verification.

The checked-in record is [`../data/capabilities/fujifilm-xm5-1.30.json`](../data/capabilities/fujifilm-xm5-1.30.json). It is loaded by `camera-xm5` and exposed to the desktop UI; it must be updated alongside the encoder, recovery allow-list, and unit tests. A direct `GetDevicePropValue` observation never changes a field from `read_detected_unverified` to `write_verified`.

## macOS PTP interface ownership

macOS may allow the system photo-import service, Image Capture, Photos, X RAW Studio, or another tethering app to claim a camera's PTP interface before this app can open it. A `could not be opened for exclusive access` result is an environmental contention error, not permission to detach or force-control the camera. Close those applications, disconnect the camera, reconnect it directly, and retry the read-only probe. The application must never forcibly detach a macOS owner because this can interrupt an active import or tethering session.

## X-M5 validation checklist

1. Set the camera's USB mode to its documented PTP/tethering-compatible mode and connect it directly by USB-C.
2. Run `cargo run -p fuji-test` on macOS and Windows.
3. Record the displayed vendor/product ID and confirm the product ID against at least two physical-device observations or an authoritative source. The first observed X-M5 is `04CB:030C`; retain the raw discovery output in local test notes.
4. Add that ID to `camera-xm5::X_M5_USB_IDS`. The first physical X-M5 has passed the standard DeviceInfo probe as `FUJIFILM X-M5`, firmware `1.30`, PTP response `0x2001`; retain `Experimental` until the complete multi-field app workflow and Windows recovery are validated.
5. Implement read-only PTP property descriptors, then save anonymized logs as test fixtures. `fuji-test ptp-descriptor D18C` now opens and closes a standard PTP session to inspect one descriptor only; it must remain opt-in and must never be used to infer that a write is safe.
6. Preserve a user-visible backup list, write journal, and explicit restore action. A rollback is not successful unless each restored property is read back and matches its captured representation.

### Current X-M5 1.30 extension test record

- `D198` Smooth Skin Effect: Off, Weak, Strong write/read-back/restore passed.
- `D1A4` Color Space: sRGB and Adobe RGB write/read-back/restore passed.
- `D199=8007` followed by `D19C`: 2500 K, 6500 K, and 10000 K passed; the
  encoder accepts the bounded UI range in 10 K steps only while the mode is
  Color Temperature.
- `D18E=0100` Image Size (`S 3:2`), `D18E=0200` Image Size (`S 16:9`),
  `D18E=0300` Image Size (`S 1:1`), `D18E=0400` Image Size (`M 3:2`),
  `D18E=0500` Image Size (`M 16:9`),
  `D18E=0600` Image Size (`M 1:1`),
  `D18E=0700` Image Size (`L 3:2`),
  `D18E=0800` Image Size (`L 16:9`), `D18E=0900` Image Size (`L 1:1`),
  `D18F=0100` Image Quality (`RAW`),
  `D18F=0200` Image Quality (`FINE`), `D18F=0300` Image Quality (`NORMAL`),
  `D18F=0400` Image Quality (`FINE+RAW`), and `D18F=0500` Image Quality
  (`NORMAL+RAW`) each passed
  their own captured-payload write/read-back/restore record. No other
  displayed option may be sent.
- `D1A3` Long Exposure NR and `D193`/`D194` monochrome colour properties
  rejected test values with PTP `201C`; they stay locked.

### Raw custom-slot preservation and image-payload calibration

The import dialog can capture a bounded X-M5 custom-slot raw-property snapshot
(`D18D…D1A5`) into the local Recipe. It temporarily selects the chosen slot,
reads every property it can, then verifies restoration of the previously active
slot. Failed reads are recorded by code. This artifact is for audit and
interchange only: its raw bytes are not part of the encoder, the automatic
rollback allow-list, or any future restore operation.

To obtain a candidate mapping for an unverified Image Size or Image Quality
option, set that exact value manually on a disposable C slot in the camera
menu, then run one of the following commands. The command reads the candidate
and restores the original active slot; it does not send an image-setting write.

```bash
cargo run -p fuji-test -- ptp-capture-xm5-image-payload C4 image-size L_16_9
cargo run -p fuji-test -- ptp-capture-xm5-image-payload C4 image-quality NORMAL
```

Treat the result as `read_only_unverified`. A value becomes write-enabled only
after a separately reviewed reversible write → read-back → restore → read-back
record updates the capability JSON, codec, recovery allow-list, and tests.

## Transport design

`UsbBackend` deliberately returns neutral `UsbDevice` records. Camera recognition happens in `camera-core`; PTP operations belong above the transport layer. This lets a future Windows-specific driver backend or a macOS entitlement-aware backend replace only transport code.

`camera-fujifilm::resolve_capability_record` additionally requires the USB ID,
DeviceInfo model, and firmware to match the checked-in record exactly. A
different firmware, even on the same USB ID, is probe-only. Use
`cargo run -p fuji-test -- ptp-scan-readonly` first for every new body; it
collects only standard DeviceInfo/property reads and cannot select a custom
slot or write a camera property.

## Interchange and metadata boundary

The frontend `.FP1/.FP2/.FP3` adapter preserves the imported profile's
unmapped XML fields where safe, removes serial-number fields, and reports all
fields it could not map to the portable Recipe schema. Export is therefore an
interchange attempt, not proof that X RAW Studio accepts every generated
profile; validate each profile generation against a real X RAW Studio release
before promoting it to a release claim.

JPEG/RAF Recipe creation invokes `exiftool -j -n -G1` through a fixed argument
list. It reads metadata only and never modifies the image. Development
machines need ExifTool installed. Release packaging must ship a pinned,
licensed, platform-appropriate metadata adapter instead of relying on the
user's `PATH`.

`raf-preview` is a transport-agnostic state machine. Offline staging accepts
only a non-empty `.RAF` and selected local Recipe and does not enumerate or
open a camera. A model adapter may run the upload/apply/render/download path
only with an exact USB ID + model + firmware capability record, and it must
implement cleanup after any started operation. No such adapter is bundled yet.

## Suggested verification commands

```bash
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run check
npm run test
npm run build
```

For a physical X-M5, the next safe manual check is:

```bash
cargo run -p fuji-test -- ptp-descriptor D18C
```

This reads the descriptor metadata for the custom-slot selector. It can reveal the camera's current selected-slot scalar as part of the standard descriptor dataset, but it does not select a slot, read C1-C4 recipes, or write a setting. Do not run property probes while another camera application is connected.

On the physical X-M5 firmware 1.30, `GetDevicePropDesc(D18C)` returns `0x2002` after an empty data phase. `GetDevicePropValue(D18C)` succeeds and returns two raw bytes (`01 00`). Treat that as a transport observation only: the value encoding and any relationship to a custom slot still need controlled, reversible calibration before being shown as C1-C4 data.

On a physical device, preserve the raw `fuji-test` output with the serial number removed before sharing it in an issue.
