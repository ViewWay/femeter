//!
//! Interface Class 20: Activity Calendar
//!
//! Reference: Blue Book Part 2 §5.20
//!
//! Activity Calendar defines seasons, weeks, and days for tariff scheduling.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM IC 20: Activity Calendar
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | calendar_name | 2 | visible-string | static |
/// | seasons | 3 | array of structure | static |
/// | weeks | 4 | array of structure | static |
/// | days | 5 | array of structure | static |
/// | active_calendar_name | 6 | visible-string | dynamic |
/// | season_profile_active | 7 | structure | dynamic |
/// | week_profile_active | 8 | structure | dynamic |
/// | day_profile_active | 9 | structure | dynamic |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | change_calendar_name | 1 | Change the calendar name |
#[derive(Debug, Clone)]
pub struct ActivityCalendar {
    logical_name: ObisCode,
    calendar_name: DlmsType,
    seasons: DlmsType,
    weeks: DlmsType,
    days: DlmsType,
    active_calendar_name: DlmsType,
    season_profile_active: DlmsType,
    week_profile_active: DlmsType,
    day_profile_active: DlmsType,
}

impl ActivityCalendar {
    /// Create a new Activity Calendar object
    pub fn new(logical_name: ObisCode, calendar_name: DlmsType) -> Self {
        Self {
            logical_name,
            calendar_name,
            seasons: DlmsType::Array(alloc::vec![]),
            weeks: DlmsType::Array(alloc::vec![]),
            days: DlmsType::Array(alloc::vec![]),
            active_calendar_name: DlmsType::VisibleString(alloc::vec![]),
            season_profile_active: DlmsType::Null,
            week_profile_active: DlmsType::Null,
            day_profile_active: DlmsType::Null,
        }
    }
}

impl CosemClass for ActivityCalendar {
    const CLASS_ID: u16 = 20;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        9
    }

    fn method_count() -> u8 {
        1
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.calendar_name.clone()),
            3 => Ok(self.seasons.clone()),
            4 => Ok(self.weeks.clone()),
            5 => Ok(self.days.clone()),
            6 => Ok(self.active_calendar_name.clone()),
            7 => Ok(self.season_profile_active.clone()),
            8 => Ok(self.week_profile_active.clone()),
            9 => Ok(self.day_profile_active.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 3 | 4 | 5 => Err(CosemError::ReadOnly),
            2 => {
                self.calendar_name = value;
                Ok(())
            }
            6 => Err(CosemError::ReadOnly),
            7 => Err(CosemError::ReadOnly),
            8 => Err(CosemError::ReadOnly),
            9 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::Null), // change_calendar_name
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_calendar_class_id() {
        let ac = ActivityCalendar::new(
            ObisCode::new(0, 0, 98, 0, 2, 255),
            DlmsType::VisibleString(alloc::vec![]),
        );
        assert_eq!(ActivityCalendar::CLASS_ID, 20);
        assert_eq!(ActivityCalendar::attribute_count(), 9);
    }
}
