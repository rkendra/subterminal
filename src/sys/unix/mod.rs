mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;

// Support for other Unix-based OSes not implemented yet
#[cfg(not(target_os = "linux"))]
pub struct Pty;