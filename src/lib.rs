mod sys;

use crate::sys as imp;
use std::io::Result;
use std::io::prelude::*;
use std::io::Error;
use std::io::ErrorKind;

#[cfg(unix)]
use std::os::fd::OwnedFd;

pub struct PtyIn {
    inner: imp::PtyIn,
}

pub struct PtyOut {
    inner: imp::PtyOut,
}

pub struct Pty {
    inner: imp::Pty,
    pub input: PtyIn,
    pub output: PtyOut,
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

impl Pty {
    /// Spawns a child process that is connected to a new pseudoterminal.
    /// Child process executes the user's default shell.
    pub fn spawn_shell(cmd: String) -> Result<Pty> {
        let inner = imp::Pty::spawn_shell(cmd)?;
        let (input, output) = inner.get_io();
        Ok(Pty{ inner, input: PtyIn{inner: input}, output: PtyOut{inner: output}})
    }

    #[cfg(unix)]
    /// Creates a new terminal session with a new child process that executes cmd
    /// and returns a Pty object that interacts with said process.
    /// # Safety
    /// This function consumes ownership of master and slave, and closes slave.
    /// Should master and slave contain the same fd, this will result in undefined behavior.
    pub unsafe fn from_raw_pty(master: OwnedFd, slave: OwnedFd, cmd: String) -> Result<Pty> {
        unsafe {
            let inner = imp::Pty::from_raw_pty(master, slave, cmd)?;
            let (input, output) = inner.get_io();
            Ok(Pty{ inner, input: PtyIn{inner: input}, output: PtyOut{inner: output}})
        }
    }

    #[cfg(unix)]
    pub fn get_termios_flags(&self) -> Result<libc::termios> {
        self.inner.get_termios_flags()
    }

    #[cfg(unix)]
    pub fn set_termios_flags(&mut self, options: i32, flags: libc::termios) -> Result<()> {
        self.inner.set_termios_flags(options, flags)
    }

    pub fn shutdown(&self) {
        self.inner.shutdown();
    }

    pub fn enable_raw(&mut self) -> Result<()> {
        self.inner.enable_raw()?;
        Ok(())
    }

}

impl Read for PtyOut {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.read(buf)
    }
}

/// Note for Unix developers:
/// If reading a terminal's echo output is desired,
/// it is instead recommended to use the Echo trait, 
/// as io::Read reads from a pty's underlying process,
/// not the terminal device itself
impl Write for PtyIn {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}
