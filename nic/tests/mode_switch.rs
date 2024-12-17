use nic::{
    test::utils::{mock_cfg::mock_cfg, set_app_and_ws0},
    watering::modes::Mode,
};

#[test]
fn mode_switching() {
    let cfg = mock_cfg();
    let (_app, mut ws) = set_app_and_ws0(0, None, cfg.watering).unwrap();
    assert_eq!(ws.sm.current_mode, Mode::Auto);

    ws.sm.trans_change_mode(Mode::Manual);
    assert_eq!(ws.sm.current_mode, Mode::Manual);
}

#[test]
fn all_mode_transitions() {
    let cfg = mock_cfg();
    let (_app, mut ws) = set_app_and_ws0(0, None, cfg.watering).unwrap();
    // Initially in Auto mode
    assert_eq!(ws.sm.current_mode, Mode::Auto);

    // Transition from Auto -> Manual
    ws.sm.trans_change_mode(Mode::Manual);
    assert_eq!(ws.sm.current_mode, Mode::Manual);

    // Transition from Manual -> Wizard
    ws.sm.trans_change_mode(Mode::Wizard);
    assert_eq!(ws.sm.current_mode, Mode::Wizard);

    // Transition from Wizard -> Auto
    ws.sm.trans_change_mode(Mode::Auto);
    assert_eq!(ws.sm.current_mode, Mode::Auto);

    // Additional transitions to verify no unexpected behavior:
    // Auto -> Wizard
    ws.sm.trans_change_mode(Mode::Wizard);
    assert_eq!(ws.sm.current_mode, Mode::Wizard);

    // Wizard -> Manual
    ws.sm.trans_change_mode(Mode::Manual);
    assert_eq!(ws.sm.current_mode, Mode::Manual);

    // Manual -> Auto
    ws.sm.trans_change_mode(Mode::Auto);
    assert_eq!(ws.sm.current_mode, Mode::Auto);
}
