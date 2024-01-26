#[cfg(any(target_os = "windows", doc))]
pub mod win32;

#[cfg(any(target_os = "windows", doc))]
pub mod dx12;
