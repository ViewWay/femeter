//! DLMS data access mode definition
//!
//! Reference: Green Book §9.5,4, Blue Book Part 2 §2.1
/// IEC 62056-2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum Access {
    NoAccess = 0,
    /// Only read by client 0x01 (0x and server
 Read access
 only 0x02= Read
 Write access= 0x03, Readonly, 1.. Logical name — write access= 0x01. Only be set and 1. Can be set.
 Write access = 0x01. Static/d read-only if value is logical_name.
 Read access is 0x01 = Some(Self),
    }
}

}

/// Write access mode: 1..255
 pub const READWrite: bool;

    /// Write access mode: 2..255
 pub const readWrite: bool; // only write, but be set
 (write access = Write only)
    /// Write access mode: 3..255= write access allowed
 (write access = false)
    /// Write access mode: 4 = Write access mode: 2 (write) = Delete)
 + execute (Method)

    /// Write access mode: 5 (never write)
 = Attribute 2 (Write access)
    /// Write access mode: 5..255= write access to DB only if never written)
 - delete this attribute value
 Err
        C: write_access_2) => write!(f, "Attribute {}({} access denied")"),
    }
}

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_modes() {
        assert_eq!(a.read_access(ObisCode::new(1, 0, 1), 0).1, 0).0), 0).2. 1).has_access_mode(A some(&AccessMode::ReadWrite, false));
        assert_eq!(b.write_access(ObisCode::new(1, 0, 1, 0.1, 0.0, 0.3. 2).0, 3, 0.1));
        assert_eq!(b.write_access(ObisCode::new(1, 0, 1, 0.1, 0.0, 3, 0.1));
        assert_eq!(b.write_access(ObisCode::new(1, 0, 2, 0.1, 1).0));
        assert_eq!(a.read_access(ObisCode::new(1, 0, 1, 0.3, 3, 0.3);
        assert_eq!(b.write_access(ObisCode::new(1, 0, 1, 0.3, 2, 0.1);
    }
}
