# Fingerprint plugin example

A complete Tauri 2 and Svelte 5 example for the DigitalPersona U.are.U 4500
fingerprint reader.

The example includes:

- reader detection and device information;
- validated fingerprint enrollment;
- a PNG preview for every completed enrollment scan;
- local or in-memory template verification;
- adjustable matching sensitivity;
- local one-to-many identification;
- diagnostic PGM capture; and
- the JSON template payload intended for server storage.

## Requirements

- Tauri 2 system dependencies;
- Rust 1.77.2 or later;
- Bun; and
- a DigitalPersona U.are.U 4500 reader.

## Start the example

From the repository root:

```bash
cd examples/tauri-app
bun install
bun run start
```

The Tauri configuration uses Bun for both `beforeDevCommand` and
`beforeBuildCommand`, so another JavaScript package manager is not required.

## Build the example

```bash
bun run tauri build
```

On Linux, add a udev rule for USB device `05ba:000a` and stop `fprintd` from
claiming the reader. On Windows, configure the reader to use WinUSB.
