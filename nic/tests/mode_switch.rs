use nic::{test::utils::set_ws0, watering::modes::ModeIdx};

#[test]
fn mode_switching() {
    let mut ws = set_ws0(0, None, None).unwrap();
    assert_eq!(ws.water_state.active_mode, ModeIdx::Auto);

    ws.water_state.switch_mode(ModeIdx::Manual);
    assert_eq!(ws.water_state.active_mode, ModeIdx::Manual);
}

#[test]
fn all_mode_transitions() {
    let mut ws = set_ws0(0, None, None).unwrap();
    // Initially in Auto mode
    assert_eq!(ws.water_state.active_mode, ModeIdx::Auto);

    // Transition from Auto -> Manual
    ws.water_state.switch_mode(ModeIdx::Manual);
    assert_eq!(ws.water_state.active_mode, ModeIdx::Manual);

    // Transition from Manual -> Wizard
    ws.water_state.switch_mode(ModeIdx::Wizard);
    assert_eq!(ws.water_state.active_mode, ModeIdx::Wizard);

    // Transition from Wizard -> Auto
    ws.water_state.switch_mode(ModeIdx::Auto);
    assert_eq!(ws.water_state.active_mode, ModeIdx::Auto);

    // Additional transitions to verify no unexpected behavior:
    // Auto -> Wizard
    ws.water_state.switch_mode(ModeIdx::Wizard);
    assert_eq!(ws.water_state.active_mode, ModeIdx::Wizard);

    // Wizard -> Manual
    ws.water_state.switch_mode(ModeIdx::Manual);
    assert_eq!(ws.water_state.active_mode, ModeIdx::Manual);

    // Manual -> Auto
    ws.water_state.switch_mode(ModeIdx::Auto);
    assert_eq!(ws.water_state.active_mode, ModeIdx::Auto);
}
