use nic::watering::mode::ModeEnum;

use test_utilities::common::set_app_state;

#[tokio::test]
async fn test_mode_switching() {
    let app_state = set_app_state().await;

    assert!(matches!(
        *app_state.watering_system.active_mode.read().await,
        ModeEnum::Auto(_)
    ));

    let manual_mode = app_state.watering_system.manual_mode.read().await.clone();
    app_state
        .watering_system
        .switch_mode(ModeEnum::Manual(manual_mode))
        .await;

    assert!(matches!(
        *app_state.watering_system.active_mode.read().await,
        ModeEnum::Manual(_)
    ));
}

#[tokio::test]
async fn test_all_mode_transitions() {
    let app_state = set_app_state().await;

    // Initially in Auto mode
    assert!(matches!(
        *app_state.watering_system.active_mode.read().await,
        ModeEnum::Auto(_)
    ));

    // Transition from Auto -> Manual
    let manual_mode = app_state.watering_system.manual_mode.read().await.clone();
    app_state
        .watering_system
        .switch_mode(ModeEnum::Manual(manual_mode))
        .await;

    assert!(matches!(
        *app_state.watering_system.active_mode.read().await,
        ModeEnum::Manual(_)
    ));

    // Transition from Manual -> Wizard
    let wizard_mode = app_state.watering_system.wizard_mode.read().await.clone();
    app_state
        .watering_system
        .switch_mode(ModeEnum::Wizard(wizard_mode))
        .await;

    assert!(matches!(
        *app_state.watering_system.active_mode.read().await,
        ModeEnum::Wizard(_)
    ));

    // Transition from Wizard -> Auto
    let auto_mode = app_state.watering_system.auto_mode.read().await.clone();
    app_state
        .watering_system
        .switch_mode(ModeEnum::Auto(auto_mode))
        .await;

    assert!(matches!(
        *app_state.watering_system.active_mode.read().await,
        ModeEnum::Auto(_)
    ));

    // Additional transitions to verify no unexpected behavior:
    // Auto -> Wizard
    app_state
        .watering_system
        .switch_mode(ModeEnum::Wizard(
            app_state.watering_system.wizard_mode.read().await.clone(),
        ))
        .await;

    assert!(matches!(
        *app_state.watering_system.active_mode.read().await,
        ModeEnum::Wizard(_)
    ));

    // Wizard -> Manual
    app_state
        .watering_system
        .switch_mode(ModeEnum::Manual(
            app_state.watering_system.manual_mode.read().await.clone(),
        ))
        .await;

    assert!(matches!(
        *app_state.watering_system.active_mode.read().await,
        ModeEnum::Manual(_)
    ));

    // Manual -> Auto
    app_state
        .watering_system
        .switch_mode(ModeEnum::Auto(
            app_state.watering_system.auto_mode.read().await.clone(),
        ))
        .await;

    assert!(matches!(
        *app_state.watering_system.active_mode.read().await,
        ModeEnum::Auto(_)
    ));
}
