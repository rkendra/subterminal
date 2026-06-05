mod unix;
mod windows;

#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
pub use windows::*;

// Trait implementations taken from std::sys
pub trait IntoInner<Inner> {
    fn into_inner(self) -> Inner;
}

pub trait FromInner<Inner> {
    fn from_inner(inner: Inner) -> Self;
}

#[cfg_attr(not(target_os = "linux"), allow(unused))]
pub trait AsInner<Inner> {
    fn as_inner(&self) -> &Inner;
}

#[cfg_attr(not(target_os = "linux"), allow(unused))]
pub trait AsInnerMut<Inner> {
    fn as_inner_mut(&mut self) -> &mut Inner;
}