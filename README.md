# tauri-plugin-fingerprint

Plugin Tauri 2 untuk membaca, mendaftarkan, dan mencocokkan sidik jari dari
DigitalPersona **U.are.U 4500** (`05ba:000a`) tanpa libfprint.

Driver memakai `nusb`, sedangkan ekstraksi minutiae dan pencocokan memakai
MINDTCT + BOZORTH3 dari `fprint-pipeline`. Implementasi desktop mendukung
macOS, Linux, dan Windows.

## Instalasi

Tambahkan crate dari crates.io melalui folder `src-tauri` aplikasi:

```bash
cd src-tauri
cargo add tauri-plugin-fingerprint
```

Atau tambahkan secara manual ke `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri-plugin-fingerprint = "0.1"
```

Daftarkan plugin pada builder Tauri:

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_fingerprint::init())
```

Tambahkan package JavaScript:

```bash
npm install tauri-plugin-fingerprint
```

Kemudian beri capability `fingerprint:default`:

```json
{
  "permissions": ["core:default", "fingerprint:default"]
}
```

## Enroll dan simpan template JSON di server

`template` adalah objek JSON yang dipakai untuk verifikasi. `images` hanya
preview PNG dalam bentuk data URL Base64 dan tidak diperlukan oleh matcher.

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
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({
    template: enrollment.template,
  }),
})

document.querySelector('img')!.src = enrollment.images[0]
```

Bentuk data utamanya:

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

Simpan seluruh objek `template` tanpa mengubah nilai koordinat atau sudutnya.
Data tersebut merupakan data biometrik sensitif, jadi enkripsi saat transit dan
saat disimpan serta batasi aksesnya.

## Verify memakai template dari server

```ts
import { verify } from 'tauri-plugin-fingerprint'

const response = await fetch('/api/fingerprints/alfian')
const { template } = await response.json()

const result = await verify({ template })

console.log(result.matched, result.score, result.threshold)
```

Jika `saveLocally` tidak diubah atau bernilai `true`, verify juga dapat memakai
template lokal:

```ts
const result = await verify({ username: 'alfian' })
```

Template lokal disimpan di folder data aplikasi Tauri, pada
`fingerprint/prints/<username>.fpt`.

## API

- `checkDevice()` — cek apakah reader dapat dibuka.
- `getDeviceInfo()` — baca versi hardware/firmware dan informasi citra.
- `capture(options?)` — scan satu jari dan simpan citra PGM untuk diagnostik.
- `captureFrames(options?)` — simpan beberapa frame diagnostik.
- `enroll(options)` — scan beberapa kali dan hasilkan template JSON.
- `verify(options)` — cocokkan scan dengan template JSON dari server atau file lokal.
- `identify(options?)` — cocokkan scan dengan seluruh template lokal.

Operasi scan dijalankan pada blocking thread dan akses reader diserialkan, jadi
UI Tauri tetap responsif serta dua operasi tidak memakai reader bersamaan.

## Catatan platform

- macOS: reader vendor-class dapat dipakai tanpa `sudo`.
- Linux: tambahkan udev rule untuk `05ba:000a` dan pastikan `fprintd` tidak
  sedang memegang device.
- Windows: reader harus memakai driver WinUSB agar dapat dibuka oleh `nusb`.
- Android/iOS: belum didukung karena implementasi ini memakai reader USB desktop.

Driver USB pada `src/uru4500.rs` merupakan port dari driver `uru4000` milik
libfprint (LGPL-2.1+).

## Lisensi

Proyek ini didistribusikan dengan lisensi
[GNU Lesser General Public License v2.1 or later](LICENSE)
(`LGPL-2.1-or-later`).

## Membuat GitHub Release

Pastikan working tree bersih, `gh` sudah login dengan akses tulis, dan remote
`origin` mengarah ke repository GitHub. Version dan changelog dapat diberikan
langsung sebagai argumen:

```bash
./scripts/release.sh 0.2.0 "Tambah enroll JSON dan perbaikan build Linux"
```

Atau jalankan tanpa argumen untuk mengisi version dan changelog melalui prompt.
Changelog dapat terdiri dari beberapa baris dan diakhiri dengan satu baris
kosong:

```bash
./scripts/release.sh
```

Skrip hanya menangani proses release Git: memperbarui version Cargo dan npm,
membuat commit release, membuat tag `v<version>`, mendorong commit dan tag ke
`origin`, lalu membuat GitHub Release dengan release notes otomatis. Seluruh
test, build, dan publish dijalankan oleh GitHub Actions.

Saat GitHub Release dipublikasikan, workflow `.github/workflows/publish.yml`
akan memublikasikan crate ke crates.io dan package ke npm. Tambahkan repository
secrets berikut sebelum release pertama:

- `CARGO_REGISTRY_TOKEN` — API token dari crates.io.
- `NPM_TOKEN` — access token npm yang memiliki izin publish package.

Workflow memvalidasi bahwa version pada tag, `Cargo.toml`, dan `package.json`
sama. GitHub prerelease dipublikasikan ke npm menggunakan dist-tag `next`;
release biasa menggunakan dist-tag `latest`.
