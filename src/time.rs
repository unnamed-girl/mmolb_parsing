use chrono::{DateTime, NaiveDate, Utc};
use std::cmp::Ordering;

use crate::enums::Day;

#[derive(Debug, PartialEq, Eq)]
pub struct Time {
    pub season: u32,
    /// Vec of (DayEquivalent, EventIndex), which is the first event after the breakpoint.
    pub ascending_days: Vec<(DayEquivalent, u16)>,
}

impl Time {
    /// Is the time before self
    pub fn before(&self, season: u32, day: Option<Day>, event_index: Option<u16>) -> bool {
        let event_index = event_index.unwrap_or(0);
        let day = day.map(|day| DayEquivalent::new(season, day));

        match season.cmp(&self.season) {
            Ordering::Less => true,     // earlier season is before
            Ordering::Greater => false, // later season is after
            Ordering::Equal => match day {
                None => false, // Assume unknown days are at end of season, and therefore after
                Some(day) => {
                    // Because of overflow, transition happens on multiple days
                    for (transition_day, transition_event_index) in &self.ascending_days {
                        match day.cmp(transition_day) {
                            Ordering::Greater => (), // Move on to check the next day in the transition period
                            Ordering::Equal => match event_index.cmp(transition_event_index) {
                                Ordering::Greater | Ordering::Equal => return false,
                                Ordering::Less => return true,
                            },
                            Ordering::Less => return true, // Before a transition day, so before (only works because its in ascending order)
                        }
                    }
                    false // After the transition point, so after
                }
            },
        }
    }

    /// Is the time after self
    pub fn after(&self, season: u32, day: Option<Day>, event_index: Option<u16>) -> bool {
        !self.before(season, day, event_index)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DayEquivalent {
    pub day: u16,
    pub offset: u8,
}
impl PartialOrd for DayEquivalent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for DayEquivalent {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.day.cmp(&other.day) {
            Ordering::Equal => self.offset.cmp(&other.offset),
            o => o,
        }
    }
}
impl DayEquivalent {
    pub fn new(_season: u32, day: Day) -> Self {
        match day {
            Day::Day(day) => DayEquivalent { day, offset: 0 },
            Day::SuperstarBreak => DayEquivalent {
                day: 120,
                offset: 255,
            },
            Day::SuperstarDay(offset) => DayEquivalent {
                day: 120,
                offset: offset + 1,
            },
            Day::PostseasonRound(round) => DayEquivalent {
                day: 254,
                offset: round,
            },
            Day::PostseasonPreview => DayEquivalent {
                day: 254,
                offset: 0,
            },
            Day::Preseason => DayEquivalent { day: 0, offset: 0 },
            Day::Election => DayEquivalent {
                day: 255,
                offset: 1,
            },
            Day::Holiday => DayEquivalent {
                day: 255,
                offset: 2,
            },
            Day::Event => DayEquivalent {
                day: 255,
                offset: 2,
            },
            Day::SpecialEvent => DayEquivalent {
                day: 255,
                offset: 2,
            },
            Day::SuperstarGame => DayEquivalent {
                day: 120,
                offset: 1,
            },
            Day::Offseason => DayEquivalent {
                day: 255,
                offset: 2,
            },
        }
    }
}

/// To get resolution within a day for a feed event, compare timestamps
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Timestamp {
    Season3RecomposeChange,
}

impl Timestamp {
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Timestamp::Season3RecomposeChange => NaiveDate::from_ymd_opt(2025, 7, 14)
                .expect("hardcoded date")
                .and_hms_opt(11, 30, 0)
                .expect("hardcoded time")
                .and_utc(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Breakpoints {
    Season1EnchantmentChange,
    S1AttributeEqualChange,
    S2D152,
    S2D169,
    Season3,
    CheersGetEmoji,
    Season3PreSuperstarBreakUpdate,
    EternalBattle,
    Season5TenseChange,
    Season7WitherTenseChange,
    Season7SuccessfulContainPeriodFix,
    Season8ItemDiscardedMessageChange,
    Season10,
}
impl From<Breakpoints> for Time {
    fn from(value: Breakpoints) -> Self {
        value.ascending_transition_time()
    }
}
impl Breakpoints {
    pub fn season(season: u32) -> Time {
        Time {
            season,
            ascending_days: vec![(DayEquivalent { day: 0, offset: 0 }, 0)],
        }
    }
    fn ascending_transition_time(self) -> Time {
        match self {
            Breakpoints::Season1EnchantmentChange => Time {
                season: 1,
                ascending_days: vec![(
                    DayEquivalent {
                        day: 120,
                        offset: 255,
                    },
                    0,
                )],
            },
            Breakpoints::S1AttributeEqualChange => Time {
                season: 1,
                ascending_days: vec![(
                    DayEquivalent {
                        day: 215,
                        offset: 0,
                    },
                    0,
                )],
            },
            Breakpoints::S2D152 => Time {
                season: 2,
                ascending_days: vec![(
                    DayEquivalent {
                        day: 152,
                        offset: 0,
                    },
                    70,
                )],
            },
            Breakpoints::S2D169 => Time {
                season: 2,
                ascending_days: vec![
                    (
                        DayEquivalent {
                            day: 168,
                            offset: 0,
                        },
                        584,
                    ),
                    (
                        DayEquivalent {
                            day: 169,
                            offset: 0,
                        },
                        94,
                    ),
                ],
            },
            Breakpoints::Season3 => Time {
                season: 3,
                ascending_days: vec![(DayEquivalent { day: 0, offset: 0 }, 0)],
            },
            Breakpoints::CheersGetEmoji => Time {
                season: 3,
                ascending_days: vec![(DayEquivalent { day: 5, offset: 0 }, 330)],
            },
            Breakpoints::Season3PreSuperstarBreakUpdate => Time {
                season: 3,
                ascending_days: vec![(
                    DayEquivalent {
                        day: 112,
                        offset: 0,
                    },
                    0,
                )],
            },
            Breakpoints::EternalBattle => Time {
                season: 2,
                ascending_days: vec![(
                    DayEquivalent {
                        day: 255,
                        offset: 0,
                    },
                    0,
                )],
            },
            Breakpoints::Season5TenseChange => Time {
                season: 5,
                ascending_days: vec![(
                    DayEquivalent {
                        day: 863,
                        offset: 0,
                    },
                    0,
                )],
            },
            Breakpoints::Season7WitherTenseChange => Time {
                season: 7,
                ascending_days: vec![(DayEquivalent { day: 0, offset: 0 }, 0)],
            },
            Breakpoints::Season7SuccessfulContainPeriodFix => Time {
                season: 7,
                ascending_days: vec![(DayEquivalent { day: 46, offset: 0 }, 24)],
            },
            Breakpoints::Season8ItemDiscardedMessageChange => Time {
                season: 8,
                ascending_days: vec![(DayEquivalent { day: 0, offset: 0 }, 0)],
            },
            Breakpoints::Season10 => Time {
                season: 10,
                ascending_days: vec![(DayEquivalent { day: 0, offset: 0 }, 0)],
            },
        }
    }
    /// Is the time before the breakpoint
    pub fn before(&self, season: u32, day: Option<Day>, event_index: Option<u16>) -> bool {
        self.ascending_transition_time()
            .before(season, day, event_index)
    }
    /// Is the time after the breakpoint
    pub fn after(&self, season: u32, day: Option<Day>, event_index: Option<u16>) -> bool {
        !self.before(season, day, event_index)
    }
}

pub fn is_superstar_game(day: Option<Day>) -> bool {
    matches!(day, Some(Day::SuperstarDay(2)))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn break_point_test() {
        assert!(Breakpoints::S2D169.before(1, Some(Day::Day(255)), Some(5)));
        assert!(Breakpoints::S2D169.before(2, Some(Day::Day(5)), Some(5)));
        assert!(Breakpoints::S2D169.before(2, Some(Day::Day(168)), Some(583)));
        assert!(Breakpoints::S2D169.after(2, Some(Day::Day(168)), Some(584)));
        assert!(Breakpoints::S2D169.before(2, Some(Day::Day(169)), Some(93)));
        assert!(Breakpoints::S2D169.after(2, Some(Day::Day(169)), Some(94)));
        assert!(Breakpoints::S2D169.after(2, Some(Day::Day(200)), Some(5)));
        assert!(Breakpoints::S2D169.after(3, Some(Day::Day(255)), Some(5)));
    }
}
