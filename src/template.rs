//! Template sidik jari: ekstraksi minutiae, pencocokan, dan penyimpanan.
//!
//! Citra dari [`crate::uru4500`] diubah jadi minutiae oleh MINDTCT, lalu
//! dicocokkan dengan BOZORTH3 — keduanya port murni-Rust dari NIST NBIS,
//! algoritma yang sama dengan yang dipakai libfprint.

use std::fmt;
use std::path::Path;

use fprint_pipeline::{
    fprint_core::Minutia, fprint_core::Template, nbis_identify, nbis_match_score, nbis_verify,
    template_from_images, GrayImage,
};

use crate::uru4500::{IMAGE_HEIGHT, IMAGE_PPI, IMAGE_WIDTH};
use crate::{FingerprintTemplate, TemplateMinutia};

/// Ambang skor BOZORTH3 untuk menerima kecocokan.
///
/// 40 adalah nilai yang dipakai NBIS/libfprint sebagai titik seimbang antara
/// false accept dan false reject. Naikkan kalau butuh lebih ketat.
pub const DEFAULT_THRESHOLD: u32 = 40;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    /// Citra tidak menghasilkan minutiae (jari kurang menempel / terlalu kering).
    NoMinutiae,
    /// File template rusak.
    BadFormat(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::NoMinutiae => write!(
                f,
                "tidak ada minutiae yang terdeteksi — tempelkan jari lebih penuh"
            ),
            Error::BadFormat(m) => write!(f, "format template: {m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Template satu jari beserta metadatanya.
#[derive(Debug, Clone)]
pub struct Enrollment {
    pub username: String,
    pub finger: String,
    pub template: Template,
}

impl Enrollment {
    /// Bangun template dari beberapa citra hasil scan (makin banyak makin
    /// toleran terhadap posisi jari yang bergeser).
    pub fn from_images(username: &str, finger: &str, images: &[Vec<u8>]) -> Result<Self> {
        let grays: Vec<GrayImage<'_>> = images
            .iter()
            .filter_map(|img| GrayImage::new(img, IMAGE_WIDTH, IMAGE_HEIGHT, IMAGE_PPI).ok())
            .collect();

        let template = template_from_images(&grays);
        if minutiae_count(&template) == 0 {
            return Err(Error::NoMinutiae);
        }

        Ok(Self {
            username: username.to_string(),
            finger: finger.to_string(),
            template,
        })
    }

    pub fn minutiae_count(&self) -> usize {
        minutiae_count(&self.template)
    }
}

/// Pilih frame terbaik dari satu tempelan jari.
///
/// Device memindai terus-menerus selama jari menempel, dan kualitas antar-frame
/// bisa jauh berbeda (frame terakhir sering sudah "kosong" karena jari mulai
/// terangkat). Yang dipakai adalah frame dengan minutiae terbanyak.
///
/// Mengembalikan `(indeks, jumlah_minutiae)` dari frame terpilih.
pub fn best_frame(frames: &[Vec<u8>]) -> Option<(usize, usize)> {
    frames
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let n = template_from_image(f)
                .map(|t| minutiae_count(&t))
                .unwrap_or(0);
            (i, n)
        })
        .max_by_key(|&(_, n)| n)
        .filter(|&(_, n)| n > 0)
}

/// Minutiae minimum agar sebuah frame layak dipakai mendaftar.
pub const MIN_MINUTIAE: usize = 20;

/// Semua frame yang cukup bagus dari satu tempelan, terbanyak dulu.
///
/// Dipakai saat enroll: makin banyak sampel yang tersimpan, makin besar
/// peluang scan berikutnya menemukan pasangan yang cocok (skor diambil dari
/// pasangan terbaik).
pub fn good_frames(frames: &[Vec<u8>]) -> Vec<(usize, usize)> {
    let mut ok: Vec<(usize, usize)> = frames
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let n = template_from_image(f)
                .map(|t| minutiae_count(&t))
                .unwrap_or(0);
            (i, n)
        })
        .filter(|&(_, n)| n >= MIN_MINUTIAE)
        .collect();
    ok.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    ok
}

/// Ubah satu citra jadi template untuk dicocokkan.
pub fn template_from_image(image: &[u8]) -> Result<Template> {
    let gray = GrayImage::new(image, IMAGE_WIDTH, IMAGE_HEIGHT, IMAGE_PPI)
        .map_err(|e| Error::BadFormat(format!("{e:?}")))?;
    let template = template_from_images(&[gray]);
    if minutiae_count(&template) == 0 {
        return Err(Error::NoMinutiae);
    }
    Ok(template)
}

/// Cocokkan 1:1. `true` kalau skornya melewati ambang.
pub fn verify(enrolled: &Template, scanned: &Template, threshold: u32) -> bool {
    nbis_verify(enrolled, scanned, threshold)
}

/// Skor mentah BOZORTH3 (berguna untuk menyetel ambang).
pub fn score(enrolled: &Template, scanned: &Template) -> Option<u32> {
    nbis_match_score(enrolled, scanned).score()
}

/// Cocokkan 1:N; mengembalikan indeks di `gallery` yang paling cocok.
pub fn identify(scanned: &Template, gallery: &[Template], threshold: u32) -> Option<usize> {
    nbis_identify(scanned, gallery, threshold)
}

pub fn minutiae_count(t: &Template) -> usize {
    match t {
        Template::Nbis(samples) => samples.iter().map(Vec::len).sum(),
        _ => 0,
    }
}

// ------------------------------------------------- simpan / muat ---------

/// Format file: teks sederhana, satu minutia per baris (`x y theta`),
/// dipisah per sampel. Mudah dibaca dan tidak mengikat ke library apa pun.
pub fn save(enrollment: &Enrollment, path: impl AsRef<Path>) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, serialize(enrollment)?)?;
    Ok(())
}

pub fn load(path: impl AsRef<Path>) -> Result<Enrollment> {
    let text = std::fs::read_to_string(path)?;
    deserialize(&text)
}

/// Ubah enrollment ke format teks FPT1 agar dapat disimpan di database/server.
pub fn serialize(enrollment: &Enrollment) -> Result<String> {
    use std::fmt::Write as _;

    let Template::Nbis(samples) = &enrollment.template else {
        return Err(Error::BadFormat("template bukan NBIS".into()));
    };

    let mut out = String::from("FPT1\n");
    let _ = writeln!(out, "user={}", enrollment.username);
    let _ = writeln!(out, "finger={}", enrollment.finger);
    for sample in samples {
        out.push_str("sample\n");
        for minutia in sample {
            let _ = writeln!(out, "{} {} {}", minutia.x, minutia.y, minutia.theta);
        }
    }

    Ok(out)
}

/// Muat enrollment langsung dari string FPT1 yang diterima dari server.
pub fn deserialize(text: &str) -> Result<Enrollment> {
    let mut lines = text.lines();

    if lines.next() != Some("FPT1") {
        return Err(Error::BadFormat("header bukan FPT1".into()));
    }

    let (mut username, mut finger) = (String::new(), String::new());
    let mut samples: Vec<Vec<Minutia>> = Vec::new();

    for line in lines {
        if let Some(v) = line.strip_prefix("user=") {
            username = v.to_string();
        } else if let Some(v) = line.strip_prefix("finger=") {
            finger = v.to_string();
        } else if line == "sample" {
            samples.push(Vec::new());
        } else if !line.trim().is_empty() {
            let nums: Vec<i32> = line
                .split_whitespace()
                .filter_map(|n| n.parse().ok())
                .collect();
            let [x, y, theta] = nums[..] else {
                return Err(Error::BadFormat(format!("baris minutia rusak: {line:?}")));
            };
            samples
                .last_mut()
                .ok_or_else(|| Error::BadFormat("minutia sebelum 'sample'".into()))?
                .push(Minutia::from_xyt(x, y, theta));
        }
    }

    Ok(Enrollment {
        username,
        finger,
        template: Template::Nbis(samples),
    })
}

/// Ubah enrollment menjadi objek yang dapat langsung diserialisasi sebagai JSON.
pub fn to_json_template(enrollment: &Enrollment) -> Result<FingerprintTemplate> {
    let Template::Nbis(samples) = &enrollment.template else {
        return Err(Error::BadFormat("template bukan NBIS".into()));
    };

    Ok(FingerprintTemplate {
        version: 1,
        username: enrollment.username.clone(),
        finger: enrollment.finger.clone(),
        samples: samples
            .iter()
            .map(|sample| {
                sample
                    .iter()
                    .map(|minutia| TemplateMinutia {
                        x: minutia.x,
                        y: minutia.y,
                        theta: minutia.theta,
                    })
                    .collect()
            })
            .collect(),
    })
}

/// Bangun enrollment dari objek JSON yang sebelumnya dikembalikan saat enroll.
pub fn from_json_template(template: &FingerprintTemplate) -> Result<Enrollment> {
    if template.version != 1 {
        return Err(Error::BadFormat(format!(
            "versi template {} tidak didukung",
            template.version
        )));
    }

    let samples: Vec<Vec<Minutia>> = template
        .samples
        .iter()
        .map(|sample| {
            sample
                .iter()
                .map(|minutia| Minutia::from_xyt(minutia.x, minutia.y, minutia.theta))
                .collect()
        })
        .collect();

    let enrollment = Enrollment {
        username: template.username.clone(),
        finger: template.finger.clone(),
        template: Template::Nbis(samples),
    };

    if enrollment.minutiae_count() == 0 {
        return Err(Error::BadFormat(
            "template JSON tidak memiliki minutiae".into(),
        ));
    }

    Ok(enrollment)
}

/// Muat semua template `*.fpt` dalam sebuah direktori.
pub fn load_dir(dir: impl AsRef<Path>) -> Result<Vec<Enrollment>> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(out);
    };
    for entry in entries {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("fpt") {
            out.push(load(&path)?);
        }
    }
    out.sort_by(|a, b| a.username.cmp(&b.username));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_template_round_trip() {
        let enrollment = Enrollment {
            username: "alfian".to_string(),
            finger: "right-index".to_string(),
            template: Template::Nbis(vec![vec![
                Minutia::from_xyt(10, 20, 30),
                Minutia::from_xyt(40, 50, 60),
            ]]),
        };

        let json_template = to_json_template(&enrollment).unwrap();
        let restored = from_json_template(&json_template).unwrap();

        assert_eq!(restored.username, enrollment.username);
        assert_eq!(restored.finger, enrollment.finger);
        assert_eq!(restored.minutiae_count(), enrollment.minutiae_count());
        assert_eq!(json_template.version, 1);
        assert_eq!(json_template.samples[0][0].x, 10);
    }

    #[test]
    fn json_template_rejects_unknown_version() {
        let json_template = FingerprintTemplate {
            version: 2,
            username: "alfian".to_string(),
            finger: "right-index".to_string(),
            samples: vec![vec![TemplateMinutia {
                x: 10,
                y: 20,
                theta: 30,
            }]],
        };

        assert!(from_json_template(&json_template).is_err());
    }
}
