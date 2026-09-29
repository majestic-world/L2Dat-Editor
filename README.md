# L2ClientDat Editor - Majestic-World Edition

Lineage 2 DAT editor maintained by **Majestic-World Studio**.

## Features

- Open and decrypt Lineage 2 DAT files.
- Edit and save modifications back to DAT format.
- Support for multiple chronicles and encryption algorithms.
- Batch pack/unpack operations.
- Text formatting and editing.

## Requirements

- **JDK 17** for building and running with Gradle.
- **IntelliJ IDEA** (optional).

The repository includes the Gradle Wrapper; no separate Gradle installation is required.

## Run

From the repository root on Windows:

```powershell
.\gradlew.bat run
```

The application runs from `dist/` and loads its structures, enums and
configuration from `dist/data/`. Keep that directory available at runtime.

## Build and test

```powershell
.\gradlew.bat build
```

The build runs the tests and generates `dist/lib/l2-editor.jar` with its
dependencies. To launch it on Windows, run `Laucher.bat` from `dist/`.
