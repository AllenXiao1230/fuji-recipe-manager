# X-M5 Recipe settings reference

This document defines the user-facing still-photography settings that Fuji
Recipe Manager can store in a Recipe. The option names and limits are based on
the **FUJIFILM X-M5 Owner's Manual**, still-photography menus, pp. 130–146,
151, 177, and technical specifications (pp. 429–430).

## Stored Recipe settings

The Recipe format includes the following official X-M5 groups:

- Image size, image quality, RAW recording compression, and JPEG/HEIF choice
- All film simulations, including ACROS and MONOCHROME yellow/red/green
  filters
- Grain Effect, Color Chrome Effect, Color Chrome FX Blue
- White Balance: White Priority, Auto, Ambience Priority, Custom 1–3, Color
  Temperature (2500–10000 K, 10 K increments), Daylight, Shade, three
  fluorescent types, Incandescent, and Underwater
- Dynamic Range (AUTO/100%/200%/400%), D Range Priority, Highlight and
  Shadow Tone (-2 to +4 in 0.5 steps on the X-M5), other tone controls,
  Portrait Enhancer LV, Clarity, Long Exposure NR, Lens Modulation Optimizer,
  and Color Space
- Still shooting context: ISO (AUTO 1–3 and the X-M5 standard/extended range),
  exposure compensation, photometry, focus mode, AF mode, drive mode, and
  shutter type

## Display and import terminology

Internal Recipe JSON stores stable English identifiers such as
`CLASSIC_CHROME` and `WHITE_PRIORITY`. In the Traditional-Chinese interface,
every available setting value is displayed in Traditional Chinese; parameter
labels remain Chinese plus English (for example, `軟片模擬 Film Simulation`).
The editor uses the X-M5 manual's terminology and values, including the
published image dimensions, white-balance light-source names, and the
film-simulation names. Pasted Recipe text accepts the canonical identifier,
the official English label, or the displayed Traditional-Chinese value for the
options currently understood by the parser.

The UI also calls out three X-M5 dependencies from the manual without silently
changing the stored Recipe: HEIF uses sRGB and disables Clarity; D Range
Priority automatically adjusts Dynamic Range and Tone Curve; DR 200% and
DR 400% require ISO 320 and ISO 640 respectively.

## Camera-write boundary

The settings editor is intentionally broader than the current USB write path.
Every setting is preserved in the local SQLite library and portable JSON.
However, `Import to camera` sends only X-M5 vendor PTP properties with a
model/firmware-specific encoding, read-back check, and verified rollback.
Unvalidated values fail before any write is sent. This prevents a complete UI
option list from being mistaken for evidence that every Fujifilm body exposes
the same USB property or encoding.

For X-M5 firmware 1.30, the current physical test record verified each
currently writable Recipe field. Enumerated controls were tested across all
supported values; numeric controls were tested at their minimum, neutral, and
maximum. Grain Off requires a two-step write to retain the requested Small or
Large size before the Off command. Highlight and Shadow below -2.0 are
rejected by the camera with PTP `201C`, so the editor and writer limit those
two controls to -2.0 through +4.0 in 0.5 steps.

The writer also limits image size to the nine verified standard S/M/L aspect
ratios and image quality to RAW, FINE, NORMAL, FINE+RAW, and NORMAL+RAW. The
`P 3:2`, `P 16:9`, and `P 1:1` 1.25× crop choices stay unavailable because
their Sports Finder or high-speed-burst prerequisite is not yet represented as
a restorable Recipe transaction. Film Simulation Auto, White Balance Custom
1–3, Long Exposure NR, and monochrome warm/cool or magenta/green controls are
stored locally when applicable but are not emitted by the X-M5 writer. See the
[camera capability matrix](camera-capability-matrix.md) for the authoritative
per-field state and the [unverified-property research procedure](xm5-unverified-property-research.md)
for the evidence required to change it.

## Sources

- [FUJIFILM X-M5 Owner's Manual (English PDF)](https://fujifilm-dsc.com/en-int/manual/x-m5/x-m5_manual_en_s_f.pdf)
- [FUJIFILM X-M5 Owner's Manual (Traditional Chinese PDF)](https://fujifilm-dsc.com/en-int/manual/x-m5/x-m5_manual_zht_s_f.pdf)
