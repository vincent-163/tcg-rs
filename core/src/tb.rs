use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

/// TB alignment — must be ≥ 8 so the low 3 bits of a TB pointer are
/// always zero and can be used to carry exit-slot information.
///
/// Mutable chaining state protected by per-TB lock.
pub struct TbJmpState {
    /// Outgoing edge: destination TB pointer for each slot.
    pub jmp_dest: [Option<*mut TranslationBlock>; 2],
    /// Incoming edges: (source_tb_ptr, slot) pairs.
    pub jmp_list: Vec<(*mut TranslationBlock, usize)>,
}

// SAFETY: TbJmpState pointers are only accessed under the TB's jmp mutex.
unsafe impl Send for TbJmpState {}
unsafe impl Sync for TbJmpState {}

impl TbJmpState {
    fn new() -> Self {
        Self {
            jmp_dest: [None; 2],
            jmp_list: Vec::new(),
        }
    }
}

/// A cached translated code block.
///
/// Maps to QEMU's `TranslationBlock`. Represents the mapping
/// from a guest code region to generated host machine code.
///
/// `#[repr(align(8))]` guarantees the low 3 bits of any `*mut
/// TranslationBlock` are zero, so we can use them to carry exit-slot
/// information (see `encode_tb_exit` / `decode_tb_exit`).
///
/// Fields above `jmp` are immutable after creation (set during
/// translation under translate_lock). The `jmp` mutex protects
/// mutable chaining state. `invalid` is atomic for lock-free checking.
#[repr(align(8))]
pub struct TranslationBlock {
    // -- Immutable after creation --
    pub pc: u64,
    pub cs_base: u64,
    pub flags: u32,
    pub cflags: u32,
    pub size: u32,
    pub icount: u16,
    pub host_offset: usize,
    pub host_size: usize,
    pub jmp_insn_offset: [Option<u32>; 2],
    pub jmp_reset_offset: [Option<u32>; 2],
    pub phys_pc: u64,
    /// Protected by TbStore hash lock.
    /// Stores the raw pointer value of the next TB in the hash bucket chain,
    /// or 0 for end-of-chain.
    pub hash_next: usize,

    // -- Per-TB lock for chaining state --
    pub jmp: Mutex<TbJmpState>,

    // -- Atomic --
    pub invalid: AtomicBool,
    /// Single-entry target cache for indirect exits (atomic, lock-free).
    /// Stores a raw `*mut TranslationBlock` as usize; 0 means no cached target.
    pub exit_target: AtomicUsize,

    // -- Profiling (atomic, embedded in TB for stable pointer) --
    /// Execution count, incremented by JIT-generated code.
    pub exec_count: AtomicU64,
    /// Set to true when this TB is reached via an indirect jump
    /// (TB_EXIT_NOCHAIN). Used to determine which TBs should be
    /// exported in AOT compilation.
    pub indirect_target: AtomicBool,
}

impl std::fmt::Debug for TranslationBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranslationBlock")
            .field("pc", &self.pc)
            .field("flags", &self.flags)
            .field("size", &self.size)
            .field("host_offset", &self.host_offset)
            .field("host_size", &self.host_size)
            .field("invalid", &self.invalid.load(Ordering::Relaxed))
            .finish()
    }
}

/// Compile flags for TranslationBlock.cflags.
pub mod cflags {
    /// Mask for the instruction count limit (0 = no limit).
    pub const CF_COUNT_MASK: u32 = 0x0000_FFFF;
    /// Last I/O instruction in the TB.
    pub const CF_LAST_IO: u32 = 0x0001_0000;
    /// TB is being single-stepped.
    pub const CF_SINGLE_STEP: u32 = 0x0002_0000;
    /// Use icount (deterministic execution).
    pub const CF_USE_ICOUNT: u32 = 0x0004_0000;
}

impl TranslationBlock {
    pub fn new(pc: u64, flags: u32, cflags: u32) -> Self {
        Self {
            pc,
            cs_base: 0,
            flags,
            cflags,
            size: 0,
            icount: 0,
            host_offset: 0,
            host_size: 0,
            jmp_insn_offset: [None; 2],
            jmp_reset_offset: [None; 2],
            phys_pc: 0,
            hash_next: 0,
            jmp: Mutex::new(TbJmpState::new()),
            invalid: AtomicBool::new(false),
            exit_target: AtomicUsize::new(0),
            exec_count: AtomicU64::new(0),
            indirect_target: AtomicBool::new(false),
        }
    }

    /// Compute hash bucket index for TB lookup.
    pub fn hash(pc: u64, flags: u32) -> usize {
        let h = pc.wrapping_mul(0x9e3779b97f4a7c15) ^ (flags as u64);
        (h as usize) & (TB_HASH_SIZE - 1)
    }

    /// Record the offset of a `goto_tb` jump instruction for exit slot `n`.
    pub fn set_jmp_insn_offset(&mut self, n: usize, offset: u32) {
        assert!(n < 2);
        self.jmp_insn_offset[n] = Some(offset);
    }

    /// Record the reset offset for exit slot `n`.
    pub fn set_jmp_reset_offset(&mut self, n: usize, offset: u32) {
        assert!(n < 2);
        self.jmp_reset_offset[n] = Some(offset);
    }

    /// Maximum number of guest instructions per TB.
    pub fn max_insns(cflags: u32) -> u32 {
        let count = cflags & cflags::CF_COUNT_MASK;
        if count == 0 {
            512
        } else {
            count
        }
    }
}

/// Number of buckets in the global TB hash table.
pub const TB_HASH_SIZE: usize = 1 << 15; // 32768

/// Number of entries in the per-CPU jump cache.
pub const TB_JMP_CACHE_SIZE: usize = 1 << 12; // 4096

/// TB exit value encoding (following QEMU `TB_EXIT_*` convention).
///
/// For **chainable** exits the return value encodes the source TB pointer
/// in the upper bits with the exit slot in the low 3 bits:
///
/// ```text
///   encoded = (tb_ptr as usize) | slot      (slot ∈ {0, 1, 2})
/// ```
///
/// Because `TranslationBlock` is `#[repr(align(8))]`, TB pointers always
/// have their low 3 bits clear, so `encoded > TB_EXIT_MAX` and the low 3
/// bits unambiguously carry the slot.
///
/// For **real** exits (ECALL, EBREAK …) the raw exit code `>= TB_EXIT_MAX`
/// is returned **without** a pointer in the upper bits.  The exec loop
/// distinguishes these from chainable exits by checking
/// `raw <= TB_EXIT_NOCHAIN_MAX` (values 0–2 are unreachable as bare values
/// because all heap pointers are far above 2).
///
/// | Value                        | Meaning                        |
/// |------------------------------|--------------------------------|
/// | `ptr | 0`                    | `goto_tb` slot 0 — chainable   |
/// | `ptr | 1`                    | `goto_tb` slot 1 — chainable   |
/// | `ptr | 2`                    | Indirect jump (no chain)       |
/// | `EXCP_ECALL` (= 3)           | ECALL exception                |
/// | `EXCP_EBREAK` (= 4)          | EBREAK exception               |
/// | `EXCP_UNDEF` (= 5)           | Undefined instruction          |
pub const TB_EXIT_IDX0: u64 = 0;
pub const TB_EXIT_IDX1: u64 = 1;
pub const TB_EXIT_NOCHAIN: u64 = 2;
/// Every real exit code must be >= TB_EXIT_MAX.
pub const TB_EXIT_MAX: u64 = 3;

/// Guest exception exit codes (must be >= `TB_EXIT_MAX`).
pub const EXCP_ECALL: u64 = TB_EXIT_MAX;
pub const EXCP_EBREAK: u64 = TB_EXIT_MAX + 1;
pub const EXCP_UNDEF: u64 = TB_EXIT_MAX + 2;

/// Encode an `exit_tb` return value.
///
/// For chainable exits (`val < TB_EXIT_MAX`) the TB pointer is OR'd with
/// the slot number into the return value.  The TB's alignment guarantees
/// the low 3 bits are zero, so `tb_ptr | val` is lossless.
///
/// Real exits (`val >= TB_EXIT_MAX`) are returned as-is; their values are
/// small integers that will never collide with heap pointers.
#[inline]
pub fn encode_tb_exit(tb_ptr: usize, val: u64) -> usize {
    if val < TB_EXIT_MAX {
        tb_ptr | (val as usize)
    } else {
        val as usize
    }
}

/// Decode an `exit_tb` return value.
///
/// Returns `(source_tb_ptr, exit_code)`.
///
/// * For chainable exits (`TB_EXIT_IDX0`, `TB_EXIT_IDX1`, `TB_EXIT_NOCHAIN`)
///   `source_tb_ptr` is `Some(ptr)` and `exit_code` is the slot (0, 1, or 2).
/// * For real exits `source_tb_ptr` is `None` and `exit_code` is the raw
///   exception code.
///
/// Distinguishing rule: if `raw > TB_NOCHAIN_MAX` (i.e. > 2) **and** the
/// value has pointer-magnitude (> a generous threshold), it is a chainable
/// exit encoded as `ptr | slot`.  In practice all heap pointers on 64-bit
/// Linux are ≥ 4096 (page size), so we use `raw > 7` as the threshold —
/// the low 3 bits carry the slot and the rest is the TB pointer.
#[inline]
pub fn decode_tb_exit(raw: usize) -> (Option<*mut TranslationBlock>, usize) {
    if raw > 7 {
        // Chainable exit: high bits = TB pointer, low 3 bits = slot.
        let slot = raw & 7;
        let tb_ptr = (raw & !7) as *mut TranslationBlock;
        (Some(tb_ptr), slot)
    } else {
        // Real exit code (small integer).
        (None, raw)
    }
}

/// Per-CPU direct-mapped TB jump cache.
///
/// Indexed by `(pc >> 2) & (TB_JMP_CACHE_SIZE - 1)`.
/// Provides O(1) lookup for the common case of re-executing the same PC.
pub struct JumpCache {
    /// Stores raw `*mut TranslationBlock` as usize; 0 means empty.
    entries: Box<[usize; TB_JMP_CACHE_SIZE]>,
}

impl JumpCache {
    pub fn new() -> Self {
        Self {
            entries: Box::new([0; TB_JMP_CACHE_SIZE]),
        }
    }

    fn index(pc: u64) -> usize {
        (pc as usize >> 2) & (TB_JMP_CACHE_SIZE - 1)
    }

    /// Look up the cached TB pointer for `pc`.  Returns `None` if empty.
    pub fn lookup(&self, pc: u64) -> Option<*mut TranslationBlock> {
        let raw = self.entries[Self::index(pc)];
        if raw == 0 {
            None
        } else {
            Some(raw as *mut TranslationBlock)
        }
    }

    pub fn insert(&mut self, pc: u64, tb: *mut TranslationBlock) {
        self.entries[Self::index(pc)] = tb as usize;
    }

    pub fn remove(&mut self, pc: u64) {
        self.entries[Self::index(pc)] = 0;
    }

    pub fn invalidate(&mut self) {
        self.entries.fill(0);
    }
}

impl Default for JumpCache {
    fn default() -> Self {
        Self::new()
    }
}
