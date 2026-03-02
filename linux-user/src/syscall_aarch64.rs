use crate::guest_space::GuestSpace;
use crate::syscall::SyscallResult;

// AArch64 Linux syscall numbers (asm-generic)
const SYS_FCNTL: u64 = 25;
const SYS_DUP: u64 = 23;
const SYS_DUP3: u64 = 24;
const SYS_IOCTL: u64 = 29;
const SYS_GETCWD: u64 = 17;
const SYS_PIPE2: u64 = 59;
const SYS_UNLINKAT: u64 = 35;
const SYS_FACCESSAT: u64 = 48;
const SYS_OPENAT: u64 = 56;
const SYS_CLOSE: u64 = 57;
const SYS_LSEEK: u64 = 62;
const SYS_READ: u64 = 63;
const SYS_WRITE: u64 = 64;
const SYS_READV: u64 = 65;
const SYS_WRITEV: u64 = 66;
const SYS_READLINKAT: u64 = 78;
const SYS_FSTATFS: u64 = 44;
const SYS_NEWFSTATAT: u64 = 79;
const SYS_FSTAT: u64 = 80;
const SYS_EXIT: u64 = 93;
const SYS_EXIT_GROUP: u64 = 94;
const SYS_SET_TID_ADDRESS: u64 = 96;
const SYS_FUTEX: u64 = 98;
const SYS_SET_ROBUST_LIST: u64 = 99;
const SYS_CLOCK_GETTIME: u64 = 113;
const SYS_CLOCK_NANOSLEEP: u64 = 115;
const SYS_TGKILL: u64 = 131;
const SYS_RT_SIGACTION: u64 = 134;
const SYS_RT_SIGPROCMASK: u64 = 135;
const SYS_UNAME: u64 = 160;
const SYS_GETPID: u64 = 172;
const SYS_GETPPID: u64 = 173;
const SYS_GETUID: u64 = 174;
const SYS_GETEUID: u64 = 175;
const SYS_GETGID: u64 = 176;
const SYS_GETEGID: u64 = 177;
const SYS_GETTID: u64 = 178;
const SYS_BRK: u64 = 214;
const SYS_MUNMAP: u64 = 215;
const SYS_MREMAP: u64 = 216;
const SYS_MMAP: u64 = 222;
const SYS_EXECVE: u64 = 221;
const SYS_MPROTECT: u64 = 226;
const SYS_MADVISE: u64 = 233;
const SYS_PRLIMIT64: u64 = 261;
const SYS_WAIT4: u64 = 260;
const SYS_GETRANDOM: u64 = 278;
const SYS_RSEQ: u64 = 293;
const SYS_CLONE: u64 = 220;

const ENOSYS: u64 = (-38i64) as u64;
const ENOTTY: u64 = (-25i64) as u64;
const ENOMEM: u64 = (-12i64) as u64;
const EINVAL: u64 = (-22i64) as u64;

/// Handle an AArch64 Linux syscall.
///
/// `regs` is X0-X30 (31 registers).
/// Syscall number in X8, args in X0-X5.
/// Return value written to regs[0] by caller.
pub fn handle_syscall_aarch64(
    space: &mut GuestSpace,
    regs: &mut [u64; 31],
    _sp: &mut u64,
    mmap_next: &mut u64,
    elf_path: &str,
) -> SyscallResult {
    let nr = regs[8]; // X8
    let a0 = regs[0]; // X0
    let a1 = regs[1]; // X1
    let a2 = regs[2]; // X2
    let a3 = regs[3]; // X3
    let a4 = regs[4]; // X4
    if std::env::var("TCG_SYSCALL").is_ok() {
        eprintln!(
            "[syscall] nr={nr} a0={a0:#x} a1={a1:#x} a2={a2:#x} a3={a3:#x} lr={:#x}",
            regs[30],
        );
    }
    let result = match nr {
        SYS_WRITE => {
            let fd = a0 as i32;
            let buf = a1;
            let len = a2 as usize;
            let host_buf = space.g2h(buf);
            let ret = unsafe {
                libc::write(fd, host_buf as *const libc::c_void, len)
            };
            if ret < 0 {
                let e = unsafe { *libc::__errno_location() };
                SyscallResult::Continue((-e) as u64)
            } else {
                SyscallResult::Continue(ret as u64)
            }
        }
        SYS_EXIT | SYS_EXIT_GROUP => SyscallResult::Exit(a0 as i32),
        SYS_BRK => do_brk(space, a0),
        SYS_MMAP => do_mmap(space, a0, a1, a2, mmap_next),
        SYS_MREMAP => do_mremap(space, a0, a1, a2, a3, a4, mmap_next),
        SYS_MPROTECT => {
            let addr = a0;
            let len = a1 as usize;
            let prot = a2 as i32;
            match space.mprotect(addr, len, prot) {
                Ok(()) => SyscallResult::Continue(0),
                Err(_) => SyscallResult::Continue((-22i64) as u64),
            }
        }
        SYS_MUNMAP | SYS_SET_ROBUST_LIST | SYS_RT_SIGACTION
        | SYS_RT_SIGPROCMASK | SYS_MADVISE => SyscallResult::Continue(0),
        SYS_SET_TID_ADDRESS => SyscallResult::Continue(host_gettid() as u64),
        SYS_GETPID => SyscallResult::Continue(unsafe { libc::getpid() as u64 }),
        SYS_GETPPID => {
            SyscallResult::Continue(unsafe { libc::getppid() as u64 })
        }
        SYS_GETTID => SyscallResult::Continue(host_gettid() as u64),
        SYS_GETUID => SyscallResult::Continue(unsafe { libc::getuid() as u64 }),
        SYS_GETEUID => {
            SyscallResult::Continue(unsafe { libc::geteuid() as u64 })
        }
        SYS_GETGID => SyscallResult::Continue(unsafe { libc::getgid() as u64 }),
        SYS_GETEGID => {
            SyscallResult::Continue(unsafe { libc::getegid() as u64 })
        }
        SYS_GETRANDOM => {
            let buf = a0;
            let len = a1 as usize;
            let host = space.g2h(buf);
            unsafe {
                std::ptr::write_bytes(host, 0, len);
            }
            SyscallResult::Continue(a1)
        }
        SYS_RSEQ => SyscallResult::Continue(ENOSYS),
        SYS_FUTEX => do_futex(space, a0, a1, a2),
        SYS_TGKILL => {
            if a2 == 6 {
                SyscallResult::Exit(128 + 6)
            } else {
                SyscallResult::Continue(0)
            }
        }
        SYS_WRITEV => do_writev(space, a0, a1, a2),
        SYS_READV => do_readv(space, a0, a1, a2),
        SYS_IOCTL => do_ioctl(a0, a1, a2),
        SYS_FCNTL => do_fcntl(a0, a1, a2),
        SYS_DUP => do_dup(a0),
        SYS_DUP3 => do_dup3(a0, a1, a2),
        SYS_GETCWD => do_getcwd(space, a0, a1),
        SYS_PIPE2 => do_pipe2(space, a0, a1),
        SYS_UNLINKAT => do_unlinkat(space, a0, a1, a2),
        SYS_FSTAT => do_fstat(space, a0, a1),
        SYS_FSTATFS => do_fstatfs(space, a0, a1),
        SYS_NEWFSTATAT => do_newfstatat(space, a0, a1, a2, a3),
        SYS_OPENAT => do_openat(space, a0, a1, a2, a3),
        SYS_CLOSE => do_close(a0),
        SYS_READ => do_read(space, a0, a1, a2),
        SYS_LSEEK => do_lseek(a0, a1, a2),
        SYS_FACCESSAT => do_faccessat(space, a0, a1, a2),
        SYS_PRLIMIT64 => do_prlimit64(space, a0, a1, a2, a3),
        SYS_UNAME => do_uname(space, a0),
        SYS_READLINKAT => do_readlinkat(space, a0, a1, a2, a3, elf_path),
        SYS_CLOCK_GETTIME => do_clock_gettime(space, a0, a1),
        SYS_CLOCK_NANOSLEEP => do_clock_nanosleep(space, a0, a1, a2, a3),
        SYS_CLONE => do_clone(space, a0, a1, a2, a3, a4),
        SYS_WAIT4 => do_wait4(space, a0, a1, a2, a3),
        SYS_EXECVE => SyscallResult::Continue(ENOSYS),
        _ => {
            eprintln!("[tcg] unknown syscall {nr} → -ENOSYS");
            SyscallResult::Continue(ENOSYS)
        }
    };
    if std::env::var("TCG_SYSCALL").is_ok() {
        match result {
            SyscallResult::Continue(ret) => {
                eprintln!("[syscall] nr={nr} ret={ret:#x}");
            }
            SyscallResult::Exit(code) => {
                eprintln!("[syscall] nr={nr} exit={code}");
            }
        }
    }
    result
}

fn do_fcntl(fd: u64, cmd: u64, arg: u64) -> SyscallResult {
    let ret =
        unsafe { libc::fcntl(fd as i32, cmd as i32, arg as libc::c_long) };
    if ret < 0 {
        SyscallResult::Continue(errno_ret())
    } else {
        SyscallResult::Continue(ret as u64)
    }
}

fn do_dup(fd: u64) -> SyscallResult {
    let ret = unsafe { libc::dup(fd as i32) };
    if ret < 0 {
        SyscallResult::Continue(errno_ret())
    } else {
        SyscallResult::Continue(ret as u64)
    }
}

fn do_dup3(oldfd: u64, newfd: u64, flags: u64) -> SyscallResult {
    let ret = unsafe { libc::dup3(oldfd as i32, newfd as i32, flags as i32) };
    if ret < 0 {
        SyscallResult::Continue(errno_ret())
    } else {
        SyscallResult::Continue(ret as u64)
    }
}

fn do_pipe2(
    space: &mut GuestSpace,
    pipefd_addr: u64,
    flags: u64,
) -> SyscallResult {
    let mut fds = [0i32; 2];
    let ret = unsafe { libc::pipe2(fds.as_mut_ptr(), flags as i32) };
    if ret < 0 {
        return SyscallResult::Continue(errno_ret());
    }
    let fd0 = (fds[0] as u32).to_le_bytes();
    let fd1 = (fds[1] as u32).to_le_bytes();
    unsafe {
        space.write_bytes(pipefd_addr, &fd0);
        space.write_bytes(pipefd_addr + 4, &fd1);
    }
    SyscallResult::Continue(0)
}

fn do_clone(
    _space: &mut GuestSpace,
    flags: u64,
    _child_stack: u64,
    _ptid: u64,
    _tls: u64,
    _ctid: u64,
) -> SyscallResult {
    // Minimal clone support for fork-like use in glibc/perl:
    // clone(flags=SIGCHLD|CLONE_CHILD_{SET,CLEAR}TID, child_stack=0, ...)
    // We map this to host fork().
    let sigchld = libc::SIGCHLD as u64;
    if (flags & sigchld) == sigchld {
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            SyscallResult::Continue(errno_ret())
        } else {
            SyscallResult::Continue(pid as u64)
        }
    } else {
        SyscallResult::Continue(ENOSYS)
    }
}

fn do_wait4(
    space: &mut GuestSpace,
    pid: u64,
    status_addr: u64,
    options: u64,
    rusage_addr: u64,
) -> SyscallResult {
    if rusage_addr != 0 {
        return SyscallResult::Continue(ENOSYS);
    }
    let mut status: i32 = 0;
    let ret = unsafe {
        libc::wait4(
            pid as i32,
            if status_addr != 0 {
                &mut status
            } else {
                std::ptr::null_mut()
            },
            options as i32,
            std::ptr::null_mut(),
        )
    };
    if ret < 0 {
        return SyscallResult::Continue(errno_ret());
    }
    if status_addr != 0 {
        let st = (status as u32).to_le_bytes();
        unsafe {
            space.write_bytes(status_addr, &st);
        }
    }
    SyscallResult::Continue(ret as u64)
}

fn do_clock_nanosleep(
    space: &mut GuestSpace,
    clockid: u64,
    flags: u64,
    req_addr: u64,
    rem_addr: u64,
) -> SyscallResult {
    let req_sec = unsafe { space.read_u64(req_addr) } as i64;
    let req_nsec = unsafe { space.read_u64(req_addr + 8) } as i64;
    let req = libc::timespec {
        tv_sec: req_sec as libc::time_t,
        tv_nsec: req_nsec as libc::c_long,
    };
    let mut rem = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let rem_ptr = if rem_addr != 0 {
        &mut rem as *mut libc::timespec
    } else {
        std::ptr::null_mut()
    };
    let ret = unsafe {
        libc::clock_nanosleep(
            clockid as libc::clockid_t,
            flags as i32,
            &req as *const libc::timespec,
            rem_ptr,
        )
    };
    if ret != 0 {
        if rem_addr != 0 {
            let rem_sec = (rem.tv_sec as i64 as u64).to_le_bytes();
            let rem_nsec = (rem.tv_nsec as i64 as u64).to_le_bytes();
            unsafe {
                space.write_bytes(rem_addr, &rem_sec);
                space.write_bytes(rem_addr + 8, &rem_nsec);
            }
        }
        SyscallResult::Continue((-(ret as i64)) as u64)
    } else {
        SyscallResult::Continue(0)
    }
}

fn errno_ret() -> u64 {
    let e = unsafe { *libc::__errno_location() };
    (-e as i64) as u64
}

fn host_gettid() -> i64 {
    unsafe { libc::syscall(libc::SYS_gettid) as i64 }
}

fn do_brk(space: &mut GuestSpace, addr: u64) -> SyscallResult {
    let old = space.brk();
    if addr == 0 {
        return SyscallResult::Continue(old);
    }

    if addr >= old {
        let new_brk = crate::guest_space::page_align_up(addr);
        let old_aligned = crate::guest_space::page_align_up(old);
        if new_brk > old_aligned {
            let sz = (new_brk - old_aligned) as usize;
            if space
                .mmap_fixed(old_aligned, sz, libc::PROT_READ | libc::PROT_WRITE)
                .is_err()
            {
                return SyscallResult::Continue(old);
            }
        }
        space.set_brk(addr);
        return SyscallResult::Continue(addr);
    }

    // Shrinking brk always succeeds. Make full pages above the new break
    // inaccessible to catch stale accesses while preserving sub-page tail.
    let old_aligned = crate::guest_space::page_align_up(old);
    let new_aligned = crate::guest_space::page_align_up(addr);
    if old_aligned > new_aligned {
        let _ = space.mprotect(
            new_aligned,
            (old_aligned - new_aligned) as usize,
            libc::PROT_NONE,
        );
    }
    space.set_brk(addr);
    SyscallResult::Continue(addr)
}

fn do_mmap(
    space: &mut GuestSpace,
    addr: u64,
    len: u64,
    prot: u64,
    mmap_next: &mut u64,
) -> SyscallResult {
    let prot = prot as i32;
    let aligned_len = crate::guest_space::page_align_up(len) as usize;
    let guest_addr = if addr != 0 {
        addr
    } else {
        let a = *mmap_next;
        *mmap_next += aligned_len as u64;
        a
    };
    match space.mmap_fixed(guest_addr, aligned_len, prot) {
        Ok(()) => SyscallResult::Continue(guest_addr),
        Err(_) => SyscallResult::Continue((-12i64) as u64),
    }
}

fn do_mremap(
    space: &mut GuestSpace,
    old_addr: u64,
    old_len: u64,
    new_len: u64,
    flags: u64,
    new_addr_arg: u64,
    mmap_next: &mut u64,
) -> SyscallResult {
    const MREMAP_MAYMOVE: u64 = 1;
    const MREMAP_FIXED: u64 = 2;

    let old_len = crate::guest_space::page_align_up(old_len);
    let new_len = crate::guest_space::page_align_up(new_len);
    if old_len == 0 || new_len == 0 {
        return SyscallResult::Continue(EINVAL);
    }

    if new_len <= old_len {
        return SyscallResult::Continue(old_addr);
    }

    let grow_from = old_addr.saturating_add(old_len);
    let grow_len = (new_len - old_len) as usize;
    if space
        .mmap_fixed(grow_from, grow_len, libc::PROT_READ | libc::PROT_WRITE)
        .is_ok()
    {
        return SyscallResult::Continue(old_addr);
    }

    if (flags & MREMAP_MAYMOVE) == 0 {
        return SyscallResult::Continue(ENOMEM);
    }

    let new_addr = if (flags & MREMAP_FIXED) != 0 {
        new_addr_arg
    } else {
        let addr = *mmap_next;
        *mmap_next += new_len;
        addr
    };

    if space
        .mmap_fixed(
            new_addr,
            new_len as usize,
            libc::PROT_READ | libc::PROT_WRITE,
        )
        .is_err()
    {
        return SyscallResult::Continue(ENOMEM);
    }

    unsafe {
        std::ptr::copy_nonoverlapping(
            space.g2h(old_addr) as *const u8,
            space.g2h(new_addr),
            old_len as usize,
        );
    }

    SyscallResult::Continue(new_addr)
}

fn do_writev(
    space: &mut GuestSpace,
    fd: u64,
    iov_addr: u64,
    iovcnt: u64,
) -> SyscallResult {
    let fd = fd as i32;
    let cnt = iovcnt as usize;
    let mut total: usize = 0;
    for i in 0..cnt {
        let entry = iov_addr + (i as u64) * 16;
        let base = unsafe { *(space.g2h(entry) as *const u64) };
        let len = unsafe { *(space.g2h(entry + 8) as *const u64) } as usize;
        if len == 0 {
            continue;
        }
        let host = space.g2h(base);
        let ret = unsafe { libc::write(fd, host as *const libc::c_void, len) };
        if ret < 0 {
            return SyscallResult::Continue(errno_ret());
        }
        total += ret as usize;
    }
    SyscallResult::Continue(total as u64)
}

fn do_fstat(space: &mut GuestSpace, fd: u64, buf_addr: u64) -> SyscallResult {
    let fd = fd as i32;
    let host_buf = space.g2h(buf_addr);
    unsafe {
        std::ptr::write_bytes(host_buf, 0, 128);
    }
    if (0..=2).contains(&fd) {
        let mode: u32 = 0o020666;
        unsafe {
            let p = host_buf.add(16) as *mut u32;
            p.write_unaligned(mode);
        }
        SyscallResult::Continue(0)
    } else {
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        let ret = unsafe { libc::fstat(fd, &mut st) };
        if ret < 0 {
            return SyscallResult::Continue(errno_ret());
        }
        // AArch64 struct stat layout (same as RISC-V)
        unsafe {
            let p = host_buf;
            *(p as *mut u64) = st.st_dev;
            *(p.add(8) as *mut u64) = st.st_ino;
            *(p.add(16) as *mut u32) = st.st_mode;
            *(p.add(20) as *mut u32) = st.st_nlink as u32;
            *(p.add(24) as *mut u32) = st.st_uid;
            *(p.add(28) as *mut u32) = st.st_gid;
            *(p.add(32) as *mut u64) = st.st_rdev;
            *(p.add(48) as *mut i64) = st.st_size;
            *(p.add(56) as *mut i32) = st.st_blksize as i32;
            *(p.add(64) as *mut i64) = st.st_blocks;
            *(p.add(72) as *mut i64) = st.st_atime;
            *(p.add(80) as *mut i64) = st.st_atime_nsec;
            *(p.add(88) as *mut i64) = st.st_mtime;
            *(p.add(96) as *mut i64) = st.st_mtime_nsec;
            *(p.add(104) as *mut i64) = st.st_ctime;
            *(p.add(112) as *mut i64) = st.st_ctime_nsec;
        }
        SyscallResult::Continue(0)
    }
}

fn do_prlimit64(
    space: &mut GuestSpace,
    _pid: u64,
    resource: u64,
    _new_rlim: u64,
    old_rlim: u64,
) -> SyscallResult {
    const RLIMIT_STACK: u64 = 3;
    const RLIM_INFINITY: u64 = u64::MAX;
    if old_rlim != 0 {
        let p = space.g2h(old_rlim);
        if resource == RLIMIT_STACK {
            unsafe {
                *(p as *mut u64) = 8 * 1024 * 1024;
                *(p.add(8) as *mut u64) = RLIM_INFINITY;
            }
        } else {
            let mut rl: libc::rlimit = unsafe { std::mem::zeroed() };
            let ret = unsafe {
                libc::getrlimit(resource as libc::__rlimit_resource_t, &mut rl)
            };
            if ret < 0 {
                return SyscallResult::Continue(errno_ret());
            }
            unsafe {
                *(p as *mut u64) = rl.rlim_cur;
                *(p.add(8) as *mut u64) = rl.rlim_max;
            }
        }
    }
    SyscallResult::Continue(0)
}

fn do_getcwd(
    space: &mut GuestSpace,
    buf_addr: u64,
    size: u64,
) -> SyscallResult {
    if buf_addr == 0 || size == 0 {
        return SyscallResult::Continue(EINVAL);
    }
    let host_buf = space.g2h(buf_addr) as *mut libc::c_char;
    let ret = unsafe {
        libc::syscall(libc::SYS_getcwd, host_buf, size as libc::size_t)
    };
    if ret < 0 {
        SyscallResult::Continue(errno_ret())
    } else {
        SyscallResult::Continue(ret as u64)
    }
}

fn do_uname(space: &mut GuestSpace, buf_addr: u64) -> SyscallResult {
    let p = space.g2h(buf_addr);
    unsafe {
        std::ptr::write_bytes(p, 0, 390);
    }
    let fields: [&[u8]; 6] = [
        b"Linux",   // sysname
        b"tcg-rs",  // nodename
        b"6.1.0",   // release
        b"#1 SMP",  // version
        b"aarch64", // machine
        b"(none)",  // domainname
    ];
    for (i, val) in fields.iter().enumerate() {
        let dst = unsafe { p.add(i * 65) };
        let len = val.len().min(64);
        unsafe {
            std::ptr::copy_nonoverlapping(val.as_ptr(), dst, len);
        }
    }
    SyscallResult::Continue(0)
}

fn do_readlinkat(
    space: &mut GuestSpace,
    dirfd: u64,
    path_addr: u64,
    buf_addr: u64,
    bufsiz: u64,
    elf_path: &str,
) -> SyscallResult {
    let host_path = space.g2h(path_addr);
    let path = unsafe { std::ffi::CStr::from_ptr(host_path as *const i8) };
    let path_bytes = path.to_bytes();
    if path_bytes == b"/proc/self/exe" {
        let elf = elf_path.as_bytes();
        let len = elf.len().min(bufsiz as usize);
        let dst = space.g2h(buf_addr);
        unsafe {
            std::ptr::copy_nonoverlapping(elf.as_ptr(), dst, len);
        }
        SyscallResult::Continue(len as u64)
    } else {
        let dst = space.g2h(buf_addr) as *mut libc::c_char;
        let ret = unsafe {
            libc::readlinkat(
                dirfd as i32,
                path.as_ptr(),
                dst,
                bufsiz as libc::size_t,
            )
        };
        if ret < 0 {
            SyscallResult::Continue(errno_ret())
        } else {
            SyscallResult::Continue(ret as u64)
        }
    }
}

fn do_clock_gettime(
    space: &mut GuestSpace,
    clk_id: u64,
    tp_addr: u64,
) -> SyscallResult {
    let mut ts: libc::timespec = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::clock_gettime(clk_id as i32, &mut ts) };
    if ret < 0 {
        return SyscallResult::Continue(errno_ret());
    }
    let p = space.g2h(tp_addr);
    unsafe {
        *(p as *mut i64) = ts.tv_sec;
        *(p.add(8) as *mut i64) = ts.tv_nsec;
    }
    SyscallResult::Continue(0)
}

fn do_unlinkat(
    space: &mut GuestSpace,
    dirfd: u64,
    path_addr: u64,
    flags: u64,
) -> SyscallResult {
    let host_path = space.g2h(path_addr);
    let path = unsafe { std::ffi::CStr::from_ptr(host_path as *const i8) };
    let ret =
        unsafe { libc::unlinkat(dirfd as i32, path.as_ptr(), flags as i32) };
    if ret < 0 {
        SyscallResult::Continue(errno_ret())
    } else {
        SyscallResult::Continue(0)
    }
}

fn do_futex(
    space: &mut GuestSpace,
    uaddr: u64,
    op: u64,
    _val: u64,
) -> SyscallResult {
    const FUTEX_CMD_MASK: u64 = 0x7f;
    const FUTEX_WAIT: u64 = 0;
    const FUTEX_WAKE: u64 = 1;
    const EAGAIN: u64 = (-11i64) as u64;
    let _ = space.g2h(uaddr);

    match op & FUTEX_CMD_MASK {
        FUTEX_WAIT => SyscallResult::Continue(EAGAIN),
        FUTEX_WAKE => SyscallResult::Continue(0),
        _ => SyscallResult::Continue(ENOSYS),
    }
}

// ---------------------------------------------------------------
// openat(dirfd, pathname, flags, mode)
// ---------------------------------------------------------------

fn do_openat(
    space: &mut GuestSpace,
    dirfd: u64,
    path_addr: u64,
    flags: u64,
    mode: u64,
) -> SyscallResult {
    let host_path = space.g2h(path_addr);
    let path = unsafe { std::ffi::CStr::from_ptr(host_path as *const i8) };
    let fd = unsafe {
        libc::openat(
            dirfd as i32,
            path.as_ptr(),
            flags as i32,
            mode as libc::mode_t,
        )
    };
    if fd < 0 {
        SyscallResult::Continue(errno_ret())
    } else {
        SyscallResult::Continue(fd as u64)
    }
}

// ---------------------------------------------------------------
// close(fd)
// ---------------------------------------------------------------

fn do_close(fd: u64) -> SyscallResult {
    let fd = fd as i32;
    // Don't close stdio
    if fd <= 2 {
        return SyscallResult::Continue(0);
    }
    let ret = unsafe { libc::close(fd) };
    if ret < 0 {
        SyscallResult::Continue(errno_ret())
    } else {
        SyscallResult::Continue(0)
    }
}

// ---------------------------------------------------------------
// read(fd, buf, count)
// ---------------------------------------------------------------

fn do_read(
    space: &mut GuestSpace,
    fd: u64,
    buf_addr: u64,
    count: u64,
) -> SyscallResult {
    let fd = fd as i32;
    let len = count as usize;
    let host_buf = space.g2h(buf_addr);
    let ret = unsafe { libc::read(fd, host_buf as *mut libc::c_void, len) };
    if ret < 0 {
        SyscallResult::Continue(errno_ret())
    } else {
        SyscallResult::Continue(ret as u64)
    }
}

// ---------------------------------------------------------------
// readv(fd, iov, iovcnt)
// ---------------------------------------------------------------

fn do_readv(
    space: &mut GuestSpace,
    fd: u64,
    iov_addr: u64,
    iovcnt: u64,
) -> SyscallResult {
    let fd = fd as i32;
    let cnt = iovcnt as usize;
    let mut total: usize = 0;
    for i in 0..cnt {
        let entry = iov_addr + (i as u64) * 16;
        let base = unsafe { *(space.g2h(entry) as *const u64) };
        let len = unsafe { *(space.g2h(entry + 8) as *const u64) } as usize;
        if len == 0 {
            continue;
        }
        let host = space.g2h(base);
        let ret = unsafe { libc::read(fd, host as *mut libc::c_void, len) };
        if ret < 0 {
            return SyscallResult::Continue(errno_ret());
        }
        total += ret as usize;
        if (ret as usize) < len {
            break;
        }
    }
    SyscallResult::Continue(total as u64)
}

// ---------------------------------------------------------------
// lseek(fd, offset, whence)
// ---------------------------------------------------------------

fn do_lseek(fd: u64, offset: u64, whence: u64) -> SyscallResult {
    let ret = unsafe { libc::lseek(fd as i32, offset as i64, whence as i32) };
    if ret < 0 {
        SyscallResult::Continue(errno_ret())
    } else {
        SyscallResult::Continue(ret as u64)
    }
}

// ---------------------------------------------------------------
// newfstatat / fstatat64(dirfd, pathname, statbuf, flags)
// ---------------------------------------------------------------

fn do_newfstatat(
    space: &mut GuestSpace,
    dirfd: u64,
    path_addr: u64,
    buf_addr: u64,
    flags: u64,
) -> SyscallResult {
    let host_path = space.g2h(path_addr);
    let path = unsafe { std::ffi::CStr::from_ptr(host_path as *const i8) };
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let ret = unsafe {
        libc::fstatat(dirfd as i32, path.as_ptr(), &mut st, flags as i32)
    };
    if ret < 0 {
        return SyscallResult::Continue(errno_ret());
    }
    let p = space.g2h(buf_addr);
    unsafe {
        std::ptr::write_bytes(p, 0, 128);
        *(p as *mut u64) = st.st_dev;
        *(p.add(8) as *mut u64) = st.st_ino;
        *(p.add(16) as *mut u32) = st.st_mode;
        *(p.add(20) as *mut u32) = st.st_nlink as u32;
        *(p.add(24) as *mut u32) = st.st_uid;
        *(p.add(28) as *mut u32) = st.st_gid;
        *(p.add(32) as *mut u64) = st.st_rdev;
        *(p.add(48) as *mut i64) = st.st_size;
        *(p.add(56) as *mut i32) = st.st_blksize as i32;
        *(p.add(64) as *mut i64) = st.st_blocks;
        *(p.add(72) as *mut i64) = st.st_atime;
        *(p.add(80) as *mut i64) = st.st_atime_nsec;
        *(p.add(88) as *mut i64) = st.st_mtime;
        *(p.add(96) as *mut i64) = st.st_mtime_nsec;
        *(p.add(104) as *mut i64) = st.st_ctime;
        *(p.add(112) as *mut i64) = st.st_ctime_nsec;
    }
    SyscallResult::Continue(0)
}

// ---------------------------------------------------------------
// fstatfs(fd, buf)
// ---------------------------------------------------------------

fn do_fstatfs(space: &mut GuestSpace, fd: u64, buf_addr: u64) -> SyscallResult {
    let mut st: libc::statfs = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::fstatfs(fd as i32, &mut st) };
    if ret < 0 {
        return SyscallResult::Continue(errno_ret());
    }
    // AArch64 struct statfs64 layout (120 bytes)
    let p = space.g2h(buf_addr);
    unsafe {
        std::ptr::write_bytes(p, 0, 120);
        *(p as *mut i64) = st.f_type;
        *(p.add(8) as *mut i64) = st.f_bsize;
        *(p.add(16) as *mut i64) = st.f_blocks as i64;
        *(p.add(24) as *mut i64) = st.f_bfree as i64;
        *(p.add(32) as *mut i64) = st.f_bavail as i64;
        *(p.add(40) as *mut i64) = st.f_files as i64;
        *(p.add(48) as *mut i64) = st.f_ffree as i64;
        // f_fsid at 56 (8 bytes), f_namelen at 64, f_frsize at 72
        *(p.add(64) as *mut i64) = st.f_namelen;
        *(p.add(72) as *mut i64) = st.f_frsize;
    }
    SyscallResult::Continue(0)
}

// ---------------------------------------------------------------
// faccessat(dirfd, pathname, mode)
// ---------------------------------------------------------------

fn do_faccessat(
    space: &mut GuestSpace,
    dirfd: u64,
    path_addr: u64,
    mode: u64,
) -> SyscallResult {
    let host_path = space.g2h(path_addr);
    let path = unsafe { std::ffi::CStr::from_ptr(host_path as *const i8) };
    let ret =
        unsafe { libc::faccessat(dirfd as i32, path.as_ptr(), mode as i32, 0) };
    if ret < 0 {
        SyscallResult::Continue(errno_ret())
    } else {
        SyscallResult::Continue(0)
    }
}

// ---------------------------------------------------------------
// ioctl(fd, cmd, arg)
// ---------------------------------------------------------------

fn do_ioctl(fd: u64, _cmd: u64, _arg: u64) -> SyscallResult {
    let fd = fd as i32;
    // TCGETS/TIOCGWINSZ on non-tty → ENOTTY
    if fd > 2 {
        return SyscallResult::Continue(ENOTTY);
    }
    // For stdio, also return ENOTTY (we're not a terminal)
    SyscallResult::Continue(ENOTTY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_handle_getcwd_syscall() {
        let mut space = GuestSpace::new().expect("guest space");
        let guest_buf = 0x10000u64;
        let map_len = 4096usize;
        space
            .mmap_fixed(guest_buf, map_len, libc::PROT_READ | libc::PROT_WRITE)
            .expect("map getcwd buffer");
        let mut regs = [0u64; 31];
        regs[8] = SYS_GETCWD;
        regs[0] = guest_buf;
        regs[1] = map_len as u64;
        let mut sp = 0u64;
        let mut mmap_next = 0u64;
        let res = handle_syscall_aarch64(
            &mut space,
            &mut regs,
            &mut sp,
            &mut mmap_next,
            "/tmp/guest",
        );
        let len = match res {
            SyscallResult::Continue(v) => v as usize,
            SyscallResult::Exit(code) => panic!("unexpected exit: {code}"),
        };
        assert!(len > 1 && len <= map_len);
        let host = space.g2h(guest_buf);
        let bytes = unsafe { std::slice::from_raw_parts(host, len) };
        assert_eq!(bytes[len - 1], 0);
    }

    #[test]
    fn test_readlinkat_proc_self_exe() {
        let mut space = GuestSpace::new().expect("guest space");
        let guest_base = 0x20000u64;
        space
            .mmap_fixed(guest_base, 4096, libc::PROT_READ | libc::PROT_WRITE)
            .expect("map readlinkat buffers");

        let path_addr = guest_base;
        let buf_addr = guest_base + 0x100;
        unsafe {
            space.write_bytes(path_addr, b"/proc/self/exe\0");
        }

        let res = do_readlinkat(
            &mut space,
            libc::AT_FDCWD as u64,
            path_addr,
            buf_addr,
            256,
            "/tmp/fake-elf",
        );
        let len = match res {
            SyscallResult::Continue(v) => v as usize,
            SyscallResult::Exit(code) => panic!("unexpected exit: {code}"),
        };
        assert_eq!(len, "/tmp/fake-elf".len());
        let got =
            unsafe { std::slice::from_raw_parts(space.g2h(buf_addr), len) };
        assert_eq!(got, b"/tmp/fake-elf");
    }

    #[test]
    fn test_readlinkat_symlink_passthrough() {
        let mut space = GuestSpace::new().expect("guest space");
        let guest_base = 0x30000u64;
        space
            .mmap_fixed(guest_base, 8192, libc::PROT_READ | libc::PROT_WRITE)
            .expect("map readlinkat buffers");

        let uniq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "tcgrs-a64-readlinkat-{}-{uniq}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        let target = dir.join("target.txt");
        let link = dir.join("link.txt");
        fs::write(&target, b"x").expect("write target");
        symlink("target.txt", &link).expect("create symlink");

        let path = format!("{}\0", link.to_string_lossy());
        let path_addr = guest_base;
        let buf_addr = guest_base + 0x400;
        unsafe {
            space.write_bytes(path_addr, path.as_bytes());
        }

        let res = do_readlinkat(
            &mut space,
            libc::AT_FDCWD as u64,
            path_addr,
            buf_addr,
            256,
            "/tmp/fake-elf",
        );
        let len = match res {
            SyscallResult::Continue(v) => v as usize,
            SyscallResult::Exit(code) => panic!("unexpected exit: {code}"),
        };
        assert_eq!(len, "target.txt".len());
        let got =
            unsafe { std::slice::from_raw_parts(space.g2h(buf_addr), len) };
        assert_eq!(got, b"target.txt");

        let _ = fs::remove_file(&link);
        let _ = fs::remove_file(&target);
        let _ = fs::remove_dir(&dir);
    }
}
