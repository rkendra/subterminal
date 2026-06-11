use std::os::fd::*;
use std::ptr;
use std::mem::MaybeUninit;

use std::io::prelude::*;
use std::io::Result;

pub struct Pty {
    manager: RawFd,
    child_pid: i32,
    out_stream: RawFd
}

impl Pty {
    /// Spawns a child process that is connected to a new pseudoterminal
    /// Child process executes the user's default shell
    pub fn spawn_shell() -> Result<Pty> {
        let mut manager: libc::c_int = -1;
        let mut child_pid = -1;
        // SAFETY: manager was just initialized, and null pointers are properly handled by forkpty
        // Child process is properly handled in implementation of Drop
        let pid = unsafe {
            libc::forkpty(&mut manager, ptr::null_mut(), ptr::null(), ptr::null())
        };
        match pid {
            -1 => return Err(std::io::Error::last_os_error()),
            0 => Pty::execute(std::env::var("SHELL").unwrap(), Vec::<&str>::new())?,
            child => child_pid = child
        }

        Ok(Pty{ manager, child_pid, out_stream: manager })
    }

    pub fn spawn_piped_shell() -> Result<Pty> {
        let mut manager: libc::c_int = -1;
        let mut child_pid: i32 = -1;
        let mut pipe: [libc::c_int; 2] = [-1; 2];
        // SAFETY: pipe was just declared as an array of two c_ints
        // which is required by libc::pipe
        let pipe_res = unsafe {
            libc::pipe(pipe.as_mut_ptr())
        };
        match pipe_res {
            -1 => return Err(std::io::Error::last_os_error()),
            _ => {}
        }
        // SAFETY: manager was just initialized, and null pointers are properly handled by forkpty
        // Child process is properly handled in implementation of Drop
        let pid = unsafe {
            libc::forkpty(&mut manager, ptr::null_mut(), ptr::null(), ptr::null())
        };
        match pid {
            -1 => return Err(std::io::Error::last_os_error()),
            0 => {
                // SAFETY: both pipe fds are guaranteed to be open at this point
                unsafe {
                    handle_c_ret(libc::close(pipe[0]))?;
                    handle_c_ret(libc::dup2(libc::STDOUT_FILENO, pipe[1]))?;
                }
                Pty::execute(std::env::var("SHELL").unwrap(), Vec::<&str>::new())?
            },
            child => {
                // SAFETY: pipe[1] is guaranteed to be open at this point
                unsafe {
                    handle_c_ret(libc::close(pipe[1]))?
                }
                child_pid = child;
            }
        }

        Ok(Pty{ manager, child_pid, out_stream: pipe[0] })
    }

    pub fn get_termios_flags(&self) -> Result<libc::termios> {
        let mut flags:MaybeUninit<libc::termios> = MaybeUninit::uninit();
        // SAFETY: tcgetattr is guaranteed to initialize flags when no error occurs
        unsafe {
            handle_c_ret(libc::tcgetattr(self.manager, flags.as_mut_ptr()))?;
            Ok(flags.assume_init())
        }
    }

    pub fn set_termios_flags(&mut self, option: i32, flags: libc::termios) -> Result<()> {
        unsafe {
            handle_c_ret(libc::tcsetattr(self.manager, option, &flags))
        }
    }
    // To be only invoked by forked child processes
    // Executes the program indicated by path
    // Does not return unless an error occured
    fn execute(cmd: String, args: Vec<impl Into<Vec<u8>>>) -> Result<()> {
        let mut byte_args: Vec<Vec<u8>> = Vec::new();
        byte_args.push(cmd.as_bytes().into());
        for i in args {
            byte_args.push(i.into());
        }
        let mut arg_ptrs: Vec<*const i8> = Vec::new();
        for i in byte_args {
            arg_ptrs.push(i.as_ptr() as *const i8);
        }
        // SAFETY: argv[0] is guaranteed to be equal to cmd
        // All parameters are guaranteed to be valid ASCII at this point (i.e., n <= 128)
        unsafe {
            handle_c_ret(
                libc::execvp(cmd.as_ptr() as *const i8, arg_ptrs.as_ptr())
            )
        }
    }

    pub fn get_raw_pty() -> (OwnedFd, OwnedFd) {
        let mut master: libc::c_int = -1;
        let mut slave: libc::c_int = -1;
        // SAFETY: master and slave were just initialized, so their
        // pointers are valid, null pointers are handled by openpty
        unsafe {
            libc::openpty(&mut master, &mut slave, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
        }
        // SAFETY: master and slave are created in the prior syscall, meaning no other code can access
        // these fds before ownership is consumed
        unsafe {
            (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave))
        }
    }

    /// Creates a new terminal session with a new child process that executes cmd
    /// returns a Pty object that interacts with said process
    /// # Safety
    /// This function consumes ownership of master and slave, and closes slave.
    /// Should master and slave contain the same fd, this will result in undefined behavior
    pub unsafe fn from_raw_pty(master: OwnedFd, slave: OwnedFd, cmd: String) -> Result<Pty> {
        let pid = unsafe {
            libc::fork()
        };
        match pid {
            -1 => return Err(std::io::Error::last_os_error()),
            0 => {
                unsafe {
                    handle_c_ret(libc::login_tty(slave.into_raw_fd()))?
                }
                let args = cmd.split(char::is_whitespace).collect();
                Pty::execute(args[0], args[1..].to_vec)?
            }
        }
    }
}

impl std::ops::Drop for Pty {
    fn drop(&mut self) {
        // Closing the fd first causes child process to receive SIGHUP
        // waitpid exists as a sanity check
        // SAFETY: All RawFds are guaranteed to be open due to being private fields
        // and guards are in place to prevent double closures
        unsafe {
            if self.out_stream != self.manager {
                libc::close(self.out_stream);
            }
            libc::close(self.manager);
            libc::waitpid(self.child_pid, ptr::null_mut(), 0);
        }
    }
}

impl Read for Pty {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read_limit = if buf.len() < libc::ssize_t::MAX as usize{
            buf.len()
        } else {
            libc::ssize_t::MAX as usize
        };
        let bytes_read = unsafe { 
            libc::read(
                self.out_stream.as_raw_fd(),
                buf.as_mut_ptr() as *mut libc::c_void,
                read_limit
            )
        };
        if bytes_read == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(bytes_read as usize)
        }
    }
}

impl Write for Pty {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let read_limit = if buf.len() < libc::ssize_t::MAX as usize{
            buf.len()
        } else {
            libc::ssize_t::MAX as usize
        };
        let bytes_written = unsafe { 
            libc::write(
                self.manager.as_raw_fd(),
                buf.as_ptr() as *const libc::c_void,
                read_limit
            )
        };
        if bytes_written == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(bytes_written as usize)
        }
    }

    // Empty function as Pty has no buffer
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// impl crate::Echo for Pty {
//     fn echo(&mut self, buf: &[u8]) -> Result<(usize, Option<Vec<u8>>)> {
//         self.write(buf);
        
//     }
// }

fn handle_c_ret(code: libc::c_int) -> Result<()> {
    match code {
        -1 => Err(std::io::Error::last_os_error()),
        _ => Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_error_handler() {
        assert_eq!(handle_c_ret(-1).is_ok(), false);
        assert_eq!(handle_c_ret(0).is_ok(), true);
        assert_eq!(handle_c_ret(27).is_ok(), true);
    }
}