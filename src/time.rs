use std::cmp::Ordering;

use crate::enums::{Day, MaybeRecognized};

#[derive(Debug, PartialEq, Eq)]
pub struct Time {
    pub season: u32,
    pub ascending_days: Vec<(DayEquivalent, u16)>
}

#[derive(Debug, PartialEq, Eq)]
pub struct DayEquivalent {
    pub day: u16,
    pub offset: u8
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
            o @ _ => o
        }
    }
}
impl DayEquivalent {
    pub fn new(season: u32, day: &MaybeRecognized<Day>) -> Option<Self> {
        match season {
            0..=2 => match day {
                MaybeRecognized::NotRecognized(_) => None,
                MaybeRecognized::Recognized(Day::Day(day)) => Some(DayEquivalent { day: *day, offset: 0 }),
                MaybeRecognized::Recognized(Day::SuperstarBreak) =>  Some(DayEquivalent { day: 120, offset: 255 }),
                MaybeRecognized::Recognized(Day::SuperstarDay(offset)) => Some(DayEquivalent { day: 120, offset: offset + 1 }),
                MaybeRecognized::Recognized(Day::Holiday) => None
            }
            _ => None
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Breakpoints {
    Season1EnchantmentChange,
    S1AttributeEqualChange,
    S2D152,
    S2D169,
}
impl Breakpoints {
    fn ascending_transition_time(self) -> Time {
        match self {
            Breakpoints::Season1EnchantmentChange => Time {
                season: 1,
                ascending_days: vec![
                    (DayEquivalent { day: 120, offset: 255 }, 0),
                ]
            },
            Breakpoints::S1AttributeEqualChange => Time { 
                season: 1, ascending_days: vec![
                    (DayEquivalent {day: 215, offset: 0}, 0)
                ]
            },
            Breakpoints::S2D152 => Time {
                season: 2, 
                ascending_days: vec![
                    (DayEquivalent { day: 152, offset: 0 }, 0),
                ]
            },
            Breakpoints::S2D169 => Time { 
                season: 2, 
                ascending_days: vec![
                    (DayEquivalent { day: 168, offset: 0 }, 584),
                    (DayEquivalent { day: 169, offset: 0 }, 94),
                ]
            }
        }
    }
    pub fn before(&self, season: u32, day: &MaybeRecognized<Day>, event_index: Option<u16>) -> bool {
        let event_index = event_index.unwrap_or(0);
        let day = DayEquivalent::new(season, day);

        let transition = self.ascending_transition_time();

        match season.cmp(&transition.season) {
            Ordering::Less => true, // earlier season is before
            Ordering::Greater => false, // later season is after
            Ordering::Equal => match day {
                None => return false, // Assume unknown days are at end of season, and therefore after
                Some(day) => {
                    // Because of overflow, transition happens on multiple days
                    for (transition_day, transition_event_index) in transition.ascending_days {
                        match day.cmp(&transition_day) {
                            Ordering::Greater => (), // Move on to check the next day in the transition period
                            Ordering::Equal => match event_index.cmp(&transition_event_index) {
                                Ordering::Greater | Ordering::Equal => return false,
                                Ordering::Less => return true,
                            },
                            Ordering::Less => return true // Before a transition day, so before (only works because its in ascending order)
                        }
                    }
                    false // After the transition point, so after
                }
            }
        }
    }
    pub fn after(&self, season: u32, day: &MaybeRecognized<Day>, event_index: Option<u16>) -> bool {
        !self.before(season, day, event_index)
    }
}


#[cfg(test)]
mod test {
    use crate::{enums::{Day, MaybeRecognized}, time::Breakpoints};

    #[test]
    fn break_point_test() {
        assert!(Breakpoints::S2D169.before(1, &MaybeRecognized::Recognized(Day::Day(255)), Some(5)));
        assert!(Breakpoints::S2D169.before(2, &MaybeRecognized::Recognized(Day::Day(5)), Some(5)));
        assert!(Breakpoints::S2D169.before(2, &MaybeRecognized::Recognized(Day::Day(168)), Some(583)));
        assert!(Breakpoints::S2D169.after(2, &MaybeRecognized::Recognized(Day::Day(168)), Some(584)));
        assert!(Breakpoints::S2D169.before(2, &MaybeRecognized::Recognized(Day::Day(169)), Some(93)));
        assert!(Breakpoints::S2D169.after(2, &MaybeRecognized::Recognized(Day::Day(169)), Some(94)));
        assert!(Breakpoints::S2D169.after(2, &MaybeRecognized::Recognized(Day::Day(200)), Some(5)));
        assert!(Breakpoints::S2D169.after(3, &MaybeRecognized::Recognized(Day::Day(255)), Some(5)));
    }
}