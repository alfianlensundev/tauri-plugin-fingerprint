use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Manager, Runtime};

use crate::models::*;
use crate::template::{self, Enrollment, DEFAULT_THRESHOLD};
use crate::uru4500::{save_pgm, Uru4500, IMAGE_HEIGHT, IMAGE_PPI, IMAGE_WIDTH, PID, VID};
use crate::{Error, Result};

const DEFAULT_WAIT_SECS: u64 = 60;
const MAX_WAIT_SECS: u64 = 300;
const DEFAULT_ENROLL_SAMPLES: usize = 4;
const MAX_ENROLL_SAMPLES: usize = 10;
const FRAMES_PER_TOUCH: usize = 3;
const MAX_DIAGNOSTIC_FRAMES: usize = 20;

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> Result<Fingerprint<R>> {
    Ok(Fingerprint {
        app: app.clone(),
        device_lock: Arc::new(Mutex::new(())),
    })
}

/// Access to the fingerprint APIs.
pub struct Fingerprint<R: Runtime> {
    app: AppHandle<R>,
    device_lock: Arc<Mutex<()>>,
}

impl<R: Runtime> Clone for Fingerprint<R> {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
            device_lock: Arc::clone(&self.device_lock),
        }
    }
}

impl<R: Runtime> Fingerprint<R> {
    pub fn check_device(&self) -> Result<DeviceStatus> {
        let _guard = self.lock_device()?;

        let (detected, message) = match Uru4500::open() {
            Ok(_) => (true, None),
            Err(error) => (false, Some(error.to_string())),
        };

        Ok(DeviceStatus {
            detected,
            model: "DigitalPersona U.are.U 4500".to_string(),
            vendor_id: format!("{VID:04x}"),
            product_id: format!("{PID:04x}"),
            message,
        })
    }

    pub fn get_device_info(&self) -> Result<DeviceInfo> {
        self.with_ready_device(|device| {
            Ok(DeviceInfo {
                model: "DigitalPersona U.are.U 4500".to_string(),
                hardware_firmware_version: device.version()?,
                image_width: IMAGE_WIDTH,
                image_height: IMAGE_HEIGHT,
                image_ppi: IMAGE_PPI,
            })
        })
    }

    pub fn capture(&self, payload: CaptureRequest) -> Result<CaptureResponse> {
        let timeout = wait_duration(payload.timeout_secs)?;
        let (image, minutiae_count) =
            self.with_ready_device(|device| scan_once(device, timeout))?;
        let path = match payload.output_path {
            Some(path) => PathBuf::from(path),
            None => self
                .storage_dir()?
                .join("captures")
                .join(format!("capture-{}.pgm", timestamp()?)),
        };

        create_parent_dir(&path)?;
        save_pgm(&image, &path)?;

        Ok(CaptureResponse {
            path: path_string(&path),
            minutiae_count,
            image_width: IMAGE_WIDTH,
            image_height: IMAGE_HEIGHT,
        })
    }

    pub fn capture_frames(&self, payload: CaptureFramesRequest) -> Result<Vec<CapturedFrame>> {
        let count = payload.count.unwrap_or(5);
        if !(1..=MAX_DIAGNOSTIC_FRAMES).contains(&count) {
            return Err(Error::InvalidInput(format!(
                "count harus antara 1 dan {MAX_DIAGNOSTIC_FRAMES}"
            )));
        }

        let timeout = wait_duration(payload.timeout_secs)?;
        let frames = self.with_ready_device(|device| {
            device.await_finger_on(Some(timeout))?;
            let frames = device.capture_frames(count)?;
            device.await_finger_off(Some(timeout))?;
            Ok(frames)
        })?;

        let prefix = match payload.output_prefix {
            Some(prefix) => PathBuf::from(prefix),
            None => self
                .storage_dir()?
                .join("frames")
                .join(format!("frame-{}-", timestamp()?)),
        };

        create_parent_dir(&prefix)?;
        let mut captured = Vec::with_capacity(frames.len());
        for (index, image) in frames.iter().enumerate() {
            let path = PathBuf::from(format!("{}{index}.pgm", prefix.to_string_lossy()));
            save_pgm(image, &path)?;
            let minutiae_count = template::template_from_image(image)
                .map(|template| template::minutiae_count(&template))
                .unwrap_or(0);
            captured.push(CapturedFrame {
                path: path_string(&path),
                minutiae_count,
            });
        }

        Ok(captured)
    }

    pub fn enroll(&self, payload: EnrollRequest) -> Result<EnrollResponse> {
        validate_username(&payload.username)?;
        if payload.finger.trim().is_empty() {
            return Err(Error::InvalidInput("finger tidak boleh kosong".to_string()));
        }

        let scan_count = payload.samples.unwrap_or(DEFAULT_ENROLL_SAMPLES);
        if !(1..=MAX_ENROLL_SAMPLES).contains(&scan_count) {
            return Err(Error::InvalidInput(format!(
                "samples harus antara 1 dan {MAX_ENROLL_SAMPLES}"
            )));
        }

        let timeout = wait_duration(payload.timeout_secs)?;
        let (images, preview_images) = self.with_ready_device(|device| {
            let mut images = Vec::new();
            let mut preview_images = Vec::with_capacity(scan_count);

            for _ in 0..scan_count {
                device.await_finger_on(Some(timeout))?;
                let frames = device.capture_frames(FRAMES_PER_TOUCH)?;
                device.await_finger_off(Some(timeout))?;

                let (best_index, _) = template::best_frame(&frames).ok_or_else(|| {
                    Error::Fingerprint(
                        "tidak ada minutiae — tempelkan jari lebih penuh".to_string(),
                    )
                })?;
                preview_images.push(frames[best_index].clone());

                let good_frames = template::good_frames(&frames);
                if good_frames.is_empty() {
                    images.push(frames[best_index].clone());
                } else {
                    images.extend(good_frames.iter().map(|&(index, _)| frames[index].clone()));
                }
            }

            Ok((images, preview_images))
        })?;

        let enrollment = Enrollment::from_images(&payload.username, &payload.finger, &images)?;
        let json_template = template::to_json_template(&enrollment)?;
        let path = if payload.save_locally.unwrap_or(true) {
            let path = self.template_path(&payload.username)?;
            template::save(&enrollment, &path)?;
            Some(path)
        } else {
            None
        };
        let encoded_images = if payload.include_images.unwrap_or(false) {
            preview_images
                .iter()
                .map(|image| image_data_url(image))
                .collect::<Result<Vec<_>>>()?
        } else {
            Vec::new()
        };
        let minutiae_count = enrollment.minutiae_count();

        Ok(EnrollResponse {
            username: enrollment.username,
            finger: enrollment.finger,
            path: path.as_deref().map(path_string),
            template: json_template,
            images: encoded_images,
            scan_count,
            template_sample_count: images.len(),
            minutiae_count,
        })
    }

    pub fn verify(&self, payload: VerifyRequest) -> Result<VerifyResponse> {
        let threshold = payload.threshold.unwrap_or(DEFAULT_THRESHOLD);
        let enrolled = match payload.template.as_ref() {
            Some(template) => template::from_json_template(template)?,
            None => {
                let username = payload.username.as_deref().ok_or_else(|| {
                    Error::InvalidInput("username atau template JSON harus diberikan".to_string())
                })?;
                validate_username(username)?;
                template::load(self.template_path(username)?)?
            }
        };
        let timeout = wait_duration(payload.timeout_secs)?;
        let (image, _) = self.with_ready_device(|device| scan_once(device, timeout))?;
        let scanned = template::template_from_image(&image)?;
        let score = template::score(&enrolled.template, &scanned).unwrap_or(0);

        Ok(VerifyResponse {
            matched: template::verify(&enrolled.template, &scanned, threshold),
            username: enrolled.username,
            finger: enrolled.finger,
            score,
            threshold,
        })
    }

    pub fn identify(&self, payload: IdentifyRequest) -> Result<IdentifyResponse> {
        let threshold = payload.threshold.unwrap_or(DEFAULT_THRESHOLD);
        let gallery = template::load_dir(self.prints_dir()?)?;
        let gallery_count = gallery.len();

        if gallery.is_empty() {
            return Ok(IdentifyResponse {
                matched: false,
                username: None,
                finger: None,
                score: None,
                threshold,
                gallery_count,
            });
        }

        let timeout = wait_duration(payload.timeout_secs)?;
        let (image, _) = self.with_ready_device(|device| scan_once(device, timeout))?;
        let scanned = template::template_from_image(&image)?;
        let templates: Vec<_> = gallery
            .iter()
            .map(|enrollment| enrollment.template.clone())
            .collect();

        let Some(index) = template::identify(&scanned, &templates, threshold) else {
            return Ok(IdentifyResponse {
                matched: false,
                username: None,
                finger: None,
                score: None,
                threshold,
                gallery_count,
            });
        };

        Ok(IdentifyResponse {
            matched: true,
            username: Some(gallery[index].username.clone()),
            finger: Some(gallery[index].finger.clone()),
            score: template::score(&templates[index], &scanned),
            threshold,
            gallery_count,
        })
    }

    fn with_ready_device<T>(&self, operation: impl FnOnce(&mut Uru4500) -> Result<T>) -> Result<T> {
        let _guard = self.lock_device()?;
        let mut device = Uru4500::open()?;

        if let Err(error) = device.init() {
            let _ = device.power_off();
            return Err(error.into());
        }

        let operation_result = operation(&mut device);
        let power_off_result = device.power_off().map_err(Error::from);

        match (operation_result, power_off_result) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Ok(value), Ok(())) => Ok(value),
        }
    }

    fn lock_device(&self) -> Result<std::sync::MutexGuard<'_, ()>> {
        self.device_lock
            .lock()
            .map_err(|_| Error::Fingerprint("kunci device fingerprint rusak".to_string()))
    }

    fn storage_dir(&self) -> Result<PathBuf> {
        Ok(self.app.path().app_data_dir()?.join("fingerprint"))
    }

    fn prints_dir(&self) -> Result<PathBuf> {
        Ok(self.storage_dir()?.join("prints"))
    }

    fn template_path(&self, username: &str) -> Result<PathBuf> {
        validate_username(username)?;
        Ok(self.prints_dir()?.join(format!("{username}.fpt")))
    }
}

fn scan_once(device: &mut Uru4500, timeout: Duration) -> Result<(Vec<u8>, usize)> {
    device.await_finger_on(Some(timeout))?;
    let frames = device.capture_frames(FRAMES_PER_TOUCH)?;
    device.await_finger_off(Some(timeout))?;

    let (index, minutiae_count) = template::best_frame(&frames).ok_or_else(|| {
        Error::Fingerprint("tidak ada minutiae — tempelkan jari lebih penuh".to_string())
    })?;

    Ok((frames[index].clone(), minutiae_count))
}

fn wait_duration(seconds: Option<u64>) -> Result<Duration> {
    let seconds = seconds.unwrap_or(DEFAULT_WAIT_SECS);
    if !(1..=MAX_WAIT_SECS).contains(&seconds) {
        return Err(Error::InvalidInput(format!(
            "timeoutSecs harus antara 1 dan {MAX_WAIT_SECS}"
        )));
    }
    Ok(Duration::from_secs(seconds))
}

fn validate_username(username: &str) -> Result<()> {
    if username.is_empty() {
        return Err(Error::InvalidInput(
            "username tidak boleh kosong".to_string(),
        ));
    }
    if username.len() > 128 {
        return Err(Error::InvalidInput(
            "username maksimal 128 karakter".to_string(),
        ));
    }
    if !username
        .chars()
        .all(|character| character.is_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(Error::InvalidInput(
            "username hanya boleh berisi huruf, angka, '-' dan '_'".to_string(),
        ));
    }
    Ok(())
}

fn create_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

fn timestamp() -> Result<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| Error::Fingerprint(error.to_string()))
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn image_data_url(image: &[u8]) -> Result<String> {
    let mut png_data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_data, IMAGE_WIDTH as u32, IMAGE_HEIGHT as u32);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| Error::Fingerprint(format!("gagal membuat PNG: {error}")))?;
        writer
            .write_image_data(image)
            .map_err(|error| Error::Fingerprint(format!("gagal menulis PNG: {error}")))?;
    }

    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(png_data)
    ))
}
