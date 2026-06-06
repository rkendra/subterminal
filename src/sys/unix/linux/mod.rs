use std::os::fd::*;
use std::ptr;
use std::mem::ManuallyDrop;
use std::io::prelude::*;
use std::io::Result;

pub struct Pty {
    manager: ManuallyDrop<OwnedFd>,
    child_pid: i32
}

impl Pty {
    // Spawns a child process that is connected to a new pseudoterminal
    // Child process executes the user's default shell
    pub fn spawn_shell() -> Result<Pty> {
        let mut manager: libc::c_int = -1;
        let mut child_pid = -1;
        let pid = unsafe {
            libc::forkpty(&mut manager, ptr::null_mut(), ptr::null(), ptr::null())
        };
        match pid {
            -1 => return Err(std::io::Error::last_os_error()),
            0 => Pty::execute(std::env::var("SHELL").unwrap(), Vec::<&str>::new())?,
            child => child_pid = child
        }
        // Have Pty object take ownership of master pty
        let manager = unsafe {
            OwnedFd::from_raw_fd(manager)
        };

        Ok(Pty{ manager: ManuallyDrop::new(manager), child_pid })
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
        let status = unsafe {
            libc::execvp(cmd.as_ptr() as *const i8, arg_ptrs.as_ptr())
        };
        match status {
            -1 => Err(std::io::Error::last_os_error()),
            _ => Ok(())
        }
    }
}

impl std::ops::Drop for Pty {
    fn drop(&mut self) {
        // Closing the fd first causes child process to receive SIGHUP
        // waitpid exists as a sanity check
        unsafe {
            ManuallyDrop::drop(&mut self.manager);
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
                self.manager.as_raw_fd(),
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