//! Memory pool traits
//!
//! Provides fixed-size memory pools for efficient allocation/deallocation.

use core::fmt;

#[cfg(feature = "std")]
extern crate std;

/// Memory pool configuration
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct PoolConfig {
    /// Size of each block in bytes
    pub block_size: usize,
    /// Number of blocks in the pool
    pub block_count: usize,
}

impl PoolConfig {
    /// Create a new pool configuration
    pub const fn new(block_size: usize, block_count: usize) -> Self {
        Self {
            block_size,
            block_count,
        }
    }

    /// Get total pool size in bytes
    pub const fn total_size(&self) -> usize {
        self.block_size * self.block_count
    }
}

/// Memory pool handle for managing pool state
///
/// This handle provides operations on a memory pool instance.
pub trait MemPoolHandle: Send + Sync {
    /// Get the block size
    fn block_size(&self) -> usize;

    /// Get total number of blocks
    fn block_count(&self) -> usize;

    /// Get number of free blocks
    fn free_count(&self) -> usize;

    /// Get number of allocated blocks
    fn allocated_count(&self) -> usize {
        self.block_count() - self.free_count()
    }

    /// Check if the pool has free blocks
    fn has_free(&self) -> bool {
        self.free_count() > 0
    }

    /// Check if the pool is empty (all blocks allocated)
    fn is_empty(&self) -> bool {
        self.free_count() == 0
    }
}

/// Memory pool-related errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum MemPoolError {
    /// Pool is full (all blocks allocated)
    OutOfMemory,
    /// Invalid block pointer
    InvalidBlock,
    /// Block not from this pool
    NotFromPool,
    /// Double free detected
    DoubleFree,
    /// Invalid pool configuration
    InvalidConfig,
}

#[cfg(feature = "std")]
impl fmt::Display for MemPoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "Pool out of memory"),
            Self::InvalidBlock => write!(f, "Invalid block pointer"),
            Self::NotFromPool => write!(f, "Block not from this pool"),
            Self::DoubleFree => write!(f, "Double free detected"),
            Self::InvalidConfig => write!(f, "Invalid pool configuration"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MemPoolError {}

/// RTOS memory pool trait
///
/// Fixed-size memory pools for efficient, deterministic allocation.
/// Unlike general allocators, memory pools have O(1) allocation/deallocation
/// and never fragment.
///
/// # Example
///
/// ```ignore
/// let config = PoolConfig::new(256, 10);  // 10 blocks of 256 bytes
/// let pool = rtos.create_pool(config)?;
///
/// // Allocate a block
/// let block = pool.allocate()?;
///
/// // Use the block...
///
/// // Deallocate when done
/// pool.deallocate(block);
/// ```
pub trait RtosMemPool: Sized {
    /// Memory pool handle type
    type Handle: MemPoolHandle;

    /// Create a new memory pool
    ///
    /// # Arguments
    /// * `config` - Pool configuration (block size and count)
    ///
    /// # Errors
    /// Returns an error if:
    /// - Insufficient memory for the pool
    /// - Invalid configuration (e.g., block_size = 0)
    fn create_pool(&self, config: PoolConfig) -> Result<Self::Handle, MemPoolError>;

    /// Allocate a block from the pool
    ///
    /// Returns a pointer to the allocated block.
    ///
    /// # Errors
    /// Returns `OutOfMemory` if no free blocks available.
    fn allocate(&self, pool: &Self::Handle) -> Result<*mut u8, MemPoolError>;

    /// Deallocate a block back to the pool
    ///
    /// # Errors
    /// Returns errors if:
    /// - Block pointer is invalid
    /// - Block doesn't belong to this pool
    /// - Double free detected
    fn deallocate(&self, pool: &Self::Handle, block: *mut u8) -> Result<(), MemPoolError>;

    /// Try to allocate without blocking
    ///
    /// Same as `allocate` for memory pools (allocation doesn't block).
    fn try_allocate(&self, pool: &Self::Handle) -> Result<*mut u8, MemPoolError> {
        self.allocate(pool)
    }

    /// Get pool statistics
    fn get_stats(&self, pool: &Self::Handle) -> PoolStats {
        PoolStats {
            block_size: pool.block_size(),
            total_blocks: pool.block_count(),
            free_blocks: pool.free_count(),
            allocated_blocks: pool.allocated_count(),
        }
    }
}

/// Memory pool statistics
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct PoolStats {
    /// Size of each block
    pub block_size: usize,
    /// Total number of blocks
    pub total_blocks: usize,
    /// Number of free blocks
    pub free_blocks: usize,
    /// Number of allocated blocks
    pub allocated_blocks: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config() {
        let config = PoolConfig::new(256, 10);
        assert_eq!(config.block_size, 256);
        assert_eq!(config.block_count, 10);
        assert_eq!(config.total_size(), 2560);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_mempool_error_display() {
        assert_eq!(alloc::format!("{}", MemPoolError::OutOfMemory), "Pool out of memory");
        assert_eq!(alloc::format!("{}", MemPoolError::DoubleFree), "Double free detected");
    }
}
