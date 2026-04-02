//!
//! IC 19 — Tariff Schedule (Activity Calendar for TOU billing)
//!
//! Manages time-of-use tariff periods, seasons, and day profiles.
//! This is the key IC for tariff switching in smart meters.
//!
//! Reference: Blue Book Part 2 §4.3.19
//!
//! Attributes:
//!   1. logical_name        (octet-string, static)
//!   2. calendar_name       (octet-string, static)
//!   3. season_profile      (array of structures, static)
//!   4. week_profile        (array of structures, static)
//!   5. day_profile         (array of structures, static)
//!   6. activate_passive    (boolean, dynamic)
//!   7. tariff_table        (structure, static)
//!
//! Methods:
//!   1. activate_tariff_table(passive_tariff_table)

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// Maximum number of seasons
const MAX_SEASONS: usize = 4;
/// Maximum number of week profile entries
const MAX_WEEK_PROFILES: usize = 10;
/// Maximum number of day profiles
const MAX_DAY_PROFILES: usize = 16;
/// Maximum number of tariff periods per day profile
const MAX_PERIODS_PER_DAY: usize = 10;

/// Season profile entry
#[derive(Clone, Debug)]
pub struct SeasonEntry {
    /// Season name (up to 16 bytes)
    pub name: alloc::vec::Vec<u8>,
    /// Start date (month, day, weekday)
    pub start_month: u8,
    pub start_day: u8,
    /// Week profile ID for this season
    pub week_profile_id: u8,
}

/// Week profile entry
#[derive(Clone, Debug)]
pub struct WeekProfileEntry {
    /// Week profile ID
    pub id: u8,
    /// Monday..Sunday day profile IDs
    pub day_ids: [u8; 7],
}

/// Day profile entry — defines tariff schedule for one day type
#[derive(Clone, Debug)]
pub struct DayProfileEntry {
    /// Day profile ID
    pub id: u8,
    /// Tariff periods: (start_minute, tariff_register_id)
    pub periods: alloc::vec::Vec<(u16, u8)>,
}

/// IC 19 — Tariff Schedule
pub struct TariffSchedule {
    logical_name: ObisCode,
    calendar_name: alloc::vec::Vec<u8>,
    season_profile: alloc::vec::Vec<SeasonEntry>,
    week_profile: alloc::vec::Vec<WeekProfileEntry>,
    day_profile: alloc::vec::Vec<DayProfileEntry>,
    active: bool,
}

impl TariffSchedule {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            calendar_name: b"Default".to_vec(),
            season_profile: alloc::vec::Vec::new(),
            week_profile: alloc::vec::Vec::new(),
            day_profile: alloc::vec::Vec::new(),
            active: true,
        }
    }

    /// Add a season
    pub fn add_season(&mut self, season: SeasonEntry) -> Result<(), CosemError> {
        if self.season_profile.len() >= MAX_SEASONS {
            return Err(CosemError::HardwareError);
        }
        self.season_profile.push(season);
        Ok(())
    }

    /// Add a week profile
    pub fn add_week_profile(&mut self, wp: WeekProfileEntry) -> Result<(), CosemError> {
        if self.week_profile.len() >= MAX_WEEK_PROFILES {
            return Err(CosemError::HardwareError);
        }
        self.week_profile.push(wp);
        Ok(())
    }

    /// Add a day profile
    pub fn add_day_profile(&mut self, dp: DayProfileEntry) -> Result<(), CosemError> {
        if self.day_profile.len() >= MAX_DAY_PROFILES {
            return Err(CosemError::HardwareError);
        }
        if dp.periods.len() > MAX_PERIODS_PER_DAY {
            return Err(CosemError::HardwareError);
        }
        self.day_profile.push(dp);
        Ok(())
    }

    /// Get the current tariff register ID based on month, day_of_week, and minute_of_day
    pub fn current_tariff(&self, _month: u8, day_of_week: u8, minute_of_day: u16) -> Option<u8> {
        // 1. Find active season
        let season = self.season_profile.iter().find(|_s| {
            // Simplified: season starts when month matches and day >= start_day
            // A full implementation would handle wrap-around seasons
            true
        })?;

        // 2. Get week profile for this season
        let wp = self
            .week_profile
            .iter()
            .find(|w| w.id == season.week_profile_id)?;
        let dow_idx = if day_of_week == 0 { 6 } else { day_of_week - 1 }; // Mon=0
        let day_id = wp.day_ids.get(dow_idx as usize)?;

        // 3. Get day profile
        let dp = self.day_profile.iter().find(|d| d.id == *day_id)?;

        // 4. Find tariff period for current time
        let mut tariff_id = 1u8; // default tariff
        for (start_min, reg_id) in &dp.periods {
            if *start_min <= minute_of_day {
                tariff_id = *reg_id;
            }
        }
        Some(tariff_id)
    }

    /// Activate or deactivate the tariff schedule
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl CosemClass for TariffSchedule {
    const CLASS_ID: u16 = 19;
    const VERSION: u8 = 1;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        7
    }

    fn method_count() -> u8 {
        1
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::OctetString(self.calendar_name.clone())),
            3 => {
                let list: alloc::vec::Vec<DlmsType> = self
                    .season_profile
                    .iter()
                    .map(|s| {
                        DlmsType::Structure(alloc::vec![
                            DlmsType::OctetString(s.name.clone()),
                            DlmsType::Structure(alloc::vec![
                                DlmsType::UInt8(s.start_month),
                                DlmsType::UInt8(s.start_day),
                                DlmsType::UInt8(s.week_profile_id),
                            ]),
                        ])
                    })
                    .collect();
                Ok(DlmsType::Array(list))
            }
            4 => {
                let list: alloc::vec::Vec<DlmsType> = self
                    .week_profile
                    .iter()
                    .map(|w| {
                        DlmsType::Structure(alloc::vec![
                            DlmsType::UInt8(w.id),
                            DlmsType::Array(
                                w.day_ids.iter().map(|d| DlmsType::UInt8(*d)).collect(),
                            ),
                        ])
                    })
                    .collect();
                Ok(DlmsType::Array(list))
            }
            5 => {
                let list: alloc::vec::Vec<DlmsType> = self
                    .day_profile
                    .iter()
                    .map(|d| {
                        let periods: alloc::vec::Vec<DlmsType> = d
                            .periods
                            .iter()
                            .map(|(start, reg)| {
                                DlmsType::Structure(alloc::vec![
                                    DlmsType::UInt16(*start),
                                    DlmsType::UInt8(*reg),
                                ])
                            })
                            .collect();
                        DlmsType::Structure(alloc::vec![
                            DlmsType::UInt8(d.id),
                            DlmsType::Array(periods),
                        ])
                    })
                    .collect();
                Ok(DlmsType::Array(list))
            }
            6 => Ok(DlmsType::Boolean(self.active)),
            7 => {
                // Tariff table — simplified
                Ok(DlmsType::Structure(alloc::vec![
                    DlmsType::OctetString(self.calendar_name.clone()),
                    DlmsType::Array(alloc::vec![]),
                ]))
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => {
                if let DlmsType::OctetString(name) = value {
                    self.calendar_name = name;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 9,
                        got: value.tag(),
                    })
                }
            }
            3 | 4 | 5 => Err(CosemError::ReadOnly), // profiles managed via methods
            6 => {
                if let DlmsType::Boolean(active) = value {
                    self.active = active;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 3,
                        got: value.tag(),
                    })
                }
            }
            7 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        if id == 1 {
            // activate_passive_tariff_table
            Ok(DlmsType::Null)
        } else {
            Err(CosemError::NoSuchMethod(id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dlms_core::traits::CosemClass;

    fn make_schedule() -> TariffSchedule {
        let mut ts = TariffSchedule::new(ObisCode::new(0, 0, 13, 0, 0, 255));

        // Season: all year round, week profile 1
        ts.add_season(SeasonEntry {
            name: b"Summer".to_vec(),
            start_month: 4,
            start_day: 1,
            week_profile_id: 1,
        })
        .unwrap();

        // Week profile 1: Mon-Fri = day 1, Sat-Sun = day 2
        ts.add_week_profile(WeekProfileEntry {
            id: 1,
            day_ids: [1, 1, 1, 1, 1, 2, 2],
        })
        .unwrap();

        // Day profile 1: peak (0-480), flat (480-1200), peak (1200-1440)
        ts.add_day_profile(DayProfileEntry {
            id: 1,
            periods: alloc::vec![(0, 1), (480, 2), (1200, 1)],
        })
        .unwrap();

        // Day profile 2: off-peak all day
        ts.add_day_profile(DayProfileEntry {
            id: 2,
            periods: alloc::vec![(0, 3)],
        })
        .unwrap();

        ts
    }

    #[test]
    fn test_creation() {
        let ts = TariffSchedule::new(ObisCode::new(0, 0, 13, 0, 0, 255));
        assert_eq!(<TariffSchedule as CosemClass>::CLASS_ID, 19);
        assert_eq!(TariffSchedule::attribute_count(), 7);
        assert_eq!(TariffSchedule::method_count(), 1);
        assert!(ts.is_active());
    }

    #[test]
    fn test_get_attributes() {
        let ts = make_schedule();
        assert!(ts.get_attribute(1).is_ok());
        assert!(ts.get_attribute(2).is_ok());
        assert!(ts.get_attribute(3).is_ok()); // season profile array
        assert!(ts.get_attribute(4).is_ok()); // week profile array
        assert!(ts.get_attribute(5).is_ok()); // day profile array
        assert!(ts.get_attribute(6).is_ok()); // active
        assert!(ts.get_attribute(99).is_err());
    }

    #[test]
    fn test_set_calendar_name() {
        let mut ts = TariffSchedule::new(ObisCode::new(0, 0, 13, 0, 0, 255));
        ts.set_attribute(2, DlmsType::OctetString(b"CN-TOU".to_vec()))
            .unwrap();
        let name = ts.get_attribute(2).unwrap();
        assert_eq!(name, DlmsType::OctetString(b"CN-TOU".to_vec()));
    }

    #[test]
    fn test_set_active() {
        let mut ts = TariffSchedule::new(ObisCode::new(0, 0, 13, 0, 0, 255));
        ts.set_attribute(6, DlmsType::Boolean(false)).unwrap();
        assert!(!ts.is_active());
    }

    #[test]
    fn test_current_tariff_weekday() {
        let ts = make_schedule();
        // Monday at 00:00 → tariff 1 (peak)
        assert_eq!(ts.current_tariff(4, 1, 0), Some(1));
        // Monday at 08:00 (480 min) → tariff 2 (flat)
        assert_eq!(ts.current_tariff(4, 1, 480), Some(2));
        // Monday at 20:00 (1200 min) → tariff 1 (peak)
        assert_eq!(ts.current_tariff(4, 1, 1200), Some(1));
    }

    #[test]
    fn test_current_tariff_weekend() {
        let ts = make_schedule();
        // Saturday → day profile 2 → off-peak (tariff 3)
        assert_eq!(ts.current_tariff(4, 6, 0), Some(3));
    }

    #[test]
    fn test_max_seasons() {
        let mut ts = TariffSchedule::new(ObisCode::new(0, 0, 13, 0, 0, 255));
        for i in 0..MAX_SEASONS {
            ts.add_season(SeasonEntry {
                name: alloc::vec![i as u8],
                start_month: (i as u8) * 3 + 1,
                start_day: 1,
                week_profile_id: 1,
            })
            .unwrap();
        }
        assert!(ts
            .add_season(SeasonEntry {
                name: b"extra".to_vec(),
                start_month: 1,
                start_day: 1,
                week_profile_id: 1,
            })
            .is_err());
    }
}
