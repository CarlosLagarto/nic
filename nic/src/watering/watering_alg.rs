use super::{
    ds::{DailyPlan, SectorInfo, WaterSector}, water_window::WaterWin, DAILY_PERCOLATION_FACTOR, SECTOR_TRANSITION_SECS
};
use crate::utils::get_week_day_from_ts;
use tracing::debug;

#[derive(Clone, Debug)]
pub enum ScheduleType {
    Weekday(chrono::Weekday), // For auto mode
    Date(i64),                // For wizard mode (specific dates)
}

#[derive(Clone, Debug)]
pub struct ScheduleEntry {
    pub schedule_type: ScheduleType,
    pub start_times: DailyPlan,
}

#[derive(Clone, Debug)]
pub struct Schedule {
    pub entries: Vec<ScheduleEntry>,
}

impl Schedule {
    pub fn new(entries: Vec<ScheduleEntry>) -> Self {
        Self { entries }
    }
}

pub fn adjust_daily_sector_progress(sectors: &mut [&mut SectorInfo], daily_et: f64, daily_rain: f64, new_week: bool) {
    let adjustment = daily_et - daily_rain + if new_week { 2.5 } else { 0. };
    let mut percolation;
    for sector in sectors.iter_mut() {
        percolation = calc_daily_percolation(sector).max(0.0);
        sector.progress = (sector.progress - adjustment - percolation).max(0.);
        debug!(
                "Sector {}: Adjusted progress by -{:.2} cm due to evapotranspiration, -{:.2} due to percolation and +{:.2} mm due to rain. New progress: {:.2} cm.",
                sector.id, daily_et, percolation, daily_rain, sector.progress
            );
    }
}

/// Calculate dialy percolation in the soil in cm
pub fn calc_daily_percolation(sector: &SectorInfo) -> f64 {
    sector.percolation_rate * DAILY_PERCOLATION_FACTOR
}

/// Calculate irrigation time in seconds
pub fn calc_irrigation_time(sector: &SectorInfo) -> Option<i64> {
    let remaining_target = sector.weekly_target - sector.progress; // Total water needed in cm
    if remaining_target <= 0. {
        return None; // No watering needed; target met
    }
    let irrigation_time = ((remaining_target / sector.sprinkler_debit) * 3600.0).ceil() as i64;
    Some(irrigation_time.min(sector.max_duration))
}

pub fn calc_daily_plan(sectors: &[SectorInfo], current_time: i64, timeframe: WaterWin) -> Vec<DailyPlan> {
    let remaining_days = calculate_remaining_days(current_time);
    let mut plans = generate_daily_plan(sectors, remaining_days, timeframe);
    plans.iter_mut().for_each(|daily_plan| {
        daily_plan.0.sort_by_key(|sector| sector.start);
    });
    plans
}

/// Is always called at new day (midnight), which means that when turned on, only will water next day morning.
/// If one needs immediate watering, should do a manual watering
#[allow(clippy::option_map_unit_fn)]  //complexity/readability.
pub fn generate_daily_plan(sectors: &[SectorInfo], remaining_days: i64, mut timeframe: WaterWin) -> Vec<DailyPlan> {
    let mut plans = Vec::with_capacity(2); // at max we have a morning and evening session

    // Clone sectors to modify their progress during calculation without altering original values
    let mut sectors = sectors.to_vec();
    for rem_days in (0..remaining_days).rev() {
        // Check if there's unmet target across all sectors
        if !sectors.iter().all(|sec| sec.weekly_target > sec.progress) {
            timeframe.next_mut();
            continue; // Skip this day if no sector needs watering
        }
        let (need_evening, mut daily_plan) = get_next_watering_for_day(&mut sectors, &mut timeframe, rem_days, true);
        daily_plan.take().map(|p| plans.push(p));
        // advance timeframe.  either will serve the next day at 22, and also the next morning if the evening whatering is not needed
        timeframe.next_mut();
        if need_evening {
            let (_, mut daily_plan) = get_next_watering_for_day(&mut sectors, &mut timeframe, rem_days, false);
            daily_plan.take().map(|p| plans.push(p));
        }
        if !plans.is_empty() {
            return plans;
        }
    }
    plans
}

fn get_next_watering_for_day(
    sectors: &mut [SectorInfo], timeframe: &mut WaterWin, remaining_days: i64, morning: bool,
) -> (bool, Option<DailyPlan>) {
    let mut daily_plan = DailyPlan::new();
    let mut need_evening = false;
    let mut water_time = if morning { timeframe.day_end_time } else { timeframe.day_start_time };
    let sector_iter: Box<dyn Iterator<Item = &mut SectorInfo>> =
        if morning { Box::new(sectors.iter_mut().rev()) } else { Box::new(sectors.iter_mut()) };

    for sector in sector_iter {
        // Calculate remaining weekly water needs for the sector
        let remaining_weekly_need = (sector.weekly_target - sector.progress).max(0.0);
        let daily_capacity = (sector.max_duration as f64 / 3600.0) * sector.sprinkler_debit;

        // Skip the sector if the (remaining days - 1) are sufficient to fulfill its needs
        if remaining_weekly_need <= daily_capacity * (remaining_days - 1) as f64 {
            continue;
        }
        if remaining_weekly_need > daily_capacity * remaining_days as f64 {
            need_evening = true;
        }

        let secs_irrigation_time = calc_irrigation_time(sector).unwrap_or(0);
        if secs_irrigation_time <= 300 {
            continue; // Skip sectors with negligible needs
        }

        let proposed_start =
            if morning { water_time - secs_irrigation_time - SECTOR_TRANSITION_SECS } else { water_time };

        daily_plan.0.push(WaterSector::new(sector.id, proposed_start, secs_irrigation_time));
        sector.progress += secs_irrigation_time as f64 * (sector.sprinkler_debit / 3600.0);

        if morning {
            water_time = proposed_start; // Move earlier for morning sessions
        } else {
            water_time += secs_irrigation_time + SECTOR_TRANSITION_SECS; // Move later for evening sessions
        }
    }
    (need_evening, (!daily_plan.0.is_empty()).then_some(daily_plan))
}

fn calculate_remaining_days(current_time: i64) -> i64 {
    7 - get_week_day_from_ts(current_time).num_days_from_sunday() as i64
}

#[cfg(test)]
mod test {

    use crate::watering::{ds::SectorInfo, watering_alg::*};
    use chrono::{TimeZone, Utc, Weekday};

    fn mock_sector(id: u32, weekly_target: f64, progress: f64, max_duration: i64, sprinkler_debit: f64) -> SectorInfo {
        SectorInfo { id, weekly_target, progress, max_duration, sprinkler_debit, ..Default::default() }
    }

    fn mock_sector_info(
        id: u32, weekly_target: f64, progress: f64, sprinkler_debit: f64, percolation_rate: f64, max_duration: i64,
    ) -> SectorInfo {
        SectorInfo { id, weekly_target, progress, sprinkler_debit, percolation_rate, max_duration, last_water: 0 }
    }

    #[tokio::test]
    async fn et_adjustments() {
        let mut sectors = vec![SectorInfo::build(1, 3., 1., 30 * 60, 0.5, 0.5, 0)];
        let secs = &mut sectors.iter_mut().collect::<Vec<&mut SectorInfo>>();
        adjust_daily_sector_progress(secs, 1., 0.5, false);
        assert!(sectors[0].progress == 0.5 - 1. + 0.5)
    }

    #[test]
    fn calc_irrigation_time_respects_max_duration() {
        let sector = mock_sector(1, 10.0, 5.0, 3600, 1.0); // Needs 5cm of water, 1cm/hr, max duration 1 hour
        let irrigation_time = calc_irrigation_time(&sector);
        assert_eq!(irrigation_time, Some(3600)); // Limited to 1 hour
    }

    #[test]
    fn calc_irrigation_time_does_not_exceed_needs() {
        let sector = mock_sector(1, 10.0, 9.5, 3600, 1.0); // Needs 0.5cm, 1cm/hr
        let irrigation_time = calc_irrigation_time(&sector);
        assert_eq!(irrigation_time, Some(1800)); // Only needs 0.5 hour
    }

    #[test]
    fn calculate_irrigation_time() {
        let sector = SectorInfo::build(1, 2.5, 1.0, 30 * 60, 1., 0.5, 0);

        // No progress yet
        let result = calc_irrigation_time(&sector);
        assert_eq!(result, Some(30 * 60)); // 1.5 cm at 1.0 cm/hour
    }

    #[test]
    fn daily_et_adjustment() {
        let mut sectors = vec![
            SectorInfo::build(1, 2.5, 1., 30 * 60, 1.5, 0., 0),
            SectorInfo::build(2, 1.8, 0.8, 20 * 60, 0.5, 0., 0),
        ];

        let daily_et = 0.3;
        let secs = &mut sectors.iter_mut().collect::<Vec<&mut SectorInfo>>();
        adjust_daily_sector_progress(secs, daily_et, 0., false);

        assert_eq!(sectors[0].progress, 1.2); // Reduced by 0.3
        assert_eq!(sectors[1].progress, 0.2); // Reduced by 0.3 but clamped to 0.2
    }

    #[test]
    fn test_calculate_remaining_days() {
        // we checked that this day is a wednesday
        let current_time = Utc.with_ymd_and_hms(2024, 12, 11, 22, 0, 0).unwrap().timestamp(); // 6:00 AM UTC
        let remaining_days = calculate_remaining_days(current_time);

        // Assuming today is Wednesday
        let expected_days = 7 - Weekday::Wed.num_days_from_sunday() as i64;
        assert_eq!(remaining_days, expected_days);
    }

    #[test]
    fn generate_weekly_plan_with_waterwin() {
        let sectors =
            vec![mock_sector_info(1, 10.0, 5.0, 2.0, 0.5, 3600), mock_sector_info(2, 15.0, 10.0, 1.5, 0.4, 3600)];
        let fixed_time = Utc.with_ymd_and_hms(2023, 12, 25, 0, 0, 0).unwrap().timestamp();
        let timeframe = WaterWin::new(fixed_time, 6, 12);

        let current_time = timeframe.day_start_time; // Fixed current time
        let remaining_days = calculate_remaining_days(current_time);
        let weekly_plan = generate_daily_plan(&sectors, remaining_days, timeframe);

        assert!(!weekly_plan.is_empty());
        if let Some(daily_plan) = weekly_plan.get(0) {
            assert!(!daily_plan.0.is_empty());
            assert!(daily_plan.0.iter().all(|sector| timeframe.is_within_or_future(sector.start)));
        }
    }

    #[test]
    fn test_get_next_watering_for_day() {
        let fixed_time = Utc.with_ymd_and_hms(2024, 12, 14, 2, 0, 0).unwrap().timestamp();
        let mut sectors =
            vec![mock_sector_info(1, 10.0, 9.0, 1.0, 0.1, 3600), mock_sector_info(2, 8.0, 7.5, 0.8, 0.2, 2700)];
        let mut timeframe = WaterWin::new(fixed_time, 6, 12);

        // Call the function for morning session
        let result_morning = get_next_watering_for_day(&mut sectors, &mut timeframe, 1, true);

        // Assert that a valid daily plan is returned for morning
        assert!(result_morning.1.is_some(), "Morning session should have a valid daily plan.");
        let daily_plan = result_morning.1.unwrap();
        assert!(!daily_plan.0.is_empty(), "Morning session should have watering tasks.");

        // Validate evening session
        let result_evening = get_next_watering_for_day(&mut sectors, &mut timeframe, 7, false);

        // Assert that the evening session is valid only if more progress is needed
        if sectors.iter().any(|sec| sec.weekly_target > sec.progress) {
            assert!(
                result_evening.1.is_some(),
                "Evening session should have a valid daily plan if targets remain unmet."
            );
            let daily_plan = result_evening.1.unwrap();
            assert!(!daily_plan.0.is_empty(), "Evening session should have watering tasks.");
        } else {
            assert!(result_evening.1.is_none(), "Evening session should be None if all targets are met.");
        }
    }

    #[test]
    fn test_calc_daily_plan_with_waterwin() {
        let sectors =
            vec![mock_sector_info(1, 10.0, 5.0, 2.0, 0.5, 3600), mock_sector_info(2, 15.0, 10.0, 1.5, 0.4, 3600)];
        let fixed_time = Utc.with_ymd_and_hms(2023, 12, 25, 0, 0, 0).unwrap().timestamp();
        let timeframe = WaterWin::new(fixed_time, 6, 12);
        let current_time = timeframe.day_start_time + 10;

        let daily_plan = calc_daily_plan(&sectors, current_time, timeframe);

        assert!(!daily_plan.is_empty());
        let daily_plan = daily_plan.get(0).unwrap();
        assert!(!daily_plan.0.is_empty());
    }
}
