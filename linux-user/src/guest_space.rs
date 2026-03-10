use std::collections::BTreeMap;
use std::io;
use std::ptr;

/// Guest address space size: 4 GiB.
const GUEST_SPACE_SIZE: usize = 4 * (1 << 30);

/// Default guest stack top address.
pub const GUEST_STACK_TOP: u64 = 0xC000_0000;

/// Default guest stack size: usize = 8 MiB.
pub const GUEST_STACK_SIZE: usize = 8 * 1024 * 1024;

/// mmap-based guest address space.
///
/// Reserves a contiguous region of host memory and maps
/// guest addresses as offsets within it.
pub struct GuestSpace {
    base: *mut u8,
    size: usize,
    brk: u64,
    mapped: BTreeMap<u64, u64>,
}

// SAFETY: GuestSpace owns its mmap'd memory exclusively.
unsafe impl Send for GuestSpace {}

impl GuestSpace {
    /// Reserve a 1 GiB guest address space.
    pub fn new() -> io::Result<Self> {
        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                GUEST_SPACE_SIZE,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                -1,
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            base: ptr as *mut u8,
            size: GUEST_SPACE_SIZE,
            brk: 0,
            mapped: BTreeMap::new(),
        })
    }

    #[inline]
    pub fn g2h(&self, guest_addr: u64) -> *mut u8 {
        assert!(
            (guest_addr as usize) < self.size,
            "guest addr {guest_addr:#x} out of range"
        );
        unsafe { self.base.add(guest_addr as usize) }
    }

    #[inline]
    pub fn h2g(&self, host_ptr: *const u8) -> u64 {
        let off = host_ptr as usize - self.base as usize;
        assert!(off < self.size, "host pointer not in guest space");
        off as u64
    }

    #[inline]
    pub fn guest_base(&self) -> *const u8 {
        self.base as *const u8
    }

    #[inline]
    pub fn brk(&self) -> u64 {
        self.brk
    }

    #[inline]
    pub fn set_brk(&mut self, brk: u64) {
        self.brk = brk;
    }

    fn range_end(guest_addr: u64, size: usize) -> u64 {
        guest_addr.saturating_add(size as u64)
    }

    pub(crate) fn remove_mapped_range(&mut self, guest_addr: u64, size: usize) {
        let end = Self::range_end(guest_addr, size);
        let overlaps: Vec<(u64, u64)> = self
            .mapped
            .range(..end)
            .filter_map(|(&start, &mapped_end)| {
                if mapped_end > guest_addr {
                    Some((start, mapped_end))
                } else {
                    None
                }
            })
            .collect();

        for (start, mapped_end) in overlaps {
            self.mapped.remove(&start);
            if start < guest_addr {
                self.mapped.insert(start, guest_addr);
            }
            if mapped_end > end {
                self.mapped.insert(end, mapped_end);
            }
        }
    }

    pub(crate) fn insert_mapped_range(&mut self, guest_addr: u64, size: usize) {
        if size == 0 {
            return;
        }
        let end = Self::range_end(guest_addr, size);
        self.remove_mapped_range(guest_addr, size);
        self.mapped.insert(guest_addr, end);
    }

    pub fn range_is_free(&self, guest_addr: u64, size: usize) -> bool {
        if size == 0 {
            return true;
        }
        let end = Self::range_end(guest_addr, size);
        self.mapped
            .range(..end)
            .next_back()
            .is_none_or(|(_, &mapped_end)| mapped_end <= guest_addr)
    }

    pub fn find_free_range_top_down(
        &self,
        floor: u64,
        ceiling: u64,
        size: usize,
    ) -> Option<u64> {
        if size == 0 {
            return None;
        }
        let size = size as u64;
        let mut cursor = page_align_down(ceiling);
        let floor = page_align_up(floor);
        if cursor < floor.saturating_add(size) {
            return None;
        }

        for (&start, &end) in self.mapped.range(..cursor).rev() {
            if end <= floor {
                break;
            }
            if cursor >= end.saturating_add(size) {
                return Some(cursor - size);
            }
            cursor = cursor.min(start);
            if cursor < floor.saturating_add(size) {
                return None;
            }
        }

        if cursor >= floor.saturating_add(size) {
            Some(cursor - size)
        } else {
            None
        }
    }

    pub fn next_mapped_addr(&self, guest_addr: u64) -> Option<u64> {
        self.mapped.range(guest_addr..).next().map(|(&start, _)| start)
    }

    pub fn mmap_fixed(
        &mut self,
        guest_addr: u64,
        size: usize,
        prot: i32,
    ) -> io::Result<()> {
        let host = self.g2h(guest_addr);
        let ret = unsafe {
            libc::mmap(
                host as *mut libc::c_void,
                size,
                prot,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_FIXED,
                -1,
                0,
            )
        };
        if ret == libc::MAP_FAILED {
            Err(io::Error::last_os_error())
        } else {
            if prot == libc::PROT_NONE {
                self.remove_mapped_range(guest_addr, size);
            } else {
                self.insert_mapped_range(guest_addr, size);
            }
            Ok(())
        }
    }

    pub fn mmap_fixed_host(
        &mut self,
        guest_addr: u64,
        size: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> io::Result<()> {
        let host = self.g2h(guest_addr);
        let ret = unsafe {
            libc::mmap(
                host as *mut libc::c_void,
                size,
                prot,
                flags | libc::MAP_FIXED,
                fd,
                offset as libc::off_t,
            )
        };
        if ret == libc::MAP_FAILED {
            Err(io::Error::last_os_error())
        } else {
            if prot == libc::PROT_NONE {
                self.remove_mapped_range(guest_addr, size);
            } else {
                self.insert_mapped_range(guest_addr, size);
            }
            Ok(())
        }
    }

    pub fn munmap_fixed(&mut self, guest_addr: u64, size: usize) -> io::Result<()> {
        let host = self.g2h(guest_addr);
        let ret = unsafe {
            libc::mmap(
                host as *mut libc::c_void,
                size,
                libc::PROT_NONE,
                libc::MAP_PRIVATE
                    | libc::MAP_ANONYMOUS
                    | libc::MAP_FIXED
                    | libc::MAP_NORESERVE,
                -1,
                0,
            )
        };
        if ret == libc::MAP_FAILED {
            Err(io::Error::last_os_error())
        } else {
            self.remove_mapped_range(guest_addr, size);
            Ok(())
        }
    }

    pub fn mprotect(
        &mut self,
        guest_addr: u64,
        size: usize,
        prot: i32,
    ) -> io::Result<()> {
        let host = self.g2h(guest_addr);
        let ret = unsafe { libc::mprotect(host as *mut libc::c_void, size, prot) };
        if ret != 0 {
            Err(io::Error::last_os_error())
        } else {
            if prot == libc::PROT_NONE {
                self.remove_mapped_range(guest_addr, size);
            } else {
                self.insert_mapped_range(guest_addr, size);
            }
            Ok(())
        }
    }

    pub unsafe fn write_bytes(&self, guest_addr: u64, data: &[u8]) {
        let dst = self.g2h(guest_addr);
        ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
    }

    pub unsafe fn write_u64(&self, guest_addr: u64, val: u64) {
        let dst = self.g2h(guest_addr);
        (dst as *mut u64).write_unaligned(val);
    }

    pub unsafe fn write_u8(&self, guest_addr: u64, val: u8) {
        let dst = self.g2h(guest_addr);
        *(dst as *mut u8) = val;
    }

    pub unsafe fn read_u64(&self, guest_addr: u64) -> u64 {
        let src = self.g2h(guest_addr);
        (src as *const u64).read_unaligned()
    }
}

impl Drop for GuestSpace {
    fn drop(&mut self) {
        if !self.base.is_null() {
            unsafe {
                libc::munmap(self.base as *mut libc::c_void, self.size);
            }
        }
    }
}

pub fn page_size() -> usize {
    let size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if size <= 0 {
        4096
    } else {
        size as usize
    }
}

pub fn page_align_up(addr: u64) -> u64 {
    let ps = page_size() as u64;
    (addr + ps - 1) & !(ps - 1)
}

pub fn page_align_down(addr: u64) -> u64 {
    let ps = page_size() as u64;
    addr & !(ps - 1)
}
