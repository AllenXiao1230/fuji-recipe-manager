# Fuji Recipe Manager project audit

Audit updated: 2026-09-06. This separates verified implementation facts from
work that must not be treated as camera-write support until it has been tested
against a specific model and firmware.

## What is working well

- The Rust crates split USB transport, standard PTP framing, Fujifilm
  recognition, and X-M5 capability data. This is the right boundary for adding
  models without exposing raw PTP writes to the UI.
- X-M5 writes are explicitly gated to USB ID `04CB:030C` and firmware `1.30`.
  The write transaction captures a durable pre-write backup, journals progress,
  reads every value back, restores on failure, and restores the previously
  active C slot.
- The UI distinguishes locally stored Recipe settings from values that have
  actually passed X-M5 write/read-back validation. This prevents a broad
  editor from implying universal camera support.
- The X-M5 physical test evidence covers the currently enabled write path,
  including a 15-field GUI transaction and verified recovery.
- The capability record now distinguishes internal transport controls,
  snapshot-only fields, camera-rejected fields, and unknown fields. The new
  fixed-scope audit records evidence without turning into a generic PTP writer.

## Completed in this review

- Added a single canonical value-label table. Traditional Chinese now displays
  Chinese values across all existing selects, while the English interface uses
  official English names. Internal JSON and PTP values are unchanged.
- Parameter labels remain bilingual in the Chinese interface.
- Recipe-text import now recognizes Chinese displayed values and Chinese field
  names such as `軟片模擬`, `亮部色調`, `高 ISO 降噪`, and Color Chrome FX
  Blue, in addition to the existing English forms.
- Added non-destructive compatibility notices for HEIF, D Range Priority, and
  Dynamic Range minimum ISO interactions documented for X-M5.

## Priority 0 — release blockers

1. **Model and firmware evidence.** Only X-M5 firmware 1.30 is allowed to
   write. Every other body and firmware must remain probe-only until the full
   property-descriptor, disposable-slot, read-back, recovery, and regression
   checklist passes.
2. **Windows validation and signing.** The macOS build has been tested, but
   the Windows installer, Windows USB ownership behaviour, and signed release
   path have not been verified. A cross-platform product cannot claim Windows
   support until this is completed on hardware.
3. **Production signing and notarization.** The current macOS package is
   ad-hoc signed for local testing. Distribution requires a Developer ID
   signature, notarization, and a fresh download/install verification.

## Priority 1 — data integrity and protocol safety

1. **Recovery retention and export.** SQLite is the desktop authority and
   interrupted journals are visible with explicit verified restore. The next
   improvement is a visible backup retention policy, export, integrity status,
   and non-destructive cleanup. Never prune the only backup for a slot
   automatically.
2. **Capability fixtures.** Property IDs, allowed values, firmware identity,
   canonical read-back aliases, and bilingual labels are bundled in a
   versioned X-M5 capability record. Add sanitized fixtures and schema checks
   for each future model/firmware so capability changes can be replayed in CI.
3. **Evidence promotion discipline.** The read-only audit correctly preserves
   observations for `D191`, `D1A5`, and unknown codes. Add a reviewed trace
   attachment workflow before any candidate is promoted to a hardware write
   test.

## Priority 2 — camera semantics and usability

1. **Official menu coverage boundaries.** X-M5 also exposes MONOCHROMATIC
   COLOR, film-simulation dial assignment, pixel mapping, custom-mode mode,
   auto-update custom settings, and mount-adaptor settings. Some are
   maintenance, lens-specific, or global camera behaviour rather than Recipe
   parameters. Classify them before adding them; do not silently add values or
   PTP encodings without physical evidence.
2. **Availability rules.** The new notices cover three documented conflicts.
   Extend this into model-aware availability metadata: e.g. shooting mode,
   ISO, lens, HEIF, film simulation, and photo/movie context should control
   whether a value is selectable, warning-only, or excluded from a write.
3. **Recipe provenance.** Preserve original URL and author as now, but add
   optional license/permission, imported-at timestamp, parser confidence, and
   an editable import report. Never scrape a site merely from a pasted URL.
4. **Accessibility and desktop ergonomics.** Add keyboard focus tests,
   visible focus states, concise control descriptions, and a desktop-sized
   layout test at 1200×640 and a larger monitor. Avoid mobile-oriented layout
   work unless requirements change.

## Priority 3 — engineering quality

1. **Frontend tests.** Type-checking catches syntax but not parser, label,
   import, undo, modal, or setting-dependency behaviour. Add unit tests for
   `normalizeRecipe`, `parseRecipeText`, option localization, and capability
   preflight; add Tauri UI smoke tests for backup and restore dialogs.
2. **Hardware fixtures.** Store sanitized USB/PTP traces and expected
   property payloads outside production secrets. Replay them in CI to guard
   property encoding and rollback behaviour without needing a camera.
3. **Security hardening.** Replace the unrestricted Tauri CSP with the
   narrowest production CSP compatible with the UI, review external-link
   handling, and run dependency advisories in CI. Test developer-mode CSP
   separately so it does not weaken release builds.
4. **Release automation.** CI should build and test both macOS and Windows,
   attach checksums/SBOMs, run signing/notarization in protected release jobs,
   and publish explicit supported-model/firmware evidence with every release.

## Official references

- [X-M5 Image Quality Setting (Still Photography)](https://app.fujifilm-dsc.com/en/manual/x-m5/menu_shooting/image_quality_setting/)
- [X-M5 Menu List](https://app.fujifilm-dsc.com/en-int/manual/x-m5/introduction/menu_list/index.html)
- [X-M5 Quick Menu](https://app.fujifilm-dsc.com/en-int/manual/x-m5/shortcuts/quick_menu/)
