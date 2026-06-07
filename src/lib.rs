mod sys;

use crate::sys as imp;
use crate::sys::{IntoInner, FromInner, AsInner, AsInnerMut};
use std::io::Result;
use std::io::prelude::*;


pub struct Pty {
    inner: imp::Pty,
}

impl IntoInner<imp::Pty> for Pty {
    fn into_inner(self) -> imp::Pty {
        self.inner
    }
}

impl FromInner<imp::Pty> for Pty {
    fn from_inner(inner: imp::Pty) -> Pty {
        Pty {
            inner
        }
    }
}

impl AsInner<imp::Pty> for Pty {
    fn as_inner(&self) -> &imp::Pty {
        &self.inner
    }
}

impl AsInnerMut<imp::Pty> for Pty {
    fn as_inner_mut(&mut self) -> &mut imp::Pty {
        &mut self.inner
    }
}

impl Pty {
    pub fn spawn_shell() -> Result<Pty> {
        Ok(Pty{inner: imp::Pty::spawn_shell()?})
    }

    #[cfg(unix)]
    pub fn data_readable(&self) -> bool {
        self.inner.data_readable()
    }
}

impl Read for Pty {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.read(buf)
    }
}

/// Note for Unix developers:
/// As only one pipe is created by the OS for a pty, it is recommended
/// to call data_readable() before performing any write operation, as 
/// writing will void any buffered readable data. As such, while io::Write 
/// is fully implemented, use of bulk-writing methods such as write_all and 
/// write_fmt is generally not recommended.
impl Write for Pty {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}