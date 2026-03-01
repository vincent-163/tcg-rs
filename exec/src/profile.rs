//! TB execution profiling and AOT profile data.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::AtomicU64;

pub struct TbProfile {
    pub exec_count: AtomicU64,
}

impl TbProfile {
    pub fn new() -> Self {
        Self {
            exec_count: AtomicU64::new(0),
        }
    }
}

pub const DEFAULT_HOT_THRESHOLD: u64 = 10;

const MAGIC: &[u8; 8] = b"TCGPROF\0";
const VERSION: u32 = 2;

#[derive(Debug, Clone, Copy)]
pub struct ProfileEntry {
    pub file_offset: u64,
    pub exec_count: u64,
}

#[derive(Debug, Clone)]
pub struct ProfileData {
    pub threshold: u32,
    pub entries: Vec<ProfileEntry>,
}

impl ProfileData {
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut f = File::create(path)?;
        f.write_all(MAGIC)?;
        f.write_all(&VERSION.to_le_bytes())?;
        f.write_all(&self.threshold.to_le_bytes())?;
        f.write_all(&(self.entries.len() as u64).to_le_bytes())?;
        for e in &self.entries {
            f.write_all(&e.file_offset.to_le_bytes())?;
            f.write_all(&e.exec_count.to_le_bytes())?;
        }
        Ok(())
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let mut f = File::open(path)?;
        let mut m = [0u8; 8];
        f.read_exact(&mut m)?;
        if &m != MAGIC { return Err(io::Error::new(io::ErrorKind::InvalidData, "bad magic")); }
        let mut b4 = [0u8; 4];
        let mut b8 = [0u8; 8];
        f.read_exact(&mut b4)?;
        if u32::from_le_bytes(b4) != VERSION {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "bad version"));
        }
        f.read_exact(&mut b4)?;
        let threshold = u32::from_le_bytes(b4);
        f.read_exact(&mut b8)?;
        let count = u64::from_le_bytes(b8) as usize;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            f.read_exact(&mut b8)?;
            let file_offset = u64::from_le_bytes(b8);
            f.read_exact(&mut b8)?;
            let exec_count = u64::from_le_bytes(b8);
            entries.push(ProfileEntry { file_offset, exec_count });
        }
        Ok(Self { threshold, entries })
    }

    pub fn hot_offsets(&self) -> std::collections::HashSet<u64> {
        self.entries.iter().map(|e| e.file_offset).collect()
    }

    pub fn should_export(_e: &ProfileEntry) -> bool {
        true
    }
}
