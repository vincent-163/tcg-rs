use crate::elf::{Elf64Ehdr, Elf64Phdr, ElfError, EM_AARCH64, ET_DYN, PF_R, PF_W, PF_X, PT_LOAD};
use crate::guest_space::{page_align_down, page_align_up, page_size, GuestSpace, GUEST_STACK_SIZE, GUEST_STACK_TOP};

const AARCH64_VDSO_BYTES: &[u8] =
    include_bytes!("../vdso/aarch64/linux-vdso.so.1");

pub fn map_guest_vdso(
    space: &GuestSpace,
    machine: u16,
) -> Result<Option<u64>, ElfError> {
    if machine != EM_AARCH64 {
        return Ok(None);
    }

    let data = AARCH64_VDSO_BYTES;
    let ehdr = Elf64Ehdr::from_bytes(data)?;
    if ehdr.e_type != ET_DYN {
        return Err(ElfError::UnsupportedType);
    }
    ehdr.validate_machine(machine)?;
    let phdrs = ehdr.program_headers(data)?;

    let mut min_vaddr = u64::MAX;
    let mut max_vaddr = 0u64;
    let mut has_load = false;
    for ph in &phdrs {
        if ph.p_type != PT_LOAD {
            continue;
        }
        has_load = true;
        min_vaddr = min_vaddr.min(page_align_down(ph.p_vaddr));
        max_vaddr = max_vaddr.max(page_align_up(ph.p_vaddr + ph.p_memsz));
    }
    if !has_load {
        return Err(ElfError::InvalidPhdr);
    }

    let total_size = max_vaddr.checked_sub(min_vaddr).ok_or(ElfError::InvalidPhdr)?;
    let stack_base = GUEST_STACK_TOP - GUEST_STACK_SIZE as u64;
    let guard = page_size() as u64;
    let guest_base = page_align_down(
        stack_base
            .checked_sub(guard + total_size)
            .ok_or(ElfError::InvalidPhdr)?,
    );
    let load_bias = guest_base.checked_sub(min_vaddr).ok_or(ElfError::InvalidPhdr)?;

    for ph in &phdrs {
        if ph.p_type != PT_LOAD {
            continue;
        }
        map_load_segment(space, data, ph, load_bias)?;
    }

    Ok(Some(load_bias))
}

fn map_load_segment(
    space: &GuestSpace,
    data: &[u8],
    ph: &Elf64Phdr,
    load_bias: u64,
) -> Result<(), ElfError> {
    let seg_start = load_bias + ph.p_vaddr;
    let aligned_start = page_align_down(seg_start);
    let aligned_end = page_align_up(seg_start + ph.p_memsz);
    let aligned_size = aligned_end.checked_sub(aligned_start).ok_or(ElfError::InvalidPhdr)? as usize;

    space
        .mmap_fixed(aligned_start, aligned_size, libc::PROT_READ | libc::PROT_WRITE)
        .map_err(|_| ElfError::InvalidPhdr)?;

    if ph.p_filesz > 0 {
        let src_off = ph.p_offset as usize;
        let src_end = src_off.checked_add(ph.p_filesz as usize).ok_or(ElfError::InvalidPhdr)?;
        if src_end > data.len() {
            return Err(ElfError::InvalidPhdr);
        }
        unsafe {
            space.write_bytes(seg_start, &data[src_off..src_end]);
        }
    }

    let prot = elf_to_prot(ph.p_flags);
    if prot != (libc::PROT_READ | libc::PROT_WRITE) {
        space
            .mprotect(aligned_start, aligned_size, prot)
            .map_err(|_| ElfError::InvalidPhdr)?;
    }

    Ok(())
}

fn elf_to_prot(flags: u32) -> i32 {
    let mut prot = 0;
    if flags & PF_R != 0 {
        prot |= libc::PROT_READ;
    }
    if flags & PF_W != 0 {
        prot |= libc::PROT_WRITE;
    }
    if flags & PF_X != 0 {
        prot |= libc::PROT_EXEC;
    }
    prot
}
