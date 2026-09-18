//! Driver native DigitalPersona U.are.U 4500 (USB `05ba:000a`) memakai `nusb`.
//!
//! Ini port sinkron dari driver `uru4000` milik libfprint (LGPL-2.1+), dibuat
//! supaya reader bisa dibaca langsung tanpa libfprint — jadi jalan di macOS,
//! Linux, maupun Windows.
//!
//! Alur pemakaian:
//! ```no_run
//! # use tauri_plugin_fingerprint::uru4500::Uru4500;
//! let mut dev = Uru4500::open()?;
//! dev.init()?;                       // power-up + tunggu interrupt 0x56aa
//! dev.await_finger_on(None)?;        // tunggu jari ditempel
//! let frames = dev.capture_frames(3)?; // beberapa citra 384x290, mutunya beda-beda
//! dev.await_finger_off(None)?;
//! dev.power_off()?;
//! # Ok::<(), tauri_plugin_fingerprint::uru4500::Error>(())
//! ```

use std::fmt;
use std::time::{Duration, Instant};

use nusb::transfer::{Bulk, ControlIn, ControlOut, ControlType, In, Interrupt, Recipient};
use nusb::MaybeFuture;

pub const VID: u16 = 0x05ba;
pub const PID: u16 = 0x000a;

pub const IMAGE_WIDTH: usize = 384;
pub const IMAGE_HEIGHT: usize = 290;
/// Sensor ini 500 dpi — dipakai MINDTCT untuk skala minutiae.
pub const IMAGE_PPI: u16 = 500;

const HEADER_LEN: usize = 64;
const TRANSFER_LEN: usize = HEADER_LEN + IMAGE_WIDTH * IMAGE_HEIGHT;
const NUM_BLOCKS: usize = 15;

const USB_RQ: u8 = 0x04;
const EP_INTR: u8 = 0x81;
const EP_DATA: u8 = 0x82;
const IRQ_LENGTH: usize = 64;
const CTRL_TIMEOUT: Duration = Duration::from_secs(5);

const REG_HWSTAT: u16 = 0x07;
const REG_SCRAMBLE_DATA_INDEX: u16 = 0x33;
const REG_SCRAMBLE_DATA_KEY: u16 = 0x34;
const REG_MODE: u16 = 0x4e;
const REG_DEVICE_INFO: u16 = 0xf0;

const MODE_AWAIT_FINGER_ON: u8 = 0x10;
const MODE_AWAIT_FINGER_OFF: u8 = 0x12;
const MODE_CAPTURE: u8 = 0x20;
const MODE_OFF: u8 = 0x70;

const IRQ_SCANPWR_ON: u16 = 0x56aa;
const IRQ_FINGER_ON: u16 = 0x0101;
const IRQ_FINGER_OFF: u16 = 0x0200;
const IRQ_DEATH: u16 = 0x0800;

/// Ambang "citra ini teracak atau tidak", dari libfprint.
const ENC_THRESHOLD: i64 = 5000;

const BLOCKF_CHANGE_KEY: u8 = 0x80;
const BLOCKF_NO_KEY_UPDATE: u8 = 0x04;
const BLOCKF_ENCRYPTED: u8 = 0x02;
const BLOCKF_NOT_PRESENT: u8 = 0x01;

// ---------------------------------------------------------------- error ----

#[derive(Debug)]
pub enum Error {
    NotFound,
    Usb(nusb::Error),
    Transfer(nusb::transfer::TransferError),
    /// Device tidak merespons dalam batas waktu.
    Timeout(&'static str),
    /// Device mengirim data yang tidak masuk akal.
    Protocol(String),
    /// Interrupt "of death" — scan berikutnya dipastikan gagal.
    DeviceDeath,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound => write!(
                f,
                "U.are.U 4500 ({VID:04x}:{PID:04x}) tidak ditemukan di bus USB"
            ),
            Error::Usb(e) => write!(f, "usb: {e}"),
            Error::Transfer(e) => write!(f, "transfer: {e}"),
            Error::Timeout(what) => write!(f, "timeout menunggu {what}"),
            Error::Protocol(m) => write!(f, "protokol: {m}"),
            Error::DeviceDeath => write!(f, "device mengirim interrupt 0x0800 (scan gagal)"),
        }
    }
}

impl std::error::Error for Error {}

impl From<nusb::Error> for Error {
    fn from(e: nusb::Error) -> Self {
        Error::Usb(e)
    }
}

impl From<nusb::transfer::TransferError> for Error {
    fn from(e: nusb::transfer::TransferError) -> Self {
        Error::Transfer(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// --------------------------------------------------------------- device ----

pub struct Uru4500 {
    iface: nusb::Interface,
    intr: nusb::Endpoint<Interrupt, In>,
    data: nusb::Endpoint<Bulk, In>,
    /// Seed acak untuk descrambling citra (device mengacak dengan LFSR).
    enc_seed: u32,
}

impl Uru4500 {
    /// Cari dan buka reader pertama yang cocok.
    pub fn open() -> Result<Self> {
        let info = nusb::list_devices()
            .wait()?
            .find(|d| d.vendor_id() == VID && d.product_id() == PID)
            .ok_or(Error::NotFound)?;

        let device = info.open().wait()?;
        let iface = device.claim_interface(0).wait()?;
        let intr = iface.endpoint::<Interrupt, In>(EP_INTR)?;
        let data = iface.endpoint::<Bulk, In>(EP_DATA)?;

        Ok(Self {
            iface,
            intr,
            data,
            // Seed awal; diganti tiap capture.
            enc_seed: 0x1234_5678,
        })
    }

    // ---- register I/O: vendor control transfer, wValue = nomor register ----

    fn read_regs(&self, first_reg: u16, num_regs: u16) -> Result<Vec<u8>> {
        Ok(self
            .iface
            .control_in(
                ControlIn {
                    control_type: ControlType::Vendor,
                    recipient: Recipient::Device,
                    request: USB_RQ,
                    value: first_reg,
                    index: 0,
                    length: num_regs,
                },
                CTRL_TIMEOUT,
            )
            .wait()?)
    }

    fn read_reg(&self, reg: u16) -> Result<u8> {
        let v = self.read_regs(reg, 1)?;
        v.first()
            .copied()
            .ok_or_else(|| Error::Protocol(format!("register {reg:#x} balik kosong")))
    }

    fn write_regs(&self, first_reg: u16, values: &[u8]) -> Result<()> {
        self.iface
            .control_out(
                ControlOut {
                    control_type: ControlType::Vendor,
                    recipient: Recipient::Device,
                    request: USB_RQ,
                    value: first_reg,
                    index: 0,
                    data: values,
                },
                CTRL_TIMEOUT,
            )
            .wait()?;
        Ok(())
    }

    fn write_reg(&self, reg: u16, value: u8) -> Result<()> {
        self.write_regs(reg, &[value])
    }

    // ------------------------------- interrupt endpoint ------------------

    /// Ambil satu interrupt. `None` = timeout.
    fn next_irq(&mut self, timeout: Duration) -> Result<Option<u16>> {
        if self.intr.pending() == 0 {
            let buf = self.intr.allocate(IRQ_LENGTH);
            self.intr.submit(buf);
        }

        let Some(completion) = self.intr.wait_next_complete(timeout) else {
            return Ok(None);
        };
        completion.status?;

        let bytes: &[u8] = &completion.buffer;
        if bytes.len() < 2 {
            return Err(Error::Protocol(format!(
                "interrupt cuma {} byte",
                bytes.len()
            )));
        }
        let irq = u16::from_be_bytes([bytes[0], bytes[1]]);

        // Selalu sediakan transfer berikutnya supaya interrupt tidak terlewat.
        let buf = self.intr.allocate(IRQ_LENGTH);
        self.intr.submit(buf);

        if std::env::var_os("FP_DEBUG").is_some() {
            eprintln!("[irq] {irq:#06x}");
        }

        if irq == IRQ_DEATH {
            return Err(Error::DeviceDeath);
        }
        Ok(Some(irq))
    }

    /// Tunggu interrupt tertentu, abaikan yang lain.
    fn wait_irq(&mut self, want: u16, deadline: Duration, what: &'static str) -> Result<()> {
        let start = Instant::now();
        while start.elapsed() < deadline {
            let sisa = deadline - start.elapsed();
            match self.next_irq(sisa.min(Duration::from_millis(500)))? {
                Some(irq) if irq == want => return Ok(()),
                Some(_) => continue, // interrupt lain: abaikan
                None => continue,    // timeout kecil: coba lagi
            }
        }
        Err(Error::Timeout(what))
    }

    // ------------------------------------------- inisialisasi / power ----

    /// Power-up device sampai siap memindai.
    ///
    /// Port dari `init_run_state` libfprint: cek hwstat → (reboot power) →
    /// power down → power up → tunggu interrupt `0x56aa`.
    pub fn init(&mut self) -> Result<()> {
        for percobaan in 1..=3 {
            let mut hwstat = self.read_reg(REG_HWSTAT)?;

            // Bit 2 + bit 7 menyala = device bingung, perlu di-reboot dulu.
            if hwstat & 0x84 == 0x84 {
                hwstat = self.reboot_power(hwstat)?;
            }

            // Pastikan device dalam low power mode sebelum dinyalakan.
            if hwstat & 0x80 == 0 {
                self.write_reg(REG_HWSTAT, hwstat | 0x80)?;
                hwstat |= 0x80;
            }

            self.power_up(hwstat)?;

            match self.wait_irq(IRQ_SCANPWR_ON, Duration::from_millis(900), "scan power") {
                Ok(()) => return Ok(()),
                Err(Error::Timeout(_)) if percobaan < 3 => continue,
                Err(e) => return Err(e),
            }
        }
        Err(Error::Timeout("scan power (3x percobaan)"))
    }

    fn reboot_power(&self, mut hwstat: u8) -> Result<u8> {
        self.write_reg(REG_HWSTAT, hwstat & 0x0f)?;
        for _ in 0..100 {
            hwstat = self.read_reg(REG_HWSTAT)?;
            if hwstat & 0x01 != 0 {
                return Ok(hwstat);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Err(Error::Protocol("gagal reboot power device".into()))
    }

    fn power_up(&self, hwstat: u8) -> Result<()> {
        let target = hwstat & 0x0f;
        for _ in 0..100 {
            self.write_reg(REG_HWSTAT, target)?;
            if self.read_reg(REG_HWSTAT)? & 0x80 == 0 {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Err(Error::Protocol("device tidak mau power up".into()))
    }

    /// Versi hardware/firmware dari `REG_DEVICE_INFO`.
    pub fn version(&self) -> Result<String> {
        let r = self.read_regs(REG_DEVICE_INFO, 16)?;
        if r.len() < 12 {
            return Err(Error::Protocol("device info terlalu pendek".into()));
        }
        Ok(format!(
            "{:02x}{:02x} / {:02x}{:02x}",
            r[10], r[11], r[4], r[5]
        ))
    }

    /// Matikan sensor (mode off). Panggil setelah selesai.
    pub fn power_off(&self) -> Result<()> {
        self.write_reg(REG_MODE, MODE_OFF)
    }

    // ------------------------------------------------ deteksi jari -------

    /// Tunggu sampai jari ditempel.
    pub fn await_finger_on(&mut self, timeout: Option<Duration>) -> Result<()> {
        self.write_reg(REG_MODE, MODE_AWAIT_FINGER_ON)?;
        self.wait_irq(
            IRQ_FINGER_ON,
            timeout.unwrap_or(Duration::from_secs(30)),
            "jari ditempel",
        )
    }

    /// Tunggu sampai jari diangkat.
    pub fn await_finger_off(&mut self, timeout: Option<Duration>) -> Result<()> {
        self.write_reg(REG_MODE, MODE_AWAIT_FINGER_OFF)?;
        self.wait_irq(
            IRQ_FINGER_OFF,
            timeout.unwrap_or(Duration::from_secs(30)),
            "jari diangkat",
        )
    }

    // ---------------------------------------------------- capture --------

    /// Ambil beberapa frame berturut-turut dalam satu tempelan jari.
    ///
    /// Tiap frame sudah dinormalisasi (descramble + flip + invert) dan siap
    /// masuk MINDTCT. Ambil lebih dari satu lalu pilih yang terbaik: mutu
    /// antar-frame berbeda jauh, terutama saat jari mulai terangkat.
    ///
    /// Device terus memindai selama mode CAPTURE, jadi frame berikutnya cukup
    /// diambil dengan transfer baru tanpa menulis ulang register mode.
    pub fn capture_frames(&mut self, n: usize) -> Result<Vec<Vec<u8>>> {
        self.write_reg(REG_MODE, MODE_CAPTURE)?;
        (0..n).map(|_| self.read_frame()).collect()
    }

    /// Satu frame dari bulk endpoint: baca, buka acakan, susun, normalisasi.
    fn read_frame(&mut self) -> Result<Vec<u8>> {
        let buf = self.data.allocate(TRANSFER_LEN);
        self.data.submit(buf);

        let completion = self
            .data
            .wait_next_complete(Duration::from_secs(10))
            .ok_or(Error::Timeout("data citra"))?;
        completion.status?;

        let mut raw: Vec<u8> = completion.buffer.to_vec();
        let actual = raw.len();
        if actual < HEADER_LEN {
            return Err(Error::Protocol(format!("citra cuma {actual} byte")));
        }
        raw.resize(TRANSFER_LEN, 0);

        let num_lines = u16::from_le_bytes([raw[4], raw[5]]) as usize;
        let key_number = raw[6];
        if num_lines >= IMAGE_HEIGHT || actual < num_lines * IMAGE_WIDTH + HEADER_LEN {
            return Err(Error::Protocol(format!(
                "citra rusak: {num_lines} baris, {actual} byte"
            )));
        }

        let mut blocks: Vec<(u8, usize)> = (0..NUM_BLOCKS)
            .map(|i| (raw[16 + i * 2], raw[17 + i * 2] as usize))
            .collect();

        let noise = self.noisiness(&raw, &blocks);
        let scrambled = noise >= ENC_THRESHOLD;
        if scrambled {
            self.descramble(&mut raw, &mut blocks, key_number, num_lines)?;
        }

        if std::env::var_os("FP_DEBUG").is_some() {
            eprintln!(
                "[frame] {num_lines} baris, {actual} byte, noise={noise}, teracak={scrambled}, blok={:?}",
                blocks
                    .iter()
                    .take_while(|(_, n)| *n > 0)
                    .map(|(f, n)| format!("{f:02x}:{n}"))
                    .collect::<Vec<_>>()
            );
        }

        Ok(normalize(assemble(&raw, &blocks, num_lines)))
    }

    /// Seberapa "berisik" dua baris pertama — dipakai libfprint untuk menebak
    /// apakah citra teracak.
    fn noisiness(&self, raw: &[u8], blocks: &[(u8, usize)]) -> i64 {
        let mut rows: Vec<usize> = Vec::new();
        let mut r = 0usize;
        for &(flags, num_lines) in blocks {
            if flags & BLOCKF_NOT_PRESENT != 0 {
                continue;
            }
            for _ in 0..num_lines {
                if rows.len() < 2 {
                    rows.push(r);
                }
                r += 1;
            }
            if rows.len() >= 2 {
                break;
            }
        }
        if rows.len() < 2 {
            return 0;
        }

        let line = |row: usize| -> &[u8] {
            let off = HEADER_LEN + row * IMAGE_WIDTH;
            &raw[off..off + IMAGE_WIDTH]
        };
        let (a, b) = (line(rows[0]), line(rows[1]));

        let mean: i64 = (0..IMAGE_WIDTH)
            .map(|i| a[i] as i64 + b[i] as i64)
            .sum::<i64>()
            / IMAGE_WIDTH as i64;
        let res: i64 = (0..IMAGE_WIDTH)
            .map(|i| {
                let dev = a[i] as i64 + b[i] as i64 - mean;
                dev * dev
            })
            .sum();
        res / IMAGE_WIDTH as i64
    }

    /// Buka acakan citra: minta kunci ke device lalu XOR dengan LFSR.
    fn descramble(
        &mut self,
        raw: &mut [u8],
        blocks: &mut [(u8, usize)],
        mut key_number: u8,
        num_lines: usize,
    ) -> Result<()> {
        // Device bisa minta ganti kunci di tengah jalan; batasi putarannya.
        for _ in 0..NUM_BLOCKS + 1 {
            self.enc_seed = self
                .enc_seed
                .wrapping_mul(1_103_515_245)
                .wrapping_add(12_345);

            let seed = self.enc_seed;
            self.write_regs(
                REG_SCRAMBLE_DATA_INDEX,
                &[
                    key_number,
                    seed as u8,
                    (seed >> 8) as u8,
                    (seed >> 16) as u8,
                    (seed >> 24) as u8,
                ],
            )?;

            let k = self.read_regs(REG_SCRAMBLE_DATA_KEY, 4)?;
            if k.len() < 4 {
                return Err(Error::Protocol("kunci scramble < 4 byte".into()));
            }
            let mut key = u32::from_le_bytes([k[0], k[1], k[2], k[3]]) ^ seed;

            let mut lines_done = 0usize;
            let mut ganti_kunci = false;

            for (idx, block) in blocks.iter_mut().enumerate() {
                if lines_done >= num_lines {
                    break;
                }
                let (flags, num) = *block;
                if num == 0 {
                    break;
                }
                if lines_done + num > IMAGE_HEIGHT {
                    return Err(Error::Protocol(format!(
                        "blok {idx} ({num} baris) melewati batas citra"
                    )));
                }

                if flags & BLOCKF_CHANGE_KEY != 0 {
                    block.0 &= !BLOCKF_CHANGE_KEY;
                    key_number = key_number.wrapping_add(1);
                    ganti_kunci = true;
                    break;
                }

                let off = HEADER_LEN + lines_done * IMAGE_WIDTH;
                let len = IMAGE_WIDTH * num;
                match flags & (BLOCKF_NO_KEY_UPDATE | BLOCKF_ENCRYPTED) {
                    BLOCKF_ENCRYPTED => key = do_decode(&mut raw[off..off + len], key),
                    0 => {
                        for _ in 0..len {
                            key = update_key(key);
                        }
                    }
                    _ => {}
                }

                if flags & BLOCKF_NOT_PRESENT == 0 {
                    lines_done += num;
                }
            }

            if !ganti_kunci {
                return Ok(());
            }
        }
        Err(Error::Protocol("device terus minta ganti kunci".into()))
    }
}

// ------------------------------------------------------------ helpers ----

/// LFSR device: tap di bit 1 3 4 7 11 13 20 23 26 29 32.
fn update_key(key: u32) -> u32 {
    let mut bit = key & 0x9248_144d;
    bit ^= bit << 16;
    bit ^= bit << 8;
    bit ^= bit << 4;
    bit ^= bit << 2;
    bit ^= bit << 1;
    (bit & 0x8000_0000) | (key >> 1)
}

/// XOR-decode satu blok; mengembalikan kunci untuk blok berikutnya.
fn do_decode(data: &mut [u8], mut key: u32) -> u32 {
    let n = data.len();
    if n == 0 {
        return key;
    }
    for i in 0..n - 1 {
        let xorbyte = (((key >> 4) & 1) as u8)
            | ((((key >> 8) & 1) as u8) << 1)
            | ((((key >> 11) & 1) as u8) << 2)
            | ((((key >> 14) & 1) as u8) << 3)
            | ((((key >> 18) & 1) as u8) << 4)
            | ((((key >> 21) & 1) as u8) << 5)
            | ((((key >> 24) & 1) as u8) << 6)
            | ((((key >> 29) & 1) as u8) << 7);
        key = update_key(key);
        data[i] = data[i + 1] ^ xorbyte;
    }
    data[n - 1] = 0;
    update_key(key)
}

/// Susun baris-baris blok jadi satu citra utuh.
fn assemble(raw: &[u8], blocks: &[(u8, usize)], num_lines: usize) -> Vec<u8> {
    let mut out = vec![0u8; IMAGE_WIDTH * IMAGE_HEIGHT];
    let (mut src_line, mut dst) = (0usize, 0usize);

    for &(flags, num) in blocks {
        if num == 0 || src_line >= num_lines {
            break;
        }
        if src_line + num > IMAGE_HEIGHT || dst + num * IMAGE_WIDTH > out.len() {
            break;
        }
        let off = HEADER_LEN + src_line * IMAGE_WIDTH;
        out[dst..dst + num * IMAGE_WIDTH].copy_from_slice(&raw[off..off + num * IMAGE_WIDTH]);
        if flags & BLOCKF_NOT_PRESENT == 0 {
            src_line += num;
        }
        dst += num * IMAGE_WIDTH;
    }
    out
}

/// U.are.U 4500 mengirim citra terbalik horizontal+vertikal dan warnanya
/// terbalik (sama seperti flag `FPI_IMAGE_*` di libfprint untuk DP_URU4000B).
fn normalize(mut img: Vec<u8>) -> Vec<u8> {
    for row in img.chunks_mut(IMAGE_WIDTH) {
        row.reverse(); // h-flip
    }
    let mut flipped = Vec::with_capacity(img.len());
    for row in img.chunks(IMAGE_WIDTH).rev() {
        flipped.extend_from_slice(row); // v-flip
    }
    for p in &mut flipped {
        *p = 255 - *p; // warna terbalik
    }
    flipped
}

/// Simpan citra grayscale sebagai PGM (P5).
pub fn save_pgm(img: &[u8], path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    write!(f, "P5\n{IMAGE_WIDTH} {IMAGE_HEIGHT}\n255\n")?;
    f.write_all(img)
}
