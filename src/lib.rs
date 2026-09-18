use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
pub mod template;
#[cfg(desktop)]
pub mod uru4500;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::Fingerprint;
#[cfg(mobile)]
use mobile::Fingerprint;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the fingerprint APIs.
pub trait FingerprintExt<R: Runtime> {
    fn fingerprint(&self) -> &Fingerprint<R>;
}

impl<R: Runtime, T: Manager<R>> crate::FingerprintExt<R> for T {
    fn fingerprint(&self) -> &Fingerprint<R> {
        self.state::<Fingerprint<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("fingerprint")
        .invoke_handler(tauri::generate_handler![
            commands::check_device,
            commands::get_device_info,
            commands::capture,
            commands::capture_frames,
            commands::enroll,
            commands::verify,
            commands::identify,
        ])
        .setup(|app, api| {
            #[cfg(mobile)]
            let fingerprint = mobile::init(app, api)?;
            #[cfg(desktop)]
            let fingerprint = desktop::init(app, api)?;
            app.manage(fingerprint);
            Ok(())
        })
        .build()
}
