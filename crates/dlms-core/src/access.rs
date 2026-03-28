//! DLMS data access mode definition
//!
//! Reference: Green Book Ed.9 §9.5.4, Blue Book Part 2 §2.1

/// Access mode for a COSEM attribute
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum AccessMode {
    /// No access
    NoAccess = 0,
    /// Read only
    Read = 1,
    /// Write only
    Write = 2,
    /// Read and write
    ReadWrite = 3,
}

impl AccessMode {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::NoAccess),
            1 => Some(Self::Read),
            2 => Some(Self::Write),
            3 => Some(Self::ReadWrite),
            _ => None,
        }
    }

    pub fn can_read(&self) -> bool {
        matches!(self, Self::Read | Self::ReadWrite)
    }

    pub fn can_write(&self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite)
    }
}

/// Method access mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum MethodAccessMode {
    /// No access
    NoAccess = 0,
    /// Access granted
    Access = 1,
    /// Authenticated access
    Authenticated = 2,
}

impl MethodAccessMode {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::NoAccess),
            1 => Some(Self::Access),
            2 => Some(Self::Authenticated),
            _ => None,
        }
    }
}
