use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::{FingerprintExt, Result};

async fn run_blocking<T, F>(operation: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|error| crate::Error::Task(error.to_string()))?
}

#[command]
pub(crate) async fn check_device<R: Runtime>(app: AppHandle<R>) -> Result<DeviceStatus> {
    let fingerprint = app.fingerprint().clone();
    run_blocking(move || fingerprint.check_device()).await
}

#[command]
pub(crate) async fn get_device_info<R: Runtime>(app: AppHandle<R>) -> Result<DeviceInfo> {
    let fingerprint = app.fingerprint().clone();
    run_blocking(move || fingerprint.get_device_info()).await
}

#[command]
pub(crate) async fn capture<R: Runtime>(
    app: AppHandle<R>,
    payload: CaptureRequest,
) -> Result<CaptureResponse> {
    let fingerprint = app.fingerprint().clone();
    run_blocking(move || fingerprint.capture(payload)).await
}

#[command]
pub(crate) async fn capture_frames<R: Runtime>(
    app: AppHandle<R>,
    payload: CaptureFramesRequest,
) -> Result<Vec<CapturedFrame>> {
    let fingerprint = app.fingerprint().clone();
    run_blocking(move || fingerprint.capture_frames(payload)).await
}

#[command]
pub(crate) async fn enroll<R: Runtime>(
    app: AppHandle<R>,
    payload: EnrollRequest,
) -> Result<EnrollResponse> {
    let fingerprint = app.fingerprint().clone();
    run_blocking(move || fingerprint.enroll(payload)).await
}

#[command]
pub(crate) async fn verify<R: Runtime>(
    app: AppHandle<R>,
    payload: VerifyRequest,
) -> Result<VerifyResponse> {
    let fingerprint = app.fingerprint().clone();
    run_blocking(move || fingerprint.verify(payload)).await
}

#[command]
pub(crate) async fn identify<R: Runtime>(
    app: AppHandle<R>,
    payload: IdentifyRequest,
) -> Result<IdentifyResponse> {
    let fingerprint = app.fingerprint().clone();
    run_blocking(move || fingerprint.identify(payload)).await
}
