use std::{borrow::Cow, cell::RefCell, fmt::Debug, sync::atomic::AtomicU64};


/// A program level identifier for resources.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResId {
    Defined(Cow<'static, str>),
    Auto(PUID),
}

impl ResId {
    pub fn new() -> Self {
        Self::Auto(PUID::new())
    }
}

impl From<&'static str> for ResId {
    fn from(s: &'static str) -> Self {
        Self::Defined(Cow::Borrowed(s))
    }
}

impl From<String> for ResId {
    fn from(s: String) -> Self {
        Self::Defined(Cow::Owned(s))
    }
}

impl From<PUID> for ResId {
    fn from(puid: PUID) -> Self {
        Self::Auto(puid)
    }
}

impl std::fmt::Display for ResId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResId::Defined(s) => write!(f, "{:?}", s),
            ResId::Auto(puid) => write!(f, "{}", puid),
        }
    }
}

/// Stands for "Program Unique ID"
///
/// A numeric ID that is unique across the current program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PUID(u64);

impl PUID {
    pub fn new() -> Self {
        Self(Self::make_new())
    }

    /// Reserve `n` PUIDs, returning the first one.
    fn reserve_block(n: u64) -> u64 {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        NEXT_ID.fetch_add(n, std::sync::atomic::Ordering::Relaxed) // TODO ORDERING????
    }

    /// Get the next PUID.
    fn make_new() -> u64 {
        const BLOCK_SIZE: u64 = 1 << 10;

        struct Block {
            current: u64,
            end: u64,
        }

        thread_local! {
            static NEXT_ID: RefCell<Block> = RefCell::new(Block {
                current: 0,
                end: 0,
            });
        }

        NEXT_ID.with(|block| {
            let mut block = block.borrow_mut();
            if block.current >= block.end {
                block.current = Self::reserve_block(BLOCK_SIZE);
                block.end = block.current + BLOCK_SIZE;
            }
            let id = block.current;
            block.current += 1;
            id
        })
    }
}

impl std::fmt::Display for PUID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl std::fmt::Pointer for PUID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl std::fmt::LowerHex for PUID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl std::fmt::UpperHex for PUID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:X}", self.0)
    }
}

