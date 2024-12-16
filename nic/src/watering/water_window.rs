use crate::utils::sod;

#[derive(Debug, Clone, Copy)]
pub struct WaterWin {
    pub hour_start: i64,    // Start time in seconds from the start of the base day
    pub duration_secs: i64, // Duration in seconds (can span across days)
    pub day_start_time: i64,
    pub day_end_time: i64,
}

impl WaterWin {
    /// Create a new timeframe with a start hour and duration in hours.
    pub fn new(current_time: i64, hour_start: i64, duration_hours: i64) -> Self {
        let day_start_time = sod(current_time) + hour_start * 3600;
        let duration_secs = duration_hours * 3600;
        let day_end_time = day_start_time + duration_secs - 1;
        Self { hour_start, duration_secs, day_start_time, day_end_time }
    }

    pub fn next_mut(&mut self) {
        self.day_start_time += 86_400;
        self.day_end_time += 86_400;
    }

    pub fn next(&self) -> Self {
        let mut new_tf = *self;
        new_tf.next_mut();
        new_tf
    }

    pub fn roll_window(&mut self, current_time: i64) {
        if current_time > self.day_end_time {
            self.next_mut();
        }
    }

    /// Check if a given `current_time` falls within the allowed timeframe,
    /// considering possible cross-day spans.
    pub fn is_within(&self, time: i64) -> bool {
        time >= self.day_start_time && time <= self.day_end_time
    }

    pub fn is_within_or_future(&self, time: i64) -> bool {
        if time >= self.day_start_time && time <= self.day_end_time {
            true
        } else if time > self.day_end_time {
            (time - self.day_start_time) % 86_400 < (self.day_end_time - self.day_start_time + 1)
        } else {
            false
        }
    }
}

#[cfg(test)]
pub mod tests {
    use chrono::{TimeZone, Utc};

    use crate::{utils::sod, watering::water_window::WaterWin};

    #[test]
    fn allowed_timeframe_same_day() {
        let curr_time = Utc.with_ymd_and_hms(2024, 11, 25, 6, 0, 0).unwrap().timestamp();
        // Example: Define a timeframe from 6:00 AM to 8:00 AM (2 hours)
        let tf = WaterWin::new(curr_time, 6, 2);

        // Verify the converted start and end times
        assert_eq!(tf.day_start_time, Utc.with_ymd_and_hms(2024, 11, 25, 6, 0, 0).unwrap().timestamp());
        assert_eq!(tf.day_end_time, Utc.with_ymd_and_hms(2024, 11, 25, 7, 59, 59).unwrap().timestamp());

        // Test if a time is within the timeframe
        let test_time = Utc.with_ymd_and_hms(2024, 11, 25, 7, 0, 0).unwrap().timestamp();
        assert!(tf.is_within(test_time));

        // Test a time outside the timeframe (after end time)
        let outside_time_after = Utc.with_ymd_and_hms(2024, 11, 25, 9, 0, 0).unwrap().timestamp();
        assert!(!tf.is_within(outside_time_after));

        // Test a time outside the timeframe (before start time)
        let outside_time_before = Utc.with_ymd_and_hms(2024, 11, 25, 5, 0, 0).unwrap().timestamp();
        assert!(!tf.is_within(outside_time_before));
    }

    #[test]
    fn allowed_timeframe_cross_day() {
        let curr_time = Utc.with_ymd_and_hms(2024, 11, 25, 6, 0, 0).unwrap().timestamp();
        // Example: Define a timeframe from 6:00 AM to 8:00 AM (2 hours)
        let tf = WaterWin::new(curr_time, 23, 2);

        // Verify the converted start and end times
        assert_eq!(tf.day_start_time, Utc.with_ymd_and_hms(2024, 11, 25, 23, 0, 0).unwrap().timestamp());
        assert_eq!(tf.day_end_time, Utc.with_ymd_and_hms(2024, 11, 26, 0, 59, 59).unwrap().timestamp());

        // Test if a time is within the timeframe
        let test_time = Utc.with_ymd_and_hms(2024, 11, 25, 23, 0, 0).unwrap().timestamp();
        assert!(tf.is_within(test_time));

        // Test a time outside the timeframe (after end time)
        let outside_time_after = Utc.with_ymd_and_hms(2024, 11, 26, 2, 0, 0).unwrap().timestamp();
        assert!(!tf.is_within(outside_time_after));

        // Test a time outside the timeframe (before start time)
        let outside_time_before = Utc.with_ymd_and_hms(2024, 11, 25, 5, 0, 0).unwrap().timestamp();
        assert!(!tf.is_within(outside_time_before));
    }

    #[test]
    fn waterwin_new() {
        let fixed_time = Utc.with_ymd_and_hms(2023, 12, 25, 0, 0, 0).unwrap().timestamp();
        let hour_start = 6; // 6 AM
        let duration_hours = 12; // 12-hour window

        let waterwin = WaterWin::new(fixed_time, hour_start, duration_hours);

        assert_eq!(waterwin.hour_start, hour_start);
        assert_eq!(waterwin.duration_secs, duration_hours * 3600);
        assert_eq!(waterwin.day_start_time, sod(fixed_time) + hour_start * 3600);
        assert_eq!(waterwin.day_end_time, waterwin.day_start_time + waterwin.duration_secs - 1);
    }

    #[test]
    fn waterwin_next() {
        let fixed_time = Utc.with_ymd_and_hms(2023, 12, 25, 0, 0, 0).unwrap().timestamp();
        let waterwin = WaterWin::new(fixed_time, 6, 12);

        let next_win = waterwin.next();

        assert_eq!(next_win.day_start_time, waterwin.day_start_time + 86_400); // One day later
        assert_eq!(next_win.day_end_time, waterwin.day_end_time + 86_400);
    }

    #[test]
    fn waterwin_is_within() {
        let fixed_time = Utc.with_ymd_and_hms(2023, 12, 25, 0, 0, 0).unwrap().timestamp();
        let waterwin = WaterWin::new(fixed_time, 6, 12); // 6 AM to 6 PM

        let within_start = waterwin.day_start_time + 1;
        let within_end = waterwin.day_end_time - 1;

        assert!(waterwin.is_within(within_start));
        assert!(waterwin.is_within(within_end));
        assert!(!waterwin.is_within(waterwin.day_start_time - 1));
        assert!(!waterwin.is_within(waterwin.day_end_time + 1));
    }
}
