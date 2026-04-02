//!
//! COSEM Interface Classes (105 ICs per Blue Book Part 2)
//!
//! This crate implements all 105 DLMS/COSEM interface classes organized into functional groups.
//!
//! # Interface Class Groups
//!
//! - **Group 1**: Data & Register (11 ICs) - Basic data storage
//! - **Group 2**: Management (10 ICs) - COSEM object management
//! - **Group 3**: Time & Event Control (12 ICs) - Scheduling and monitoring
//! - **Group 4**: Payment (5 ICs) - Prepayment functionality
//! - **Group 5**: Local Communication (9 ICs) - Local port setup
//! - **Group 6**: M-Bus (6 ICs) - M-Bus protocol
//! - **Group 7**: Internet (9 ICs) - IP-based communication
//! - **Group 8**: PLC/Wireless (43 ICs) - Power line and wireless communication
//!
//! # Usage
//!
//! Each interface class implements the [`CosemClass`] trait from `dlms-core`.
//!
//! ```rust,no_run
//! use dlms_cosem::data_register::ic3_register::Register;
//! use dlms_core::{obis::ObisCode, traits::CosemClass};
//!
//! let mut register = Register::new(
//!     ObisCode::new(1, 0, 1, 8, 0, 255),  // Total active energy import
//!     0,   // scaler
//!     dlms_core::units::Unit::WattHour,
//! );
//! ```

#![no_std]

extern crate alloc;

use dlms_core::{errors::CosemError, obis::ObisCode, types::DlmsType};

// Base module with common utilities
mod base;

// Group 1: Data & Register (11 ICs)
pub mod data_register;

// Group 2: Management (10 ICs)
pub mod management;

// Group 3: Time & Event Control (12 ICs)
pub mod time_control;

// Group 4: Payment (5 ICs)
pub mod payment;

// Group 5: Local Communication (9 ICs)
pub mod local_comm;

// Group 6: M-Bus (6 ICs)
pub mod mbus;

// Group 7: Internet (9 ICs)
pub mod internet;

// Group 8a: PLC (23 ICs)
pub mod plc;

// Group 8b: Wireless (17 ICs)
pub mod wireless;

// Group 8c: LLC (3 ICs)
pub mod llc;

/// Helper function to create an octet-string from ObisCode
pub fn obis_to_octet_string(obis: &ObisCode) -> DlmsType {
    DlmsType::OctetString(obis.to_bytes().to_vec())
}

/// Helper function to extract ObisCode from octet-string DlmsType
pub fn octet_string_to_obis(value: &DlmsType) -> Result<ObisCode, CosemError> {
    match value {
        DlmsType::OctetString(data) if data.len() == 6 => {
            let bytes = [data[0], data[1], data[2], data[3], data[4], data[5]];
            Ok(ObisCode::from_bytes(&bytes))
        }
        _ => Err(CosemError::TypeMismatch {
            expected: 9, // octet-string tag
            got: value.tag(),
        }),
    }
}

/// Create a scaler-unit structure from scaler and unit
pub fn scaler_unit_to_dlms(scaler: i8, unit: dlms_core::units::Unit) -> DlmsType {
    DlmsType::Structure(alloc::vec![
        DlmsType::Int8(scaler),
        DlmsType::UInt16(unit as u16),
    ])
}

/// Extract scaler and unit from a DlmsType structure
pub fn dlms_to_scaler_unit(value: &DlmsType) -> Result<(i8, dlms_core::units::Unit), CosemError> {
    match value {
        DlmsType::Structure(items) if items.len() >= 2 => {
            let scaler = items[0].as_i8().ok_or(CosemError::TypeMismatch {
                expected: 16, // long (int16)
                got: items[0].tag(),
            })?;
            let unit_code = items[1].as_u16().ok_or(CosemError::TypeMismatch {
                expected: 18, // long-unsigned (uint16)
                got: items[1].tag(),
            })?;
            // Safety: unwrap_or is safe here - if the unit code is unknown,
            // we default to Unit::None which is a valid fallback
            let unit = dlms_core::units::Unit::from_code(unit_code)
                .unwrap_or(dlms_core::units::Unit::None);
            Ok((scaler, unit))
        }
        _ => Err(CosemError::TypeMismatch {
            expected: 2, // structure
            got: value.tag(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obis_conversion() {
        let obis = ObisCode::new(1, 0, 1, 8, 0, 255);
        let dlms = obis_to_octet_string(&obis);
        let result = octet_string_to_obis(&dlms).unwrap();
        assert_eq!(result, obis);
        assert_eq!(dlms, DlmsType::OctetString(alloc::vec![1, 0, 1, 8, 0, 255]));
    }

    #[test]
    fn test_scaler_unit_conversion() {
        let dlms = scaler_unit_to_dlms(-3, dlms_core::units::Unit::WattHour);
        let (scaler, unit) = dlms_to_scaler_unit(&dlms).unwrap();
        assert_eq!(scaler, -3);
        assert_eq!(unit, dlms_core::units::Unit::WattHour);
    }

    // ============================================================
    // Phase C — Boundary Tests
    // ============================================================

    #[test]
    fn test_octet_string_to_obis_invalid_length() {
        let result = octet_string_to_obis(&DlmsType::OctetString(alloc::vec![1, 2]));
        assert!(result.is_err());

        let result = octet_string_to_obis(&DlmsType::OctetString(alloc::vec![]));
        assert!(result.is_err());

        let result = octet_string_to_obis(&DlmsType::OctetString(alloc::vec![1, 2, 3, 4, 5, 6, 7]));
        assert!(result.is_err());
    }

    #[test]
    fn test_octet_string_to_obis_wrong_type() {
        let result = octet_string_to_obis(&DlmsType::UInt32(42));
        assert!(result.is_err());
    }

    #[test]
    fn test_dlms_to_scaler_unit_invalid_type() {
        let result = dlms_to_scaler_unit(&DlmsType::Null);
        assert!(result.is_err());

        let result = dlms_to_scaler_unit(&DlmsType::UInt32(42));
        assert!(result.is_err());

        let result = dlms_to_scaler_unit(&DlmsType::Structure(alloc::vec![DlmsType::from_u32(1)]));
        assert!(result.is_err());
    }

    #[test]
    fn test_obis_all_zeros() {
        let obis = ObisCode::new(0, 0, 0, 0, 0, 0);
        let dlms = obis_to_octet_string(&obis);
        let result = octet_string_to_obis(&dlms).unwrap();
        assert_eq!(result, obis);
    }

    #[test]
    fn test_obis_all_255() {
        let obis = ObisCode::new(255, 255, 255, 255, 255, 255);
        let dlms = obis_to_octet_string(&obis);
        let result = octet_string_to_obis(&dlms).unwrap();
        assert_eq!(result, obis);
    }

    #[test]
    fn test_scaler_unit_all_scalers() {
        for scaler in -7i8..=7 {
            let dlms = scaler_unit_to_dlms(scaler, dlms_core::units::Unit::None);
            let (s, _u) = dlms_to_scaler_unit(&dlms).unwrap();
            assert_eq!(s, scaler);
        }
    }

    #[test]
    fn test_gb_standard_obis_codes() {
        // China GB/T 17215.301 standard OBIS codes for energy meters
        let gb_obis = [
            ObisCode::new(1, 0, 1, 8, 0, 255),  // 正向有功总电能
            ObisCode::new(1, 0, 1, 8, 1, 255),  // 费率1正向有功
            ObisCode::new(1, 0, 1, 8, 2, 255),  // 费率2正向有功
            ObisCode::new(1, 0, 1, 8, 3, 255),  // 费率3正向有功
            ObisCode::new(1, 0, 1, 8, 4, 255),  // 费率4正向有功
            ObisCode::new(1, 0, 2, 8, 0, 255),  // 反向有功总电能
            ObisCode::new(1, 0, 3, 8, 0, 255),  // 正向无功总电能
            ObisCode::new(1, 0, 4, 8, 0, 255),  // 反向无功总电能
            ObisCode::new(0, 0, 1, 0, 0, 255),  // 日期时间
            ObisCode::new(0, 0, 96, 1, 0, 255), // 逻辑设备名
        ];
        for obis in &gb_obis {
            let dlms = obis_to_octet_string(obis);
            let result = octet_string_to_obis(&dlms).unwrap();
            assert_eq!(result, *obis);
        }
    }
}
