# L2ClientDat Editor - Majestic-World Edition

L2ClientDat is a powerful Lineage 2 Dat Editor maintained by **Majestic-World Studio**.<br>
This software is intended to assist developers and players in managing, modifying, <br>
and understanding the data used by Lineage 2.


## Features

- Open and decrypt Lineage 2 .dat files
- Save modifications back to .dat format
- Support for multiple Lineage 2 chronicles
- Mass pack/unpack operations
- Advanced text formatting and editing
- Multiple encryption/decryption algorithms support

## Native Rust application

The native application lives in `rust/`. Its desktop UI uses `egui`; DAT
cryptography, XML interpretation, text formatting and batch operations run in
Rust without a JVM. The original Java project remains available.

The desktop follows a graphite-and-copper editor layout: a workspace sidebar
for recent files, chronicle and encryption settings; a compact action toolbar;
a document tab and path; and a resizable output dock. The sidebar scrolls on
smaller windows. DAT markers, keys, numbers and bracketed strings have subtle
syntax highlighting without changing the underlying text or its line layout.
The active document keeps its original read options; sidebar changes apply to
the next open, except encryption, which controls the next save.

The [design reference](rust/design/reference.png) was generated with Higgsfield
GPT Image 2.5. The 13 outline SVG icons in `rust/assets/icons` use a 20 px grid
and 2 px strokes. They are embedded in the executable; no network or external
icon files are needed at runtime. Each icon is rasterized and cached by physical
pixel size, so simultaneous sizes and DPI/zoom changes never stretch a smaller
cached texture. Antialiased SVG pixels are sampled 1:1 without bilinear blur.
Native captures: [100%, 150% and 200% comparison](rust/design/icons-comparison.png)
and [complete workspace](rust/design/icons-workspace.png). DPI transitions were
simulated per window with `WM_DPICHANGED`, without changing desktop settings.

The application icon is a custom copper L2 monogram with stacked DAT-record
strokes, generated through Higgsfield Recraft V4.1. Its original vector is
preserved in [`rust/design/app-icon-source.svg`](rust/design/app-icon-source.svg).
[`rust/assets/app-icon.ico`](rust/assets/app-icon.ico) contains independently
rasterized 16, 20, 24, 32, 40, 48, 64, 96, 128 and 256 px images with transparent
corners. `rust/build.rs` embeds it in Windows executables using `winresource`;
the build reruns when the ICO changes. The normalized display vector is embedded
from `rust/assets/app-icon.svg`; `app-icon.png` remains a 256 px preview.
On Windows the title-bar icon is rasterized directly at 16 logical pixels times
the native display scale, and updated when the window's DPI changes. Native icon
handles were verified at 16, 24 and 32 px and on returning to 16 px. Other
platforms receive a 256 px raster. No external icon files are required at runtime.
The executable's ten icon resources were checked against the source ICO, and
the [running window icon](rust/design/app-icon-window.png) was verified.

Actual Windows captures: [open DAT](rust/design/editor.png) and
[empty workspace](rust/design/empty-state.png). These use synthetic data and
isolated preferences. The redesign was exercised with a 200-record encrypted
DAT: search, go-to-line, Unicode editing, save and reopen, batch extraction,
and compact-window rendering with horizontal scrolling instead of soft wraps.

### Run

Install Rust and a native linker (Visual Studio Build Tools with the C++ desktop
workload and Windows SDK, including `rc.exe`, on Windows), then run from the
repository root:

```powershell
cargo run --manifest-path rust/Cargo.toml --release
```

To build the Windows executable:

```powershell
cargo build --manifest-path rust/Cargo.toml --release
.\rust\target\release\l2dat-editor.exe
```

The application reads the existing `dist/data` directory directly. It does not
duplicate or modify the XML structures, enums or encryption-key configuration.
For deployment, place that directory as `data` beside the executable, or specify
`--data-dir <path>` / `L2DAT_DATA_DIR`. The directory must contain `structure`,
`enums`, `definitions.xml` and `config`.

### Editor

- Startup uses at most a 1100-by-700-point client area, including when older
  preferences contain a larger size. On Windows, the outer window is centered
  in the primary monitor's work area and limited to 85% of it, accounting for
  display DPI, taskbar space and window decorations. Manual resizing remains available.
- Open DAT, INI, HTM and UTF-8/UTF-16 text; drag-and-drop and recent files are
  supported. Saving unstructured text preserves its BOM and CRLF convention.
- Edit with aligned line numbers, undo/redo, Unicode case-insensitive search,
  previous/next matches, literal replacement and go-to-line. Save DAT or export
  UTF-8 TXT. `Ctrl+O`, `Ctrl+S`, `Ctrl+F` and `Ctrl+G` open, save, search and
  navigate; clipboard actions are also available from the editor context menu.
- The Search toolbar button, Edit menu, editor context menu and `Ctrl+F` open
  a movable, non-modal search-and-replace window with previous/next navigation,
  match counts and literal replace-all feedback. Query and replacement text
  survive closing and reopening the window.
- Click the editor to edit, undo, duplicate lines or save while search stays
  open. Matches refresh after document edits. Enter in the search field
  navigates matches without editing the document; keyboard input follows focus.
  Escape or Close dismisses search and returns focus to the editor selection.
  Replace-all can be undone in one action.
- Log messages wrap to the panel width and scroll vertically only.
- The native application window and startup error dialog use the title
  `L2DAT Studio By Mk`.
- The header badge displays `L2DAT Studio v{version} - By Mk`, using the
  package version from `rust/Cargo.toml` at compile time.
- `Ctrl+D` duplicates the current line or complete selected lines below, keeping
  the caret column or selection direction. Duplication is one undo/redo action
  and preserves existing line endings without changing the clipboard.
- Saving uses an 80-by-4-point progress bar with a compact activity spinner.
  Progress advances at real encoding, encryption, persistence and reopening
  boundaries, not by elapsed time or estimated byte percentages. Operations
  without measurable progress show only the activity spinner.
- Saving and Save As preserve the open session's exact text, blank lines,
  spacing, selection, horizontal/vertical scroll, search state and undo/redo.
  Structured DAT encoding still excludes layout-only blank lines; they remain
  in the editor for debugging, not in the DAT. The saved file is fully reopened
  for validation, but only its metadata replaces the session metadata.
- Large RSA payloads encrypt independent blocks in parallel, bounded by the
  available parallelism and eight workers. Small payloads stay sequential.
  Workers share read-only keys and write disjoint output slices in file order.
  Compression, keys, envelope bytes and decryption checks are unchanged.
  This trades temporary CPU parallelism for shorter saves; no dependencies
  or persistent worker pool are added. Same-directory temporary files,
  `sync_all`, atomic replacement and the UI's full reopen remain in place.
- The text viewport lays out and highlights visible physical lines plus a small
  overscan, rather than building glyph geometry for the entire document.
  Text and the line index remain in memory; an individual very long line is
  still laid out in full. Selection, search and editing use whole-document
  UTF-8 offsets, including lines outside the viewport.
- Editor scrollbars reserve their own space instead of covering text, with an
  8-point gap from the content and a 32-point minimum handle length. Dimensions
  scale with display DPI.
- Undo/redo stores changed ranges instead of whole-document snapshots. History
  retains at most 100 edits with a 128 MiB payload budget; the newest edit stays
  undoable even when a single paste exceeds that budget.
- Select the chronicle before opening. Chronicle, formatter and enum settings
  stay attached to the open document; changed settings apply to the next open.
- `Source` preserves the original encryption key when an encryption key with
  that name exists. Original RSA decode-only keys require an explicit choice,
  such as `v413_encdec` or `v413_encdec_bonux`; there is no silent key fallback.
- Save-as filenames must match a descriptor in the selected chronicle.
  Structured DAT output requires a structure-enabled key or `Plaintext`.
- Recursive unpack, pack and re-encryption use separate, non-nested output
  directories and refuse to overwrite existing outputs. Cancellation takes
  effect between files.
- Keep `L2GameDataName.dat` beside files that use shared names. Unpack carries
  the dictionary into the output directory; save installs appended names before
  the referencing DAT. Without a dictionary, numeric IDs remain editable, but
  adding new named entries is rejected.
  Names with unbalanced brackets, excessive nesting or reserved `<StrID:...>`
  syntax are shown using their original `[<StrID:N>]` reference instead of
  emitting ambiguous text. Saving preserves those IDs and dictionary entries.
- Settings are stored per user in `%APPDATA%\L2DatEditorRust\settings.json`
  (or the XDG configuration directory on other platforms).
- On Windows, installed Segoe UI, Malgun Gothic and Microsoft YaHei fonts are
  used when available, including fallback for Korean and Chinese text.

### Command line

Omitting a subcommand opens the desktop application. File and directory paths
are accepted by `unpack`, `pack` and `recrypt`; existing output files are rejected.

```powershell
cargo run --manifest-path rust/Cargo.toml --release -- catalog
cargo run --manifest-path rust/Cargo.toml --release -- --chronicle "Samurai (542)" unpack .\input\sysstring-e.dat .\text\sysstring-e.txt
cargo run --manifest-path rust/Cargo.toml --release -- --chronicle "Samurai (542)" --encryption v413_encdec pack .\text\sysstring-e.txt .\output\sysstring-e.dat
cargo run --manifest-path rust/Cargo.toml --release -- --encryption v413_encdec_bonux recrypt .\input .\reencrypted
```

Use `--no-formatter` and `--no-enums` for unjoined records and numeric enum
values. Use the same settings when unpacking and packing text.

### Compatibility and verification

The port implements the six bundled formatters and the configured XOR,
Blowfish, DES and RSA/zlib algorithms. RSA and XOR were compared against Java
in both directions; Blowfish blocks against the bundled Java engine; DES
decryption against Java. The original Java Blowfish wrapper and DES encryption
path are incomplete, so they cannot serve as full-file encryption oracles.
XOR 121 follows the configured fixed key, not external filename-derived variants.
Legacy Blowfish/DES preserve incomplete final blocks.

Recognized legacy footers are removed for XOR/ECB as well as RSA, avoiding the
extra bytes returned by the Java XOR/ECB wrappers. Legacy unauthenticated
formats cannot reliably distinguish every wrong key or a payload ending in the
same footer sentinel.

Existing XML defects are reported at startup and by `catalog`: six missing
chronicle-parent references and a missing counter in `hairgrp.xml` / `interlude`.
The port does not guess replacements. Valid own/inherited descriptors remain
usable; affected lookups fail explicitly. Malformed counts, missing
`SafePackage`, trailing binary data and malformed formatter records are rejected
rather than silently discarded.

```powershell
cargo test --manifest-path rust/Cargo.toml
```

Verification uses synthetic records, the real bundled XML and differential
checks against the Java implementation. It is not a claim that every DAT from
every client build has been validated.

Large-file UI verification used `Npcgrp_Classic.dat` (356 KiB compressed,
16.6 MiB decoded, 16,120 lines). The old release exceeded 1,715 MiB of private
memory within 3.1 seconds and was stopped at the measurement's safety limit.
The final virtualized release peaked at 330 MiB during a 15-second opening
probe, with no window-response timeouts. These are measurements on one Windows
machine, not memory limits or cross-machine performance guarantees.
Navigation to line 15,000, editing, undo/redo and full-document copying were
exercised in the native UI. A separate valid 20,000-record Unicode DAT
(14.9 MiB decoded text) was edited, saved, reopened and decoded for a complete
content comparison.

The NPC sample's dictionary contains a name with an unmatched `[` used by
`dialog_sound`. The editor now represents such mapped names by their original
string IDs. A real-file unpack/pack/reopen check preserved all 2,542,921
decrypted payload bytes. Native UI editing and `Ctrl+S` were also exercised;
the decoded result preserved all 16,120 records with only the intended field
change, and the dictionary remained unchanged. The original client DAT and
dictionary were not modified. Reopen DAT files in the corrected build to
regenerate valid text; already-open older instances retain the old decoding.

Save optimization was measured on isolated copies of the NPC sample, with five
release-mode saves per implementation on the same Windows machine. Median
save time fell from 2.11 s to 0.80 s; including the document snapshot and full
reopen, the median fell from 2.66 s to 1.34 s. This is a local measurement,
not a cross-machine performance guarantee. Sequential and parallel saves
produced identical 363,952-byte encrypted DAT files and identical dictionaries.
Native UI verification covered an NPC ID edit, save/reopen, rejection of a
missing required field without changing the previous DAT or dictionary,
retention of unsaved edits after failure, and recovery through undo/save.

The native modules are `schema` (XML and binary/text codec), `crypto` (DAT
envelopes), `format` (record transformations), `editor` (file operations),
`settings`, and the desktop `ui`. The visual layer is separated into `theme`
(shared colors and widget styling), `icons` (embedded SVGs), `text_state`
(line index, UTF-8 selection and delta history), `text_view` (virtualized
editing and input), `highlight` (visible-line text layout) and `ui/chrome`
(toolbar, sidebar, status and output).


## Java requirements

- **Java 17** or higher
- **IntelliJ IDEA**
- **Gradle**


## Credits

- **Current Maintainer**: Majestic-World Studio