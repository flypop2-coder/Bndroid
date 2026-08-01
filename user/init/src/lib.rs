#![no_std]
#![cfg_attr(
    any(feature = "app-lifecycle-runtime", feature = "storage-server-runtime"),
    allow(dead_code)
)]

#[cfg(all(feature = "androidbox-dex0", not(feature = "androidbox-apk-install0")))]
mod androidbox_runtime;

#[cfg(feature = "androidbox-el0-runtime0")]
mod androidbox_installed_runtime;

#[path = "main.rs"]
mod implementation;

pub use implementation::{
    app_entry, child_panic, client_entry, init_entry, init_panic, input_server_entry,
    launcher_entry, provider_entry, service_manager_entry, surface_server_entry,
};

#[cfg(feature = "androidbox-process0")]
pub use implementation::android_app_entry;

#[cfg(feature = "storage-server-runtime")]
pub use implementation::storage_server_entry;
