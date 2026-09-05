# X-M5 unverified PTP-property research

This document defines the evidence boundary for X-M5 firmware 1.30 properties
which are visible over USB/PTP but are not part of the application writer.
It applies to the physical identity `04CB:030C` / `FUJIFILM X-M5` / firmware
`1.30` only.

## Non-negotiable safety rule

An observable value, a descriptor access flag, or an example from another
Fujifilm model does **not** authorise a write. In particular, PTP response
`201C` means that the camera rejected the value in its present context; it is
not an invitation to retry arbitrary values.

The following command is the only new probe approved for this research:

```bash
cargo run -p fuji-test -- ptp-audit-xm5-unverified
```

It is deliberately fixed to `D191`, `D1A5`, and the current blocked vendor-code
list. It performs only `GetDeviceInfo`, `GetDevicePropValue`, and
`GetDevicePropDesc`. It does not select a custom slot and cannot send
`SetDevicePropValue`.

Save the output with any serial number removed. Each `AUDIT` row contains the
PTP code, stable local key, raw value result, and descriptor result. A descriptor
error is valuable evidence too: Fujifilm X-M5 already returns a general error
for some descriptor requests while a direct value read succeeds.

## Current disposition

| Codes | Status | Why it stays locked |
|---|---|---|
| `D1A3` Long Exposure NR | `write_rejected` | X-M5 firmware 1.30 rejected both On and Off with `201C`; the captured value remained unchanged. |
| `D193` monochrome warm/cool, `D194` monochrome magenta/green | `write_rejected` | With ACROS selected, direct and ×10 candidate representations were rejected with `201C`. |
| `D191`, `D1A5` | `read_detected_unverified` | They are present in a C-slot raw snapshot but have no X-M5 semantic or reversible-write record. |
| `D018`, `D01C`, `D023`, `D029`, `D02E`, `D030`, `D031`, `D032`, `D041`, `D16E`, `D208`, `D20B`, `D212`, `D21C`, `D320`, `D321`, `D34D`, `D36A`, `D36B` | `blocked_unknown` | Only model-local, read-only evidence may be collected. Their function and ownership are unknown. |

`D191` and `D1A5` are now explicit fields in the bundled X-M5 capability
record. They are intentionally excluded from the encoder, the restore allow
list, and raw-snapshot replay.

## X RAW Studio packet-comparison procedure

Use this only when a physical X-M5 is available and after closing this app and
all other PTP clients. This process is observational: X RAW Studio is the only
program that changes the setting.

1. Put a disposable C slot into a known baseline state and photograph the
   camera menu values.
2. Start a USB bulk-transfer capture, connect X RAW Studio, and save a baseline
   preset without changing an option.
3. Change **one** setting in X RAW Studio, save the same C slot, then read it
   back in X RAW Studio. Stop the capture.
4. Restore the baseline through X RAW Studio, save it, and capture the
   read-back. Keep the two capture files as a matched pair.
5. Compare only PTP `SetDevicePropValue (0x1016)` data containers and their
   response containers. Record property code, byte length, raw bytes, write
   order, response code, read-back bytes, model, firmware, and the menu
   prerequisite.

Never publish a capture without checking for serial numbers, filenames, image
paths, or personal metadata.

## Promotion gates

No property is promoted after a single successful command. A candidate needs:

1. an X-M5 1.30 capture showing X RAW Studio's exact set/read-back sequence;
2. a named model-local semantic and dependency, not an inferred X100VI mapping;
3. a disposable-slot application test: capture → write → read-back → restore
   → restore read-back;
4. explicit recovery bytes, including canonicalisation cases; and
5. updated capability JSON, encoder, restore allow list, unit tests, and
   macOS/Windows hardware evidence.

For `D193`/`D194`, the capture must select a monochrome simulation and change a
non-zero warm/cool or magenta/green value. For `D1A3`, it must prove whether
X RAW Studio sends a direct custom-slot property write at all. If it does not,
the field remains outside Recipe writing even if its value appears in a slot
snapshot.

## 1.25× crop is a feature dependency, not an unknown encoding

The `P 3:2`, `P 16:9`, and `P 1:1` image-size values depend on Sports Finder
Mode or 1.25× high-speed burst. They must not be tested as bare `D18E` values.
Before any future write support, capture the complete X RAW Studio or camera
menu transition that enables the prerequisite, identify every changed property,
and validate the complete transaction plus restoration. Until then the UI keeps
these options unavailable.

## External comparison boundary

[FilmKit](https://github.com/eggricesoy/filmkit) documents useful property
encodings from its X100VI work, including conditional monochrome fields and
unresolved `D191`/`D1A5` values. [Latent](https://github.com/formray/latent)
keeps X-M5 experimental pending hardware verification. Both are protocol
references, not X-M5 write permission.
