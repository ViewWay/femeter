//! Date/time special codec helpers
//!
//! Provides encode/decode for COSEM date, time, datetime as raw bytes.

use dlms_core::types::{CosemDate, CosemTime, CosemDateTime};

/// Encode a CosemDate to 5 bytes
pub fn encode_date(d: &CosemDate) -> [u8; 5] {
    [
        (d.year >> 8) as u8,
        (d.year & 0xFF) as u8,
        d.month,
        d.day,
        d.day_of_week,
    ]
}

/// Decode a CosemDate from 5 bytes
pub fn decode_date(b: &[u8]) -> Option<CosemDate> {
    if b.len() < 5 { return None; }
    Some(CosemDate {
        year: u16::from_be_bytes([b[0], b[1]]),
        month: b[2],
        day: b[3],
        day_of_week: b[4],
    })
}

/// Encode a CosemTime to 4 bytes
pub fn encode_time(t: &CosemTime) -> [u8; 4] {
    [t.hour, t.minute, t.second, t.hundredths]
}

/// Decode a CosemTime from 4 bytes
pub fn decode_time(b: &[u8]) -> Option<CosemTime> {
    if b.len() < 4 { return None; }
    Some(CosemTime {
        hour: b[0],
        minute: b[1],
        second: b[2],
        hundredths: b[3],
    })
}

/// Encode a CosemDateTime to 12 bytes
pub fn encode_datetime(dt: &CosemDateTime) -> [u8; 12] {
    let mut buf = [0u8; 12];
    buf[0] = (dt.date.year >> 8) as u8;
    buf[1] = (dt.date.year & 0xFF) as u8;
    buf[2] = dt.date.month;
    buf[3] = dt.date.day;
    buf[4] = dt.date.day_of_week;
    buf[5] = dt.time.hour;
    buf[6] = dt.time.minute;
    buf[7] = dt.time.second;
    buf[8] = dt.time.hundredths;
    buf[9] = (dt.deviation >> 8) as u8;
    buf[10] = (dt.deviation & 0xFF) as u8;
    buf[11] = dt.clock_status;
    buf
}

/// Decode a CosemDateTime from 12 bytes
pub fn decode_datetime(b: &[u8]) -> Option<CosemDateTime> {
    if b.len() < 12 { return None; }
    Some(CosemDateTime {
        date: CosemDate {
            year: u16::from_be_bytes([b[0], b[1]]),
            month: b[2],
            day: b[3],
            day_of_week: b[4],
        },
        time: CosemTime {
            hour: b[5],
            minute: b[6],
            second: b[7],
            hundredths: b[8],
        },
        deviation: i16::from_be_bytes([b[9], b[10]]),
        clock_status: b[11],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_roundtrip() {
        let d = CosemDate { year: 2024, month: 6, day: 15, day_of_week: 6 };
        let bytes = encode_date(&d);
        let decoded = decode_date(&bytes).unwrap();
        assert_eq!(decoded, d);
    }

    #[test]
    fn test_time_roundtrip() {
        let t = CosemTime { hour: 14, minute: 30, second: 45, hundredths: 50 };
        let bytes = encode_time(&t);
        let decoded = decode_time(&bytes).unwrap();
        assert_eq!(decoded, t);
    }

    #[test]
    fn test_datetime_roundtrip() {
        let dt = CosemDateTime {
            date: CosemDate { year: 2024, month: 1, day: 1, day_of_week: 1 },
            time: CosemTime { hour: 0, minute: 0, second: 0, hundredths: 0 },
            deviation: 480,
            clock_status: 0,
        };
        let bytes = encode_datetime(&dt);
        let decoded = decode_datetime(&bytes).unwrap();
        assert_eq!(decoded, dt);
    }
}
