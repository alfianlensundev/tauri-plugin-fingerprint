use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;
use crate::{Error, Result};

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> Result<Fingerprint<R>> {
    Ok(Fingerprint(app.clone()))
}

/// Mobile belum mendukung reader USB U.are.U 4500.
pub struct Fingerprint<R: Runtime>(AppHandle<R>);

impl<R: Runtime> Clone for Fingerprint<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<R: Runtime> Fingerprint<R> {
    pub fn check_device(&self) -> Result<DeviceStatus> {
        Err(Error::UnsupportedPlatform)
    }

    pub fn get_device_info(&self) -> Result<DeviceInfo> {
        Err(Error::UnsupportedPlatform)
    }

    pub fn capture(&self, _payload: CaptureRequest) -> Result<CaptureResponse> {
        Err(Error::UnsupportedPlatform)
    }

    pub fn capture_frames(&self, _payload: CaptureFramesRequest) -> Result<Vec<CapturedFrame>> {
        Err(Error::UnsupportedPlatform)
    }

    pub fn enroll(&self, _payload: EnrollRequest) -> Result<EnrollResponse> {
        Err(Error::UnsupportedPlatform)
    }

    pub fn verify(&self, _payload: VerifyRequest) -> Result<VerifyResponse> {
        Err(Error::UnsupportedPlatform)
    }

    pub fn identify(&self, _payload: IdentifyRequest) -> Result<IdentifyResponse> {
        Err(Error::UnsupportedPlatform)
    }
}
