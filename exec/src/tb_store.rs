use std::sync::Mutex;

use tcg_backend::code_buffer::CodeBuffer;
use tcg_backend::HostCodeGen;
use tcg_core::tb::{TranslationBlock, TB_HASH_SIZE};

/// Thread-safe storage and hash-table lookup for TBs.
///
/// Each TB is heap-allocated via `Box` and owned by this store.
/// TBs are identified and referenced by their stable raw pointer
/// (`*mut TranslationBlock`) rather than a Vec index.  The hash table
/// chains use the `hash_next` field (stored as a raw pointer value,
/// 0 = end of chain) to avoid a separate allocation.
///
/// Ownership model:
/// - `alloc` boxes a new TB and returns its raw pointer.
/// - All other operations borrow via raw pointers.
/// - `flush` / `Drop` free every owned box.
pub struct TbStore {
    /// All owned TBs (for drop bookkeeping and iteration).
    /// Protected by the hash mutex (same single mutex used for all mutations).
    owned: Mutex<(Vec<*mut TranslationBlock>, Vec<Option<usize>>)>,
    // (vec of owned ptrs, hash buckets)
}

// SAFETY: TbStore owns all the TranslationBlock pointers and
// controls all access to them through its API + mandatory locking.
unsafe impl Sync for TbStore {}
unsafe impl Send for TbStore {}

impl TbStore {
    pub fn new() -> Self {
        let buckets = vec![None; TB_HASH_SIZE];
        Self {
            owned: Mutex::new((Vec::new(), buckets)),
        }
    }

    /// Allocate a new TB on the heap.
    ///
    /// Returns a stable `*mut TranslationBlock`.  The pointer remains
    /// valid until `flush` is called.  Caller must hold `translate_lock`
    /// for the immutable-field write phase that follows.
    pub fn alloc(&self, pc: u64, flags: u32, cflags: u32) -> *mut TranslationBlock {
        let tb = Box::new(TranslationBlock::new(pc, flags, cflags));
        let ptr = Box::into_raw(tb);
        self.owned.lock().unwrap().0.push(ptr);
        ptr
    }

    /// Get a shared reference to a TB from its pointer.
    ///
    /// # Safety
    /// `tb` must be a pointer previously returned by `alloc` and not yet
    /// freed by `flush`.
    #[inline]
    pub unsafe fn get<'a>(&self, tb: *mut TranslationBlock) -> &'a TranslationBlock {
        &*tb
    }

    /// Get a mutable reference to a TB from its pointer.
    ///
    /// # Safety
    /// Caller must ensure exclusive access (e.g. under translate_lock for
    /// immutable fields, or per-TB jmp lock for chaining fields).
    #[allow(clippy::mut_from_ref)]
    #[inline]
    pub unsafe fn get_mut<'a>(&self, tb: *mut TranslationBlock) -> &'a mut TranslationBlock {
        &mut *tb
    }

    /// Lookup a valid TB by (pc, flags) in the hash table.
    /// Only returns TBs that have been translated (host_size > 0).
    pub fn lookup(&self, pc: u64, flags: u32) -> Option<*mut TranslationBlock> {
        let guard = self.owned.lock().unwrap();
        let buckets = &guard.1;
        // buckets store (raw usize cast of *mut TB) as Option<usize>, 0 = none
        let bucket = TranslationBlock::hash(pc, flags);
        let mut cur = match buckets[bucket] {
            Some(raw) => raw as *mut TranslationBlock,
            None => return None,
        };
        loop {
            let tb = unsafe { &*cur };
            use std::sync::atomic::Ordering;
            if !tb.invalid.load(Ordering::Acquire)
                && tb.pc == pc
                && tb.flags == flags
                && tb.host_size > 0
            {
                return Some(cur);
            }
            let next_raw = tb.hash_next;
            if next_raw == 0 {
                return None;
            }
            cur = next_raw as *mut TranslationBlock;
        }
    }

    /// Insert a TB into the hash table (prepend to bucket).
    ///
    /// Must be called after the TB's `pc`, `flags`, and `host_size` are set.
    ///
    /// # Safety
    /// `tb_ptr` must be a valid pointer previously returned by `alloc` and
    /// not yet freed by `flush`.
    pub unsafe fn insert(&self, tb_ptr: *mut TranslationBlock) {
        let mut guard = self.owned.lock().unwrap();
        let tb = unsafe { &mut *tb_ptr };
        let bucket = TranslationBlock::hash(tb.pc, tb.flags);
        let buckets = &mut guard.1;
        // Prepend: new node's next = old head.
        let old_head = buckets[bucket].unwrap_or(0);
        tb.hash_next = old_head;
        buckets[bucket] = Some(tb_ptr as usize);
    }

    /// Mark a TB as invalid, unlink all chained jumps, and remove it from
    /// the hash chain.
    ///
    /// # Safety
    /// `tb_ptr` must be a valid pointer previously returned by `alloc` and
    /// not yet freed by `flush`.
    pub unsafe fn invalidate<B: HostCodeGen>(
        &self,
        tb_ptr: *mut TranslationBlock,
        code_buf: &CodeBuffer,
        backend: &B,
    ) {
        use std::sync::atomic::Ordering;
        let tb = unsafe { &*tb_ptr };
        tb.invalid.store(true, Ordering::Release);

        // 1. Unlink incoming edges.
        let jmp_list = {
            let mut jmp = tb.jmp.lock().unwrap();
            std::mem::take(&mut jmp.jmp_list)
        };
        for (src_ptr, slot) in jmp_list {
            Self::reset_jump(unsafe { &*src_ptr }, code_buf, backend, slot);
            let mut src_jmp = unsafe { &*src_ptr }.jmp.lock().unwrap();
            src_jmp.jmp_dest[slot] = None;
        }

        // 2. Unlink outgoing edges.
        let outgoing = {
            let mut jmp = tb.jmp.lock().unwrap();
            let mut out: [(usize, *mut TranslationBlock); 2] =
                [(0, std::ptr::null_mut()); 2];
            let mut count = 0;
            for slot in 0..2 {
                if let Some(dst) = jmp.jmp_dest[slot].take() {
                    out[count] = (slot, dst);
                    count += 1;
                }
            }
            (out, count)
        };
        let (out, count) = outgoing;
        for &(slot, dst) in out.iter().take(count) {
            let mut dst_jmp = unsafe { &*dst }.jmp.lock().unwrap();
            dst_jmp
                .jmp_list
                .retain(|&(s, n)| !(s == tb_ptr && n == slot));
        }

        // 3. Remove from hash chain.
        let pc = tb.pc;
        let flags = tb.flags;
        let bucket = TranslationBlock::hash(pc, flags);
        let mut guard = self.owned.lock().unwrap();
        let buckets = &mut guard.1;
        let mut prev_raw: Option<*mut TranslationBlock> = None;
        let mut cur_raw = match buckets[bucket] {
            Some(r) => r as *mut TranslationBlock,
            None => return,
        };
        loop {
            let cur_tb = unsafe { &mut *cur_raw };
            if cur_raw == tb_ptr {
                let next = cur_tb.hash_next;
                if let Some(p) = prev_raw {
                    unsafe { &mut *p }.hash_next = next;
                } else {
                    buckets[bucket] = if next == 0 { None } else { Some(next) };
                }
                cur_tb.hash_next = 0;
                return;
            }
            let next_raw = cur_tb.hash_next;
            if next_raw == 0 {
                return;
            }
            prev_raw = Some(cur_raw);
            cur_raw = next_raw as *mut TranslationBlock;
        }
    }

    /// Reset a goto_tb jump back to its original target.
    fn reset_jump<B: HostCodeGen>(
        tb: &TranslationBlock,
        code_buf: &CodeBuffer,
        backend: &B,
        slot: usize,
    ) {
        if let (Some(jmp_off), Some(reset_off)) =
            (tb.jmp_insn_offset[slot], tb.jmp_reset_offset[slot])
        {
            backend.patch_jump(code_buf, jmp_off as usize, reset_off as usize);
        }
    }

    /// Flush all TBs and reset the hash table.
    ///
    /// # Safety
    /// Caller must ensure no other threads are accessing TBs.
    pub unsafe fn flush(&self) {
        let mut guard = self.owned.lock().unwrap();
        let (owned, buckets) = &mut *guard;
        // Free all TB boxes.
        for ptr in owned.drain(..) {
            drop(Box::from_raw(ptr));
        }
        // Clear hash buckets.
        for b in buckets.iter_mut() {
            *b = None;
        }
    }

    /// Iterate over all (possibly invalid) owned TB pointers.
    pub fn iter_all(&self) -> Vec<*mut TranslationBlock> {
        self.owned.lock().unwrap().0.clone()
    }

    pub fn len(&self) -> usize {
        self.owned.lock().unwrap().0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for TbStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TbStore {
    fn drop(&mut self) {
        unsafe { self.flush() };
    }
}
