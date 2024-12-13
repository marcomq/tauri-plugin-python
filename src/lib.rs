use tauri::{
  plugin::{Builder, TauriPlugin},
  Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::Python;
#[cfg(mobile)]
use mobile::Python;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the python APIs.
pub trait PythonExt<R: Runtime> {
  fn python(&self) -> &Python<R>;
}

impl<R: Runtime, T: Manager<R>> crate::PythonExt<R> for T {
  fn python(&self) -> &Python<R> {
    self.state::<Python<R>>().inner()
  }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("python")
    .invoke_handler(tauri::generate_handler![commands::ping])
    .setup(|app, api| {
      #[cfg(mobile)]
      let python = mobile::init(app, api)?;
      #[cfg(desktop)]
      let python = desktop::init(app, api)?;
      app.manage(python);
      Ok(())
    })
    .build()
}
