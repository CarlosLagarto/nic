use crate::db::DatabaseTrait;
use crate::watering::ds::CtrlSignal;
use rumqttc::AsyncClient;
use rumqttc::{Event, MqttOptions, Packet};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;

pub async fn monitor_udp<D: DatabaseTrait + 'static>(
    tx: Arc<broadcast::Sender<CtrlSignal>>,
    _db: Arc<D>,
) {
    let socket = UdpSocket::bind("0.0.0.0:12345").await.unwrap();
    let mut buf = [0; 1024];

    loop {
        let (len, _addr) = socket.recv_from(&mut buf).await.unwrap();
        if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&buf[..len]) {
            // Save to DB
            // sqlx::query("INSERT INTO weather (data) VALUES (?)")
            //     .bind(data.to_string())
            //     .execute(&db_pool)
            //     .await
            //     .unwrap();

            // Notify WebSocket clients
            tx.send(CtrlSignal::GenWeather(data.to_string())).unwrap();
        }
    }
}

#[allow(clippy::single_match)]
pub async fn monitor_mqtt(tx: Arc<broadcast::Sender<CtrlSignal>>) {
    let mut mqttoptions = MqttOptions::new("client_id", "broker.hivemq.com", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut connection) = AsyncClient::new(mqttoptions, 10);
    client
        .subscribe("devices/+/state", rumqttc::QoS::AtLeastOnce)
        .await
        .unwrap();

    while let Ok(event) = connection.poll().await {
        match event {
            Event::Incoming(Packet::Publish(publish)) => {
                if let Ok(msg) = String::from_utf8(publish.payload.to_vec()) {
                    tx.send(CtrlSignal::DevicesState(msg)).unwrap();
                }
            }
            _ => {} // Handle other events if necessary
        }
    }
}
