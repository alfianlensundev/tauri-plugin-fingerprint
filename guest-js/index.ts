import { invoke } from '@tauri-apps/api/core'

export interface DeviceStatus {
  detected: boolean
  model: string
  vendorId: string
  productId: string
  message: string | null
}

export interface DeviceInfo {
  model: string
  hardwareFirmwareVersion: string
  imageWidth: number
  imageHeight: number
  imagePpi: number
}

export interface CaptureOptions {
  outputPath?: string
  timeoutSecs?: number
}

export interface CaptureResult {
  path: string
  minutiaeCount: number
  imageWidth: number
  imageHeight: number
}

export interface CaptureFramesOptions {
  count?: number
  outputPrefix?: string
  timeoutSecs?: number
}

export interface CapturedFrame {
  path: string
  minutiaeCount: number
}

export interface EnrollOptions {
  username: string
  finger: string
  samples?: number
  timeoutSecs?: number
  includeImages?: boolean
  saveLocally?: boolean
}

export interface EnrollResult {
  username: string
  finger: string
  path: string | null
  template: FingerprintTemplate
  images: string[]
  scanCount: number
  templateSampleCount: number
  minutiaeCount: number
}

export interface VerifyOptions {
  username?: string
  template?: FingerprintTemplate
  threshold?: number
  timeoutSecs?: number
}

export interface FingerprintTemplate {
  version: number
  username: string
  finger: string
  samples: TemplateMinutia[][]
}

export interface TemplateMinutia {
  x: number
  y: number
  theta: number
}

export interface VerifyResult {
  matched: boolean
  username: string
  finger: string
  score: number
  threshold: number
}

export interface IdentifyOptions {
  threshold?: number
  timeoutSecs?: number
}

export interface IdentifyResult {
  matched: boolean
  username: string | null
  finger: string | null
  score: number | null
  threshold: number
  galleryCount: number
}

export async function checkDevice(): Promise<DeviceStatus> {
  return await invoke<DeviceStatus>('plugin:fingerprint|check_device')
}

export async function getDeviceInfo(): Promise<DeviceInfo> {
  return await invoke<DeviceInfo>('plugin:fingerprint|get_device_info')
}

export async function capture(options: CaptureOptions = {}): Promise<CaptureResult> {
  return await invoke<CaptureResult>('plugin:fingerprint|capture', {
    payload: options,
  })
}

export async function captureFrames(
  options: CaptureFramesOptions = {},
): Promise<CapturedFrame[]> {
  return await invoke<CapturedFrame[]>('plugin:fingerprint|capture_frames', {
    payload: options,
  })
}

export async function enroll(options: EnrollOptions): Promise<EnrollResult> {
  return await invoke<EnrollResult>('plugin:fingerprint|enroll', {
    payload: options,
  })
}

export async function verify(options: VerifyOptions): Promise<VerifyResult> {
  return await invoke<VerifyResult>('plugin:fingerprint|verify', {
    payload: options,
  })
}

export async function identify(options: IdentifyOptions = {}): Promise<IdentifyResult> {
  return await invoke<IdentifyResult>('plugin:fingerprint|identify', {
    payload: options,
  })
}
