//! Event Notification PDU
//!
//! Reference: IEC 62056-53 §8.4.6

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::string::ToString;
use dlms_core::{DlmsType, ObisCode};
use crate::types::{ApduError, TAG_EVENT_NOTIFICATION, PRIORITY_NORMAL, PRIORITY_HIGH};
use crate::codec::{ApduEncoder, ApduDecoder};

/// Event Notification PDU
/// Sent from meter to client to notify about events
#[derive(Debug, Clone, PartialEq)]
pub struct EventNotification {
    pub invoke_id: u8,
    pub date_time: DlmsType,
    pub event_code: EventCode,
    pub priority: Priority,
}

impl EventNotification {
    pub fn new(invoke_id: u8, date_time: DlmsType, event_code: EventCode) -> Self {
        Self {
            invoke_id,
            date_time,
            event_code,
            priority: Priority::Normal,
        }
    }

    pub fn with_priority(
        invoke_id: u8,
        date_time: DlmsType,
        event_code: EventCode,
        priority: Priority,
    ) -> Self {
        Self {
            invoke_id,
            date_time,
            event_code,
            priority,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_EVENT_NOTIFICATION, 0x00);
        enc.write_byte(self.invoke_id);
        enc.write_dlms_value(&self.date_time)?;

        // Event code as octet string
        let event_bytes = self.event_code.to_bytes();
        enc.write_byte(0x09); // octet-string tag
        enc.write_byte(event_bytes.len() as u8);
        enc.write_bytes(&event_bytes);

        // Priority
        enc.write_byte(self.priority.to_byte());

        Ok(enc.into_bytes())
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, _subtype) = dec.read_tag()?;
        if tag_type != TAG_EVENT_NOTIFICATION {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_byte()?;
        let date_time = dec.read_dlms_value()?;

        // Read event code as octet string
        let tag = dec.read_byte()?;
        if tag != 0x09 {
            return Err(ApduError::InvalidData);
        }
        let len = dec.read_byte()? as usize;
        let event_bytes = dec.read_bytes(len)?;
        let event_code = EventCode::from_bytes(event_bytes)?;

        // Read priority
        let priority_byte = dec.read_byte()?;
        let priority = Priority::from_byte(priority_byte);

        Ok(Self {
            invoke_id,
            date_time,
            event_code,
            priority,
        })
    }
}

/// Event notification priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Normal,
    High,
}

impl Priority {
    pub fn from_byte(b: u8) -> Self {
        match b {
            PRIORITY_HIGH => Self::High,
            _ => Self::Normal,
        }
    }

    pub fn to_byte(self) -> u8 {
        match self {
            Self::Normal => PRIORITY_NORMAL,
            Self::High => PRIORITY_HIGH,
        }
    }
}

/// Event codes (IEC 62056-6-2 §7.3.4)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventCode {
    /// Parameter error (wrong data)
    ParameterError,
    /// Job timeout
    JobTimeout,
    /// Schedule error
    ScheduleError,
    /// Self-test error
    SelfTestError,
    /// Memory error
    MemoryError,
    /// Communication error
    CommunicationError,
    /// Data not available
    DataNotAvailable,
    /// Clock error
    ClockError,
    /// LCD error
    LcdError,
    /// Measurement system error
    MeasurementSystemError,
    /// Watchdog reset
    WatchdogReset,
    /// Battery warning
    BatteryWarning,
    /// Error register (FRI)
    ErrorRegister,
    /// Voltage low sags
    VoltageLowSags,
    /// Voltage high swells
    VoltageHighSwells,
    /// Voltage dip
    VoltageDip,
    /// Voltage missing
    VoltageMissing,
    /// Current unbalance
    CurrentUnbalance,
    /// Overload
    Overload,
    /// Load limit exceeded
    LoadLimitExceeded,
    /// Meter cover opened
    MeterCoverOpened,
    /// Meter cover closed
    MeterCoverClosed,
    /// Power failure
    PowerFailure,
    /// Power quality
    PowerQuality,
    /// Fraud detected
    FraudDetected,
    /// Reserved for manufacturer specific (0-255)
    ManufacturerSpecific(u8),
}

impl EventCode {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::ParameterError => vec![0x01],
            Self::JobTimeout => vec![0x02],
            Self::ScheduleError => vec![0x03],
            Self::SelfTestError => vec![0x04],
            Self::MemoryError => vec![0x05],
            Self::CommunicationError => vec![0x06],
            Self::DataNotAvailable => vec![0x07],
            Self::ClockError => vec![0x08],
            Self::LcdError => vec![0x09],
            Self::MeasurementSystemError => vec![0x0A],
            Self::WatchdogReset => vec![0x0B],
            Self::BatteryWarning => vec![0x0C],
            Self::ErrorRegister => vec![0x0D],
            Self::VoltageLowSags => vec![0x0E],
            Self::VoltageHighSwells => vec![0x0F],
            Self::VoltageDip => vec![0x10],
            Self::VoltageMissing => vec![0x11],
            Self::CurrentUnbalance => vec![0x12],
            Self::Overload => vec![0x13],
            Self::LoadLimitExceeded => vec![0x14],
            Self::MeterCoverOpened => vec![0x15],
            Self::MeterCoverClosed => vec![0x16],
            Self::PowerFailure => vec![0x17],
            Self::PowerQuality => vec![0x18],
            Self::FraudDetected => vec![0x19],
            Self::ManufacturerSpecific(code) => vec![*code],
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ApduError> {
        if bytes.is_empty() {
            return Err(ApduError::TooShort);
        }

        let code = bytes[0];
        Ok(match code {
            0x01 => Self::ParameterError,
            0x02 => Self::JobTimeout,
            0x03 => Self::ScheduleError,
            0x04 => Self::SelfTestError,
            0x05 => Self::MemoryError,
            0x06 => Self::CommunicationError,
            0x07 => Self::DataNotAvailable,
            0x08 => Self::ClockError,
            0x09 => Self::LcdError,
            0x0A => Self::MeasurementSystemError,
            0x0B => Self::WatchdogReset,
            0x0C => Self::BatteryWarning,
            0x0D => Self::ErrorRegister,
            0x0E => Self::VoltageLowSags,
            0x0F => Self::VoltageHighSwells,
            0x10 => Self::VoltageDip,
            0x11 => Self::VoltageMissing,
            0x12 => Self::CurrentUnbalance,
            0x13 => Self::Overload,
            0x14 => Self::LoadLimitExceeded,
            0x15 => Self::MeterCoverOpened,
            0x16 => Self::MeterCoverClosed,
            0x17 => Self::PowerFailure,
            0x18 => Self::PowerQuality,
            0x19 => Self::FraudDetected,
            _ => Self::ManufacturerSpecific(code),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dlms_core::{CosemDateTime, CosemDate, CosemTime};

    #[test]
    fn test_event_notification_encode() {
        let dt = CosemDateTime {
            date: CosemDate { year: 2024, month: 1, day: 1, day_of_week: 1 },
            time: CosemTime { hour: 0, minute: 0, second: 0, hundredths: 0 },
            deviation: 0,
            clock_status: 0,
        };
        let event = EventNotification::new(
            1,
            DlmsType::DateTime(dt),
            EventCode::PowerFailure,
        );
        let encoded = event.encode().unwrap();

        assert_eq!(encoded[0], TAG_EVENT_NOTIFICATION);
        assert_eq!(encoded[2], 1); // invoke_id
    }

    #[test]
    fn test_event_notification_roundtrip() {
        let dt = CosemDateTime {
            date: CosemDate { year: 2024, month: 1, day: 1, day_of_week: 1 },
            time: CosemTime { hour: 0, minute: 0, second: 0, hundredths: 0 },
            deviation: 0,
            clock_status: 0,
        };
        let event = EventNotification::new(
            42,
            DlmsType::DateTime(dt),
            EventCode::BatteryWarning,
        );
        let encoded = event.encode().unwrap();
        let decoded = EventNotification::decode(&encoded).unwrap();

        assert_eq!(decoded.invoke_id, event.invoke_id);
        assert_eq!(decoded.event_code, event.event_code);
        assert_eq!(decoded.priority, event.priority);
    }

    #[test]
    fn test_event_code_roundtrip() {
        let codes = [
            EventCode::ParameterError,
            EventCode::PowerFailure,
            EventCode::BatteryWarning,
            EventCode::FraudDetected,
        ];

        for code in &codes {
            let bytes = code.to_bytes();
            let decoded = EventCode::from_bytes(&bytes).unwrap();
            assert_eq!(decoded, *code);
        }
    }

    #[test]
    fn test_priority_from_byte() {
        assert_eq!(Priority::from_byte(0), Priority::Normal);
        assert_eq!(Priority::from_byte(1), Priority::High);
        assert_eq!(Priority::from_byte(255), Priority::Normal); // Unknown = normal
    }

    #[test]
    fn test_priority_to_byte() {
        assert_eq!(Priority::Normal.to_byte(), 0);
        assert_eq!(Priority::High.to_byte(), 1);
    }

    #[test]
    fn test_manufacturer_specific_event() {
        let code = EventCode::ManufacturerSpecific(0xFE);
        let bytes = code.to_bytes();
        assert_eq!(bytes, vec![0xFE]);

        let decoded = EventCode::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, code);
    }
}
