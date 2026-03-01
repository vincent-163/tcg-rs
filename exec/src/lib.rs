//! TCG Execution Engine — TB cache, CPU execution loop, profiling, AOT.

pub mod exec_loop;
pub mod profile;
pub mod tb_store;

pub use exec_loop::{cpu_exec_loop, ExitReason};
pub use tb_store::TbStore;

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use tcg_backend::code_buffer::CodeBuffer;
use tcg_backend::HostCodeGen;
use tcg_core::tb::JumpCache;
use tcg_core::Context;

#[derive(Default)]
pub struct ExecStats {
    pub loop_iters: u64,
    pub jc_hit: u64,
    pub ht_hit: u64,
    pub translate: u64,
    pub chain_exit: [u64; 2],
    pub nochain_exit: u64,
    pub real_exit: u64,
    pub chain_patched: u64,
    pub chain_already: u64,
    pub hint_used: u64,
    pub exit_target_hit: u64,
    pub exit_target_miss: u64,
}

impl fmt::Display for ExecStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total_lookup = self.jc_hit + self.ht_hit + self.translate;
        writeln!(f, "=== TCG Execution Stats ===")?;
        writeln!(f, "loop iters:    {}", self.loop_iters)?;
        writeln!(f, "--- TB lookup ---")?;
        writeln!(f, "  jc hit:      {} ({:.1}%)", self.jc_hit, pct(self.jc_hit, total_lookup))?;
        writeln!(f, "  ht hit:      {} ({:.1}%)", self.ht_hit, pct(self.ht_hit, total_lookup))?;
        writeln!(f, "  translate:   {} ({:.1}%)", self.translate, pct(self.translate, total_lookup))?;
        writeln!(f, "--- Exit types ---")?;
        writeln!(f, "  chain[0]:    {}", self.chain_exit[0])?;
        writeln!(f, "  chain[1]:    {}", self.chain_exit[1])?;
        writeln!(f, "  nochain:     {}", self.nochain_exit)?;
        writeln!(f, "  real exit:   {}", self.real_exit)?;
        writeln!(f, "--- Chaining ---")?;
        writeln!(f, "  patched:     {}", self.chain_patched)?;
        writeln!(f, "  already:     {}", self.chain_already)?;
        writeln!(f, "--- Hint ---")?;
        writeln!(f, "  hint used:   {}", self.hint_used)?;
        writeln!(f, "--- Exit Target Cache ---")?;
        writeln!(f, "  hit:         {}", self.exit_target_hit)?;
        writeln!(f, "  miss:        {}", self.exit_target_miss)?;
        Ok(())
    }
}

fn pct(n: u64, total: u64) -> f64 {
    if total == 0 { 0.0 } else { n as f64 / total as f64 * 100.0 }
}

pub trait GuestCpu {
    fn get_pc(&self) -> u64;
    fn get_flags(&self) -> u32;
    fn gen_code(&mut self, ir: &mut Context, pc: u64, max_insns: u32) -> u32;
    fn env_ptr(&mut self) -> *mut u8;
}

/// AOT function table: maps guest PC -> native fn ptr.
pub struct AotTable {
    _handle: *mut libc::c_void,
    pub funcs: HashMap<u64, u64>,
}

unsafe impl Send for AotTable {}
unsafe impl Sync for AotTable {}

impl AotTable {
    pub fn load(path: &std::path::Path, load_vaddr: u64) -> Option<Self> {
        use std::ffi::CString;
        let cpath = CString::new(path.to_str()?).ok()?;
        unsafe {
            let handle = libc::dlopen(cpath.as_ptr(), libc::RTLD_NOW);
            if handle.is_null() { return None; }
            let idx_sym = CString::new("tb_index").unwrap();
            let idx_ptr = libc::dlsym(handle, idx_sym.as_ptr()) as *const u64;
            if idx_ptr.is_null() { libc::dlclose(handle); return None; }
            let mut funcs = HashMap::new();
            let mut i = 0;
            loop {
                let file_offset = *idx_ptr.add(i);
                // Sentinel: u64::MAX marks end of array
                if file_offset == u64::MAX {
                    break;
                }

                let sym = CString::new(
                    format!("tb_{file_offset:x}"),
                )
                .unwrap();
                let fptr = libc::dlsym(
                    handle, sym.as_ptr(),
                ) as u64;
                if fptr != 0 {
                    funcs.insert(
                        file_offset + load_vaddr,
                        fptr,
                    );
                }

                i += 1;
            }
            eprintln!("[aot] loaded {} functions from {}", funcs.len(), path.display());
            Some(Self { _handle: handle, funcs })
        }
    }

    pub fn lookup(&self, pc: u64) -> Option<u64> {
        self.funcs.get(&pc).copied()
    }
}

pub struct TranslateGuard {
    pub ir_ctx: Context,
    #[cfg(feature = "llvm")]
    pub llvm_jit: Option<tcg_backend::llvm::LlvmJit>,
}

pub struct SharedState<B: HostCodeGen> {
    pub tb_store: TbStore,
    code_buf: UnsafeCell<CodeBuffer>,
    pub backend: B,
    pub code_gen_start: usize,
    pub translate_lock: Mutex<TranslateGuard>,
    pub profiling: bool,
    pub aot_table: Option<AotTable>,
}

unsafe impl<B: HostCodeGen + Send> Send for SharedState<B> {}
unsafe impl<B: HostCodeGen + Sync> Sync for SharedState<B> {}

impl<B: HostCodeGen> SharedState<B> {
    pub fn code_buf(&self) -> &CodeBuffer {
        unsafe { &*self.code_buf.get() }
    }

    /// # Safety
    /// Caller must ensure no other threads are concurrently writing to the
    /// code buffer.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn code_buf_mut(&self) -> &mut CodeBuffer {
        &mut *self.code_buf.get()
    }
}

pub struct PerCpuState {
    pub jump_cache: JumpCache,
    pub stats: ExecStats,
}

const MIN_CODE_BUF_REMAINING: usize = 4096;

pub struct ExecEnv<B: HostCodeGen> {
    pub shared: Arc<SharedState<B>>,
    pub per_cpu: PerCpuState,
}

impl<B: HostCodeGen> ExecEnv<B> {
    pub fn new_with_opts(mut backend: B, profiling: bool, aot: Option<AotTable>) -> Self {
        let mut code_buf = CodeBuffer::new(16 * 1024 * 1024).expect("mmap failed");
        backend.emit_prologue(&mut code_buf);
        backend.emit_epilogue(&mut code_buf);
        let code_gen_start = code_buf.offset();
        let mut ir_ctx = Context::new();
        backend.init_context(&mut ir_ctx);

        let shared = Arc::new(SharedState {
            tb_store: TbStore::new(),
            code_buf: UnsafeCell::new(code_buf),
            backend,
            code_gen_start,
            translate_lock: Mutex::new(TranslateGuard {
                ir_ctx,
                #[cfg(feature = "llvm")]
                llvm_jit: None,
            }),
            profiling,
            aot_table: aot,
        });

        Self {
            shared,
            per_cpu: PerCpuState {
                jump_cache: JumpCache::new(),
                stats: ExecStats::default(),
            },
        }
    }

    pub fn new(backend: B) -> Self {
        Self::new_with_opts(backend, false, None)
    }

    #[cfg(feature = "llvm")]
    pub fn enable_llvm(&self) {
        let mut guard = self.shared.translate_lock.lock().unwrap();
        guard.llvm_jit = Some(tcg_backend::llvm::LlvmJit::new());
    }
}
