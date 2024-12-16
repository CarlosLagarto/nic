use nic::watering::ds::{CtrlSignal, WeatherSignal};
use tokio::sync::broadcast::{self};

#[tokio::test]
async fn test_rapid_signals() {
    let (tx, mut rx) = broadcast::channel(100);

    // Simulate rapid signals
    tokio::spawn(async move {
        for _ in 0..100 {
            tx.send(CtrlSignal::Weather(WeatherSignal::RainStart))
                .unwrap();
        }
    });

    let mut received_count = 0;
    while let Ok(signal) = rx.recv().await {
        match signal {
            CtrlSignal::Weather(WeatherSignal::RainStart) => {
                received_count += 1;
            }
            _ => {}
        }
    }

    assert_eq!(received_count, 100);
}
