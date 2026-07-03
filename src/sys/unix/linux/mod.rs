use std::os::fd::*;
use std::ptr;
use std::mem::MaybeUninit;

use std::io::prelude::*;
use std::io::{BufReader, Result};
use std::fs::File;

pub struct PtyOut {
    file: RawFd
}

pub struct PtyIn {
    file: RawFd
}

pub struct Pty {
    manager: RawFd,
    child_pid: i32,
}

impl Pty {
    /// Spawns a child process that is connected to a new pseudoterminal
    /// Child process executes the user's default shell
    pub fn spawn(cmd: String) -> Result<Pty> {
        let mut manager: libc::c_int = -1;
        let mut child_pid = -1;
        // SAFETY: manager was just initialized, and null pointers are properly handled by forkpty
        // Child process is properly handled in implementation of Drop
        let pid = unsafe {
            libc::forkpty(&mut manager, ptr::null_mut(), ptr::null(), ptr::null())
        };
        match pid {
            -1 => return Err(std::io::Error::last_os_error()),
            0 => {
                let args: Vec<&str> = cmd.split(char::is_whitespace).collect();
                Pty::execute(args[0].to_string(), args[1..].to_vec())?
            },
            child => child_pid = child
        }

        Ok(Pty{ manager, child_pid })
    }

    pub fn spawn_as_user(cmd: &str, uname: &str) -> Result<Pty> {
        // Get gid and uid of user
        let passwd = BufReader::new(File::open("/etc/passwd").expect("Failed to open passwd"));
        let (uid, gid): (u32, u32);
        let mut lines_iter = passwd.lines();
        loop {
            let line = lines_iter.next();
            match line {
                None => {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Requested user does not exist"));
                }
                Some(u_data) => match u_data {
                    Err(err) => {
                        return Err(err);
                    }
                    Ok(user) => {
                        let user_info: Vec<&str> = user.split(':').collect();
                        if user_info[0] == uname {
                            uid = user_info[2].parse().unwrap();
                            gid = user_info[3].parse().unwrap();
                            break;
                        }
                    }
                }
            }
        }
        // Sanity check in case loop above somehow neither finds the user nor returns Err
        if uid == 0 && gid == 0 && uname != "root" {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Requested user does not exist"))
        }

        let mut manager: libc::c_int = -1;
        let mut child_pid = -1;
        // SAFETY: manager was just initialized, and null pointers are properly handled by forkpty
        // Child process is properly handled in implementation of Drop
        let pid = unsafe {
            libc::forkpty(&mut manager, ptr::null_mut(), ptr::null(), ptr::null())
        };
        match pid {
            -1 => return Err(std::io::Error::last_os_error()),
            0 => {
                let args: Vec<&str> = cmd.split(char::is_whitespace).collect();
                
                // SAFETY: gid and uid guaranteed to point to valid user/group
                unsafe {
                    libc::setgid(gid);
                    libc::setuid(uid);
                }
                Pty::execute(args[0].to_string(), args[1..].to_vec())?
            },
            child => child_pid = child
        }

        Ok(Pty{ manager, child_pid })
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
        for i in byte_args.iter() {
            arg_ptrs.push(i.as_ptr() as *const i8);
        }
        arg_ptrs.push(std::ptr::null());
        // SAFETY: argv[0] is guaranteed to be equal to cmd
        // All parameters are guaranteed to be valid ASCII at this point (i.e., n <= 128)
        unsafe {
            handle_c_ret(
                libc::execvp(cmd.as_ptr() as *const i8, arg_ptrs.as_ptr())
            )
        }
    }

    pub fn enable_raw(&mut self) -> Result<()> {
        unsafe {
            let mut term_settings: MaybeUninit<libc::termios> = MaybeUninit::uninit();
            handle_c_ret(libc::tcgetattr(self.manager, term_settings.as_mut_ptr()))?;
            let mut term_settings = term_settings.assume_init();
            term_settings.c_lflag = term_settings.c_lflag & !(libc::ECHO);
            term_settings.c_lflag = term_settings.c_lflag & !(libc::ICANON);
            term_settings.c_iflag = term_settings.c_iflag & !(libc::IGNCR);
            handle_c_ret(libc::tcsetattr(self.manager, libc::TCSANOW, &mut term_settings))?;
        }
        Ok(())
    }

    /// Creates a new terminal session with a new child process that executes cmd
    /// returns a Pty object that interacts with said process
    /// # Safety
    /// This function consumes ownership of master and slave, and closes slave.
    /// Should master and slave contain the same fd, this will result in undefined behavior
    pub unsafe fn from_raw_pty(master: OwnedFd, slave: OwnedFd, cmd: String) -> Result<Pty> {
        let mut child_pid = -1;
        let pid = unsafe {
            libc::fork()
        };
        match pid {
            -1 => return Err(std::io::Error::last_os_error()),
            0 => {
                unsafe {
                    handle_c_ret(libc::login_tty(slave.into_raw_fd()))?
                }
                let args: Vec<&str> = cmd.split(char::is_whitespace).collect();
                Pty::execute(args[0].to_string(), args[1..].to_vec())?
            },
            child => child_pid = child
        }
        let master = master.into_raw_fd();
        Ok(Pty{ manager: master, child_pid })
    }
    
    pub fn get_io(&self) -> (PtyIn, PtyOut) {
        return (PtyIn{ file: self.manager }, PtyOut{ file: self.manager })
    }

    pub fn shutdown(&self) {
        unsafe {
            libc::kill(self.child_pid, libc::SIGHUP);
        }
    }
}

impl std::ops::Drop for Pty {
    fn drop(&mut self) {
        // Closing the fd first causes child process to receive SIGHUP
        // waitpid exists as a sanity check
        // SAFETY: Manager is guaranteed to be open due to being private
        unsafe {
            libc::close(self.manager);
            libc::waitpid(self.child_pid, ptr::null_mut(), 0);
        }
    }
}

impl Read for PtyOut {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read_limit = if buf.len() < libc::ssize_t::MAX as usize{
            buf.len()
        } else {
            libc::ssize_t::MAX as usize
        };
        let bytes_read = unsafe { 
            libc::read(
                self.file,
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

impl Write for PtyIn {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let read_limit = if buf.len() < libc::ssize_t::MAX as usize{
            buf.len()
        } else {
            libc::ssize_t::MAX as usize
        };
        let bytes_written = unsafe { 
            libc::write(
                self.file,
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
