use chrono::NaiveTime;

#[derive(Debug, Clone, Copy)]
pub struct AllowedTimeframe {
    pub start: NaiveTime,
    pub end: NaiveTime,
}

impl AllowedTimeframe {
    pub fn is_within(&self, current_time: NaiveTime) -> bool {
        if self.start <= self.end {
            current_time >= self.start && current_time <= self.end
        } else {
            // Handles timeframes that span midnight (e.g., 10 PM to 6 AM)
            current_time >= self.start || current_time <= self.end
        }
    }
}

#[test]
fn test_allowed_timeframe() {
    let timeframe = AllowedTimeframe {
        start: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
        end: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
    };

    assert!(timeframe.is_within(NaiveTime::from_hms_opt(23, 0, 0).unwrap())); // 11:00 PM
    assert!(timeframe.is_within(NaiveTime::from_hms_opt(5, 30, 0).unwrap())); // 5:30 AM
    assert!(!timeframe.is_within(NaiveTime::from_hms_opt(7, 0, 0).unwrap())); // 7:00 AM
}