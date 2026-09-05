# Camera capability matrix

This matrix prevents a USB product name from being mistaken for write support.

## Local capability-matrix database

The desktop app includes a local SQLite `local_capability_matrices` table for
documenting a specific Fujifilm camera by **USB ID + model + firmware**. A
matrix can be created from a read-only PTP DeviceInfo result or entered
manually, then edited to retain:

- custom-slot count and labels;
- field name in Traditional Chinese and English;
- observed PTP property code and scope;
- `read_detected_unverified`, `write_rejected`, or `blocked_unknown` state;
- a source URL and evidence/dependency/test notes.

Use the official camera specification and owner’s manual as the primary source
for menu options and dependencies. For example, Fujifilm’s [X-M5
specifications](https://www.fujifilm-x.com/en-us/products/cameras/x-m5/specifications/)
list 20 Film Simulation modes and Dynamic Range/White Balance constraints, and
the [X-M5 owner’s manual](https://fujifilm-dsc.com/en/manual/x-m5/) describes
when image-quality settings are available. Community lists and statistical
datasets may be stored as a source URL or note, but they remain unverified
evidence.

**A local matrix cannot mark a field `write_verified`, cannot enable PTP
writes, and cannot override a compiled capability record.** Promotion to a
trusted write record still requires the reversible per-model hardware process
below.
Every Fujifilm USB device is discoverable; only a model and firmware record that
has completed the listed gates may expose a Recipe write action.

| Support level | Meaning | UI behaviour |
|---|---|---|
| Probe required | Fujifilm device recognised, but PTP property encoding is unknown. | Discovery and explicit read-only identity probe only. |
| Read-only validated | Standard PTP probe and selected raw-property reads recorded. | Read-only camera inspection. |
| Experimental write | Model, USB ID, firmware and field codec have a reversible hardware record. | Explicit write confirmation, durable backup, journal, read-back and restore only. |
| Production write | Full multi-slot hardware matrix, Windows/macOS recovery, release QA and documentation complete. | Normal write workflow. |

## Current records

| Family | Models recognised | Highest current support |
|---|---|---|
| X Series | X-M5, X-S10, X-S20, X-T3, X-T4, X-T5, X-T30, X-T50, X-H1, X-H2, X-H2S, X-Pro3, X-E3, X-E4 and unknown X bodies | X-M5 firmware 1.30: Experimental write; all other bodies: Probe required |
| X100 | X100F, X100V, X100VI and unknown X100 bodies | Probe required |
| GFX | GFX50S II, GFX100S, GFX100S II, GFX100 II and unknown GFX bodies | Probe required |
| FinePix and other Fujifilm | Vendor-recognised devices not listed above | Probe required |

## FUJIFILM X-M5 — firmware 1.30

The machine-readable source of truth is
[`data/capabilities/fujifilm-xm5-1.30.json`](../data/capabilities/fujifilm-xm5-1.30.json).
It matches USB ID `04CB:030C`, DeviceInfo model `FUJIFILM X-M5`, and firmware
`1.30`. Its results came from a physical disposable-slot sequence: capture →
write → read-back → restore → restore read-back. The app bundles this record
and shows every property status before an X-M5 import.

| Status | Fields | Boundary |
|---|---|---|
| Write verified | Preset name, every non-Auto Film Simulation, Dynamic Range (including Auto), Grain, Color Chrome Effect, Color Chrome FX Blue, Smooth Skin Effect, White Balance (except Custom 1–3) and shift, Color Temperature, Highlight, Shadow, Color, Sharpness, High ISO NR, Clarity, Color Space | The encoder exposes only the values that passed the physical test. Highlight and Shadow are `-2.0…+4.0` in 0.5 steps. |
| Write verified, narrow payload | Image Size (`S 3:2`, `D18E=01 00`; `S 16:9`, `D18E=02 00`; `S 1:1`, `D18E=03 00`; `M 3:2`, `D18E=04 00`; `M 16:9`, `D18E=05 00`; `M 1:1`, `D18E=06 00`; `L 3:2`, `D18E=07 00`; `L 16:9`, `D18E=08 00`; `L 1:1`, `D18E=09 00`); Image Quality (`RAW`, `D18F=01 00`; `FINE`, `D18F=02 00`; `NORMAL`, `D18F=03 00`; `FINE+RAW`, `D18F=04 00`; `NORMAL+RAW`, `D18F=05 00`) | The properties may be readable with other values, but those payload encodings stay blocked until they have their own reversible record. |
| Mode-dependent / blocked | `P 3:2`, `P 16:9`, `P 1:1` at 1.25× crop | Fujifilm exposes these only in Sports Finder Mode or 1.25× high-speed burst. The prerequisite drive/crop state is not yet a C-slot Recipe property, so the app shows the choices but cannot write them. |
| Rejected by camera | Long Exposure NR (`D1A3`); Monochromatic Color (`D193`); Monochromatic MG (`D194`) | The tested writes received PTP `201C`. Both monochrome properties remain locked; any future test must use ACROS or MONOCHROME as its prerequisite. |
| Detected but unverified | Global `D001/D007/D008/D00A/D00B/D00C/D017/D104` | Read/current-value transport observations do not authorise a Recipe write. |
| Unknown / blocked | Other scanned vendor properties | Never exposed as writable controls. |

### Raw snapshot boundary

The app can retain a user-requested raw C-slot snapshot as Recipe
interoperability metadata. It is bounded to the X-M5 custom-slot property
window, includes readable rejected/unknown fields, and records failed reads by
property code. The snapshot is never fed to the camera writer or recovery
allow-list, so it cannot turn an observed vendor value into an arbitrary write.

For unverified Image Size and Image Quality values, first set the desired value
manually on a disposable C slot and capture it with
`ptp-capture-xm5-image-payload`. This only produces a candidate mapping; the
matrix remains locked until reversible hardware verification is retained.

### Dependencies recorded for X-M5

- Write `D199` White Balance as **Color Temperature** (`8007`) before writing
  `D19C` Color Temperature. The app therefore omits `D19C` under all other
  White Balance modes.
- Any future black-and-white colour test must select ACROS or a MONOCHROME
  simulation first. The current tested commands were rejected and are not
  emitted by the encoder.
- HEIF can cause the camera to force sRGB; the Recipe stores Color Space but
  the camera may canonicalise it in that mode.
- Film Simulation **Auto** and White Balance **Custom 1–3** remain locked: no
  C-slot write mapping and captured-value recovery record has been retained.

## Required record for a new model or firmware

1. Record product ID, PTP interface, DeviceInfo model and firmware without storing a serial number.
2. Add a versioned JSON record under `data/capabilities/`, keyed by USB ID, model and firmware. Store sanitised `GetDevicePropValue` fixtures for custom-slot selection, preset name and every candidate Recipe property.
3. Define the model codec in a dedicated crate: user value, write command bytes, accepted read-back bytes, and restore command bytes.
4. Add unit tests for normal values, rejected values, and every canonicalisation exception.
5. On a disposable slot, run backup → write → read-back → restore → read-back on macOS and Windows.
6. Mark the record Experimental write only after the exact model/firmware test is retained. Production write additionally requires repeated multi-slot QA and release documentation.

## Compatibility policy

Recipe import is always local. A Recipe may be edited on any machine, but the
app must show unsupported fields and refuse the camera write until the connected
model/firmware record supports every selected field. New camera firmware starts
at Probe required even when the USB product ID is unchanged.
