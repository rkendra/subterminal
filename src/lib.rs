mod sys;

use crate::sys as imp;
use crate::sys::{IntoInner, FromInner, AsInner, AsInnerMut};
use std::io::Result;
use std::io::prelude::*;
use std::io::Error;
use std::io::ErrorKind;


pub struct Pty {
    inner: imp::Pty,
}

#[cfg(unix)]
pub trait Echo: Write {
    /// Writes a buffer into the underlying writer,
    /// Returns the amount of bytes written, and an optional vector.
    /// Should the echo output differ from buf (as is the case with some ANSI escape sequences), Some(output) will be returned, 
    /// otherwise, None will be returned
    /// Any error from the underlying write() call should propagate
    fn echo(&mut self, buf: &[u8]) -> Result<(usize, Option<Vec<u8>>)>;

    // Implementations for the following methods are adapted from the 
    // method's analogues in std::io::Write

    fn echo_all(&mut self, buf: &[u8]) -> Result<Option<Vec<u8>>> {
        let mut out = Vec::new();
        let mut cursor = buf;
        while !cursor.is_empty() {
            match self.echo(cursor) {
                Ok((0, None)) => {
                    return Err(Error::new(ErrorKind::WriteZero, "Associated fd closed"))
                }
                Ok((n, None)) => {
                    out.extend_from_slice(&cursor[..n]);
                    cursor = &cursor[n..];
                }
                Ok((n, Some(vec))) => {
                    out.extend_from_slice(&vec[..]);
                    cursor = &cursor[n..];
                }
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        if buf == out {
            Ok(None)
        } else {
            Ok(Some(out))
        }
    }
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
    /// Spawns a child process tied to a new pseudoterminal
    /// which runs the user's default shell. Creates a distinct channel
    /// that captures output from the child process
    /// 
    /// On Windows, pseudoterminals have distinct input and output channels by default, 
    /// so this function is not included to avoid confusion as it is redundant
    pub fn spawn_piped_shell() -> Result<Pty> {
        Ok(Pty{inner: imp::Pty::spawn_piped_shell()?})
    }
}

impl Read for Pty {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.read(buf)
    }
}

/// Note for Unix developers:
/// If reading a terminal's echo output is desired,
/// it is instead recommended to use the Echo trait, 
/// as io::Read reads from a pty's underlying process,
/// not the terminal device itself
impl Write for Pty {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}