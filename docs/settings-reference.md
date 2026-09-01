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
  Shadow Tone (-4 to +4 in 0.5 steps), other tone controls,
  Portrait Enhancer LV, Clarity, Long Exposure NR, Lens Modulation Optimizer,
  and Color Space
- Still shooting context: ISO (AUTO 1–3 and the X-M5 standard/extended range),
  exposure compensation, photometry, focus mode, AF mode, drive mode, and
  shutter type

## Camera-write boundary

The settings editor is intentionally broader than the current USB write path.
Every setting is preserved in the local SQLite library and portable JSON.
However, `Import to camera` sends only X-M5 vendor PTP properties with a
model/firmware-specific encoding, read-back check, and verified rollback.
Unvalidated values fail before any write is sent. This prevents a complete UI
option list from being mistaken for evidence that every Fujifilm body exposes
the same USB property or encoding.

## Sources

- [FUJIFILM X-M5 Owner's Manual (English PDF)](https://fujifilm-dsc.com/en-int/manual/x-m5/x-m5_manual_en_s_f.pdf)
- [FUJIFILM X-M5 Owner's Manual (Traditional Chinese PDF)](https://fujifilm-dsc.com/en-int/manual/x-m5/x-m5_manual_zht_s_f.pdf)
