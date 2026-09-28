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
GPT Image 2.5. The outline SVG icons in `rust/assets/icons` are embedded in the
executable; no network or external icon files are needed at runtime.

The application icon is a custom copper L2 monogram with stacked DAT-record
strokes, generated through Higgsfield Recraft V4.1. Its original vector is
preserved in [`rust/design/app-icon-source.svg`](rust/design/app-icon-source.svg).
[`rust/assets/app-icon.ico`](rust/assets/app-icon.ico) contains independently
rasterized 16, 20, 24, 32, 40, 48, 64, 96, 128 and 256 px images with transparent
corners. `rust/build.rs` embeds it in Windows executables using `winresource`;
the build reruns when the ICO changes. The matching embedded `app-icon.png`
sets the native window icon. Neither requires external files at runtime.
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

- Open DAT, INI, HTM and UTF-8/UTF-16 text; drag-and-drop and recent files are
  supported. Saving unstructured text preserves its BOM and CRLF convention.
- Edit with aligned line numbers, undo/redo, Unicode case-insensitive search,
  previous/next matches, literal replacement and go-to-line. Save DAT or export
  UTF-8 TXT. `Ctrl+O`, `Ctrl+S`, `Ctrl+F` and `Ctrl+G` open, save, search and
  navigate; clipboard actions are also available from the editor context menu.
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

The native modules are `schema` (XML and binary/text codec), `crypto` (DAT
envelopes), `format` (record transformations), `editor` (file operations),
`settings`, and the desktop `ui`. The visual layer is separated into `theme`
(shared colors and widget styling), `icons` (embedded SVGs), `highlight`
(cached text layout) and `ui/chrome` (toolbar, sidebar, status and output).


## Java requirements

- **Java 17** or higher
- **IntelliJ IDEA**
- **Gradle**


## Credits

- **Current Maintainer**: Majestic-World Studio