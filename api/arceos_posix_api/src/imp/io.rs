use crate::ctypes;
use axerrno::{LinuxError, LinuxResult};
use core::ffi::{c_int, c_uint, c_ulong, c_void};

#[cfg(feature = "fd")]
use crate::imp::fd_ops::get_file_like;
#[cfg(not(feature = "fd"))]
use axio::prelude::*;

/// Read data from the file indicated by `fd`.
///
/// Return the read size if success.
pub fn sys_read(fd: c_int, buf: *mut c_void, count: usize) -> ctypes::ssize_t {
    debug!("sys_read <= {} {:#x} {}", fd, buf as usize, count);
    syscall_body!(sys_read, {
        if buf.is_null() {
            return Err(LinuxError::EFAULT);
        }
        let dst = unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, count) };
        #[cfg(feature = "fd")]
        {
            Ok(get_file_like(fd)?.read(dst)? as ctypes::ssize_t)
        }
        #[cfg(not(feature = "fd"))]
        match fd {
            0 => Ok(super::stdio::stdin().read(dst)? as ctypes::ssize_t),
            1 | 2 => Err(LinuxError::EPERM),
            _ => Err(LinuxError::EBADF),
        }
    })
}

fn write_impl(fd: c_int, buf: *const c_void, count: usize) -> LinuxResult<ctypes::ssize_t> {
    if buf.is_null() {
        return Err(LinuxError::EFAULT);
    }
    let src = unsafe { core::slice::from_raw_parts(buf as *const u8, count) };
    #[cfg(feature = "fd")]
    {
        Ok(get_file_like(fd)?.write(src)? as ctypes::ssize_t)
    }
    #[cfg(not(feature = "fd"))]
    match fd {
        0 => Err(LinuxError::EPERM),
        1 | 2 => Ok(super::stdio::stdout().write(src)? as ctypes::ssize_t),
        _ => Err(LinuxError::EBADF),
    }
}

/// Write data to the file indicated by `fd`.
///
/// Return the written size if success.
pub fn sys_write(fd: c_int, buf: *const c_void, count: usize) -> ctypes::ssize_t {
    debug!("sys_write <= {} {:#x} {}", fd, buf as usize, count);
    syscall_body!(sys_write, write_impl(fd, buf, count))
}

/// Write a vector.
pub unsafe fn sys_writev(fd: c_int, iov: *const ctypes::iovec, iocnt: c_int) -> ctypes::ssize_t {
    debug!("sys_writev <= fd: {}", fd);
    syscall_body!(sys_writev, {
        if !(0..=1024).contains(&iocnt) {
            return Err(LinuxError::EINVAL);
        }

        let iovs = unsafe { core::slice::from_raw_parts(iov, iocnt as usize) };
        let mut ret = 0;
        for iov in iovs.iter() {
            let result = write_impl(fd, iov.iov_base, iov.iov_len)?;
            ret += result;

            if result < iov.iov_len as isize {
                break;
            }
        }

        Ok(ret)
    })
}
/// Read a vector.
pub unsafe fn sys_readv(fd: c_int, iov: *const ctypes::iovec, iocnt: c_int) -> ctypes::ssize_t {
    debug!("sys_writev <= fd: {}", fd);
    syscall_body!(sys_writev, {
        if !(0..=1024).contains(&iocnt) {
            return Err(LinuxError::EINVAL);
        }

        let iovs = unsafe { core::slice::from_raw_parts(iov, iocnt as usize) };

        let mut ret = 0;
        for iov in iovs.iter() {
            let result = sys_read(fd, iov.iov_base, iov.iov_len);
            ret += result;

            if result < iov.iov_len as isize {
                break;
            }
        }

        Ok(ret)
    })
}

//
pub unsafe fn sys_fsync(fd: c_int) -> ctypes::ssize_t {
    debug!("sys_fsync  fd: {}", fd);
    syscall_body!(sys_fsync, {
        let file = get_file_like(fd)?;
        //rust的这类型真挺让人头大，在某些情况下结果可能不正常
        let _ = file.flush()?;
        Ok(0)
    })
}

use num_enum::TryFromPrimitive;
pub unsafe fn sys_ioctl(fd: c_int, cmd: c_uint, arg: c_ulong) -> ctypes::ssize_t {
    debug!("ioctl");
    syscall_body!(sys_ioctl, {
        let cmd = match IoctlCmd::try_from(cmd) {
            Ok(cmd) => cmd,
            Err(_) => {
                return Err(LinuxError::EINVAL);
            }
        };

        debug!("fd = {}, ioctl_cmd = {:?}, arg = 0x{:x}", fd, cmd, arg);

        let mut file = get_file_like(fd)?;

        let ret = match cmd {
            IoctlCmd::FIONBIO => {
                //设置文件的非阻塞模式（O_NONBLOCK）。
                file.set_nonblocking(arg & (ctypes::O_NONBLOCK as u64) > 0)?; //服了，这样也不是不行
                0
            }
            IoctlCmd::FIOASYNC => {
                //信号机制未完善
                0
            }
            IoctlCmd::FIOCLEX => {
                //设置文件的 close-on-exec 标志（CLOEXEC）。

                0
            }
            IoctlCmd::FIONCLEX => {
                // Clears the close-on-exec flag of the file.
                // Follow the implementation of fcntl()
                0
            }
            // FIXME: ioctl operations involving blocking I/O should be able to restart if interrupted
            _ => {
                // let file_owned = file.to_owned();
                // We have to drop `file_table` because some I/O command will modify the file table
                // (e.g., TIOCGPTPEER).
                // drop(file_table);

                // file_owned.ioctl(ioctl_cmd, arg)?
                0
            }
        };

        //文件系统相关，看fs

        //这样

        //
        warn!("ioctl:unimplemented,but ok!");
        Ok(0)
    })
}

// /tools/include/uapi/asm-generic/ioctls.h
//参考星绽
#[repr(u32)]
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
pub enum IoctlCmd {
    /// Get terminal attributes
    TCGETS = 0x5401,
    TCSETS = 0x5402,
    /// Drain the output buffer and set attributes
    TCSETSW = 0x5403,
    /// Drain the output buffer, and discard pending input, and set attributes
    TCSETSF = 0x5404,
    /// Make the given terminal the controlling terminal of the calling process.
    TIOCSCTTY = 0x540e,
    /// Get the process group ID of the foreground process group on this terminal
    TIOCGPGRP = 0x540f,
    /// Set the foreground process group ID of this terminal.
    TIOCSPGRP = 0x5410,
    /// Get the number of bytes in the input buffer.
    FIONREAD = 0x541B,
    /// Set window size
    TIOCGWINSZ = 0x5413,
    TIOCSWINSZ = 0x5414,
    /// Enable or disable non-blocking I/O mode.
    FIONBIO = 0x5421,
    /// the calling process gives up this controlling terminal
    TIOCNOTTY = 0x5422,
    /// Clear the close on exec flag on a file descriptor
    FIONCLEX = 0x5450,
    /// Set the close on exec flag on a file descriptor
    FIOCLEX = 0x5451,
    /// Enable or disable asynchronous I/O mode.
    FIOASYNC = 0x5452,
    /// Get Pty Number
    TIOCGPTN = 0x80045430,
    /// Lock/unlock Pty
    TIOCSPTLCK = 0x40045431,
    /// Safely open the slave
    TIOCGPTPEER = 0x40045441,
    /// Get tdx report using TDCALL
    TDXGETREPORT = 0xc4405401,
}
