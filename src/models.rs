use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    pub detected: bool,
    pub model: String,
    pub vendor_id: String,
    pub product_id: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub model: String,
    pub hardware_firmware_version: String,
    pub image_width: usize,
    pub image_height: usize,
    pub image_ppi: u16,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequest {
    pub output_path: Option<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResponse {
    pub path: String,
    pub minutiae_count: usize,
    pub image_width: usize,
    pub image_height: usize,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFramesRequest {
    pub count: Option<usize>,
    pub output_prefix: Option<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedFrame {
    pub path: String,
    pub minutiae_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollRequest {
    pub username: String,
    pub finger: String,
    pub samples: Option<usize>,
    pub timeout_secs: Option<u64>,
    pub include_images: Option<bool>,
    pub save_locally: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollResponse {
    pub username: String,
    pub finger: String,
    pub path: Option<String>,
    pub template: FingerprintTemplate,
    pub images: Vec<String>,
    pub scan_count: usize,
    pub template_sample_count: usize,
    pub minutiae_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyRequest {
    pub username: Option<String>,
    pub template: Option<FingerprintTemplate>,
    pub threshold: Option<u32>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintTemplate {
    pub version: u8,
    pub username: String,
    pub finger: String,
    pub samples: Vec<Vec<TemplateMinutia>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateMinutia {
    pub x: i32,
    pub y: i32,
    pub theta: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResponse {
    pub matched: bool,
    pub username: String,
    pub finger: String,
    pub score: u32,
    pub threshold: u32,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentifyRequest {
    pub threshold: Option<u32>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentifyResponse {
    pub matched: bool,
    pub username: Option<String>,
    pub finger: Option<String>,
    pub score: Option<u32>,
    pub threshold: u32,
    pub gallery_count: usize,
}
