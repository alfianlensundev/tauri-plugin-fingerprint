# tauri-plugin-fingerprint

A Tauri plugin for capturing, enrolling, verifying, and identifying
fingerprints with the DigitalPersona **U.are.U 4500** USB reader
(`05ba:000a`).

The plugin communicates with the reader through `nusb` and uses MINDTCT and
BOZORTH3 from `fprint-pipeline` for minutiae extraction and matching.

> **Pure Rust implementation:** this plugin communicates directly with the USB
> reader and processes fingerprints entirely in Rust. It does not use a bridge
> to a native library, vendor SDK, external fingerprint service, or additional
> runtime. No proprietary DigitalPersona driver needs to be installed. On
> Windows, the reader only needs to be associated with the built-in WinUSB
> driver; Linux may require a udev permission rule.

## Installation

### Minimum requirements

| Requirement | Minimum version or value |
| --- | --- |
| Tauri | `2.0` |
| Rust | `1.77.2` |
| Supported reader | DigitalPersona U.are.U 4500 (`05ba:000a`) |
| Supported platforms | macOS, Linux, and Windows |

Android and iOS are not supported because the implementation communicates
directly with a desktop USB fingerprint reader.

### 1. Install the Rust crate

Run the following command from your application's `src-tauri` directory:

```bash
cargo add tauri-plugin-fingerprint@1.0.2
```

Alternatively, add the crate manually to `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri-plugin-fingerprint = "1.0.2"
```

### 2. Register the plugin

Register the plugin in your Tauri builder:

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_fingerprint::init())
    .run(tauri::generate_context!())
    .expect("error while running Tauri application");
```

### 3. Install the JavaScript package

```bash
npm install tauri-plugin-fingerprint@1.0.2
```

You can also use another package manager:

```bash
pnpm add tauri-plugin-fingerprint@1.0.2
yarn add tauri-plugin-fingerprint@1.0.2
bun add tauri-plugin-fingerprint@1.0.2
```

### 4. Add the capability permission

Add `fingerprint:default` to a capability file such as
`src-tauri/capabilities/default.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "windows": ["main"],
  "permissions": ["core:default", "fingerprint:default"]
}
```

### Platform requirements

| Platform | Requirement |
| --- | --- |
| macOS | No additional driver is required. The vendor-class reader can normally be accessed without `sudo`. |
| Linux | No additional driver is required. Add a udev permission rule for USB device `05ba:000a` and ensure `fprintd` is not using the reader. |
| Windows | No proprietary driver is required. Associate the reader with the WinUSB driver included with Windows. |

## Usage

All functions are asynchronous and must be called from the Tauri webview.
Fingerprint operations are serialized internally so that two operations cannot
access the reader at the same time.

### Svelte

```svelte
<script lang="ts">
  import { enroll, verify, type EnrollResult, type VerifyResult } from 'tauri-plugin-fingerprint'

  let enrollment: EnrollResult | null = null
  let verification: VerifyResult | null = null

  async function enrollFingerprint() {
    enrollment = await enroll({
      username: 'alfian',
      finger: 'right-index',
      samples: 4,
      includeImages: true,
      saveLocally: true,
    })
  }

  async function verifyFingerprint() {
    verification = await verify({
      username: 'alfian',
      threshold: 40,
    })
  }
</script>

<button onclick={enrollFingerprint}>Enroll</button>
<button onclick={verifyFingerprint}>Verify</button>

{#if enrollment?.images[0]}
  <img src={enrollment.images[0]} alt="Fingerprint preview" />
{/if}

{#if verification}
  <p>Matched: {verification.matched ? 'Yes' : 'No'}</p>
  <p>Score: {verification.score}</p>
{/if}
```

### Vue

```vue
<script setup lang="ts">
import { ref } from 'vue'
import {
  enroll,
  verify,
  type EnrollResult,
  type VerifyResult,
} from 'tauri-plugin-fingerprint'

const enrollment = ref<EnrollResult | null>(null)
const verification = ref<VerifyResult | null>(null)

async function enrollFingerprint() {
  enrollment.value = await enroll({
    username: 'alfian',
    finger: 'right-index',
    samples: 4,
    includeImages: true,
    saveLocally: true,
  })
}

async function verifyFingerprint() {
  verification.value = await verify({
    username: 'alfian',
    threshold: 40,
  })
}
</script>

<template>
  <button @click="enrollFingerprint">Enroll</button>
  <button @click="verifyFingerprint">Verify</button>

  <img
    v-if="enrollment?.images[0]"
    :src="enrollment.images[0]"
    alt="Fingerprint preview"
  />

  <template v-if="verification">
    <p>Matched: {{ verification.matched ? 'Yes' : 'No' }}</p>
    <p>Score: {{ verification.score }}</p>
  </template>
</template>
```

### ReactJS

```tsx
import { useState } from 'react'
import {
  enroll,
  verify,
  type EnrollResult,
  type VerifyResult,
} from 'tauri-plugin-fingerprint'

export default function Fingerprint() {
  const [enrollment, setEnrollment] = useState<EnrollResult | null>(null)
  const [verification, setVerification] = useState<VerifyResult | null>(null)

  async function enrollFingerprint() {
    const result = await enroll({
      username: 'alfian',
      finger: 'right-index',
      samples: 4,
      includeImages: true,
      saveLocally: true,
    })

    setEnrollment(result)
  }

  async function verifyFingerprint() {
    const result = await verify({
      username: 'alfian',
      threshold: 40,
    })

    setVerification(result)
  }

  return (
    <main>
      <button onClick={enrollFingerprint}>Enroll</button>
      <button onClick={verifyFingerprint}>Verify</button>

      {enrollment?.images[0] && (
        <img src={enrollment.images[0]} alt="Fingerprint preview" />
      )}

      {verification && (
        <div>
          <p>Matched: {verification.matched ? 'Yes' : 'No'}</p>
          <p>Score: {verification.score}</p>
        </div>
      )}
    </main>
  )
}
```

### Store the fingerprint template on a server

Set `saveLocally` to `false` when the application should store the template on
your server instead of the local application data directory:

```ts
import { enroll } from 'tauri-plugin-fingerprint'

const enrollment = await enroll({
  username: 'alfian',
  finger: 'right-index',
  samples: 4,
  includeImages: true,
  saveLocally: false,
})

await fetch('/api/fingerprints', {
  method: 'POST',
  headers: {
    'content-type': 'application/json',
  },
  body: JSON.stringify({
    template: enrollment.template,
  }),
})

const fingerprintPreview = enrollment.images[0]
```

`fingerprintPreview` is a PNG data URL in the following format:

```text
data:image/png;base64,...
```

The image is intended for preview or diagnostics. Fingerprint matching uses
`enrollment.template`, not the Base64 image.

To verify against a template retrieved from your server:

```ts
import { verify, type FingerprintTemplate } from 'tauri-plugin-fingerprint'

const response = await fetch('/api/fingerprints/alfian')
const data: { template: FingerprintTemplate } = await response.json()

const result = await verify({
  template: data.template,
  threshold: 40,
})

console.log(result.matched, result.score, result.threshold)
```

The template is a JSON object:

```json
{
  "version": 1,
  "username": "alfian",
  "finger": "right-index",
  "samples": [
    [
      { "x": 123, "y": 87, "theta": 210 }
    ]
  ]
}
```

Store the complete template without modifying its coordinates or angles.
Fingerprint templates are sensitive biometric data and should be encrypted in
transit and at rest, with access restricted to authorized services and users.

### API reference and options

| Function | Description |
| --- | --- |
| `checkDevice()` | Checks whether the DigitalPersona U.are.U 4500 can be opened. |
| `getDeviceInfo()` | Returns the model, firmware version, image dimensions, and image PPI. |
| `capture(options?)` | Captures one fingerprint and saves a diagnostic PGM image. |
| `captureFrames(options?)` | Captures and saves multiple diagnostic PGM frames. |
| `enroll(options)` | Captures multiple samples and creates a fingerprint template. |
| `verify(options)` | Performs a one-to-one match against a local or supplied template. |
| `identify(options?)` | Performs a one-to-many match against all locally saved templates. |

#### `capture(options?)`

| Option | Type | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `outputPath` | `string` | No | Generated in the application data directory | Complete destination path for the diagnostic `.pgm` image. Parent directories are created automatically. |
| `timeoutSecs` | `number` | No | `60` | Maximum wait time for the finger, from `1` to `300` seconds. |

The result contains `path`, `minutiaeCount`, `imageWidth`, and `imageHeight`.

#### `captureFrames(options?)`

| Option | Type | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `count` | `number` | No | `5` | Number of frames to capture. The allowed range is `1` to `20`. |
| `outputPrefix` | `string` | No | Generated in the application data directory | Path prefix used to create files such as `<prefix>0.pgm` and `<prefix>1.pgm`. |
| `timeoutSecs` | `number` | No | `60` | Maximum wait time for the finger, from `1` to `300` seconds. |

The result is an array containing the saved `path` and `minutiaeCount` for each
frame.

#### `enroll(options)`

| Option | Type | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `username` | `string` | Yes | — | Identifier stored in the template. It must be 1–128 characters and may contain letters, numbers, `-`, and `_`. |
| `finger` | `string` | Yes | — | Non-empty finger label, for example `right-index` or `left-thumb`. |
| `samples` | `number` | No | `4` | Number of times the user must place the finger on the reader. The allowed range is `1` to `10`. |
| `timeoutSecs` | `number` | No | `60` | Maximum wait time for each finger placement or removal, from `1` to `300` seconds. |
| `includeImages` | `boolean` | No | `false` | Includes one PNG Base64 data URL for each scan in the `images` result. |
| `saveLocally` | `boolean` | No | `true` | Saves the generated template to the application data directory under `fingerprint/prints/<username>.fpt`. Set it to `false` when storing the template on a server. |

The result contains the JSON `template`, optional preview `images`, local
`path`, `scanCount`, `templateSampleCount`, and `minutiaeCount`.

#### `verify(options)`

| Option | Type | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `username` | `string` | Conditional | — | Loads `fingerprint/prints/<username>.fpt`. Required when `template` is not supplied. |
| `template` | `FingerprintTemplate` | Conditional | — | Template loaded from a server or another source. When supplied, it is used instead of `username`. |
| `threshold` | `number` | No | `40` | Minimum BOZORTH3 score required for a match. A higher value is stricter and can reduce false matches, but may increase false rejections. |
| `timeoutSecs` | `number` | No | `60` | Maximum wait time for the finger, from `1` to `300` seconds. |

Either `username` or `template` must be provided. The result contains
`matched`, `username`, `finger`, `score`, and the applied `threshold`.

#### `identify(options?)`

| Option | Type | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `threshold` | `number` | No | `40` | Minimum BOZORTH3 score required for a match. A higher value makes matching stricter. |
| `timeoutSecs` | `number` | No | `60` | Maximum wait time for the finger, from `1` to `300` seconds. |

Identification searches all templates saved under the local
`fingerprint/prints` directory. The result contains `matched`, `username`,
`finger`, `score`, `threshold`, and `galleryCount`.

## License

This project is licensed under the
[GNU Lesser General Public License v2.1 or later](LICENSE)
(`LGPL-2.1-or-later`).

The USB driver in `src/uru4500.rs` is a port of libfprint's `uru4000` driver,
which is licensed under LGPL-2.1-or-later.

## Support

If this project is useful to you, you can support its development:

- [PayPal](https://paypal.me/alfianlensun)
- [Buy Me a Coffee](https://www.buymeacoffee.com/alfianlensun)
