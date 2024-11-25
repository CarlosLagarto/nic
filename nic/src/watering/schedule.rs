use chrono::NaiveTime;

#[derive(Debug, Clone)]
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
