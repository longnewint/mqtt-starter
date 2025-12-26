mod shared;

use axum::{
    extract::State,
    routing::{get, post},
    Router,
    Json,
};

use paho_mqtt as mqtt;

use serde::{Deserialize, Serialize};
use serde_json;

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

use crate::shared::weather::WeatherStation;

#[derive(Clone)]
struct AppState {
    mqtt_client: Arc<Mutex<mqtt::Client>>,
    is_publishing: Arc<Mutex<bool>>,
    stations: Arc<Vec<WeatherStation>>,
    current_station: Arc<Mutex<usize>>
}

#[derive(Serialize, Deserialize)]
struct StatusResponse {
    status: String,
    publishing: bool,
}



#[tokio::main]
async fn main() {
    // Configure MQTT
    let broker_url = "tcp://localhost:1883"; // Change to your broker
    let client_id = "rust_axum_publisher";
    let topic = "random/weather";

    // Create MQTT client
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(broker_url)
        .client_id(client_id)
        .finalize();

    let client = mqtt::Client::new(create_opts).expect("Failed to create MQTT client");

    // Connect to broker
    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(std::time::Duration::from_secs(20))
        .clean_session(true)
        .finalize();

    client.connect(conn_opts).expect("Failed to connect to MQTT broker");
    println!("✓ Connected to MQTT broker at {}", broker_url);
    
    let stations = Arc::new(vec![
        WeatherStation::new(1, "Station01", 10.0),
        WeatherStation::new(3, "Station03", 20.0),
        WeatherStation::new(7, "Station07", 30.0),
    ]);

    println!(" Created 3 stations:");
    for st in stations.iter() {
        println!(" station_id: {}, station_name: {}", st.id, st.name());
    }

    // Wrap client in Arc<Mutex> for thread-safe sharing
    let state = AppState {
        mqtt_client: Arc::new(Mutex::new(client)),
        is_publishing: Arc::new(Mutex::new(true)),
        stations,
        current_station: Arc::new(Mutex::new(0))
    };
    
    println!(" Auto-publishing started (round-robin every 1 second)");

    // Start background task to publish every second
    let publish_state = state.clone();
    tokio::spawn(async move {
        publish_loop(publish_state, topic).await;
    });

    // Build Axum app
    let app = Router::new()
        .route("/start", post(start_publishing))
        .route("/stop", post(stop_publishing))
        .route("/status", get(get_status))
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");

    println!(" Server running on http://localhost:3000");
    println!(" POST /start      - Start auto-publishing every 1s");
    println!(" POST /stop       - Stop auto-publishing");
    println!(" GET  /status     - Check publishing status");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}

async fn publish_loop(state: AppState, topic: &str) {
    let mut ticker = interval(Duration::from_secs(1));

    loop {
        ticker.tick().await;

        let is_active = *state.is_publishing.lock().await;
        if !is_active {
            continue;
        }
        
        let mut current_idx = state.current_station.lock().await;
        let gen_idx = *current_idx;
        *current_idx = (gen_idx + 1) % state.stations.len();
        drop(current_idx);

        let station = &state.stations[gen_idx];
        let record = station.generate_data();

        let client = state.mqtt_client.lock().await;
        let payload = serde_json::to_string(&record).expect("Failed to serialize");

        let msg = mqtt::Message::new(topic, payload.as_bytes(), 1);

        match client.publish(msg) {
            Ok(_) => {
                println!(" Published: {} to topic '{}'", payload, topic);
            }
            Err(e) => {
                eprintln!(" Failed to publish: {:?}", e);
            }
        }
    }
}

async fn start_publishing(State(state): State<AppState>) -> Json<StatusResponse> {
    let mut is_publishing = state.is_publishing.lock().await;
    *is_publishing = true;
    println!(" Started auto-publishing");
    
    Json(StatusResponse {
        status: "Started publishing every 1 second".to_string(),
        publishing: true,
    })
}

async fn stop_publishing(State(state): State<AppState>) -> Json<StatusResponse> {
    let mut is_publishing = state.is_publishing.lock().await;
    *is_publishing = false;
    println!(" Stopped auto-publishing");
    
    Json(StatusResponse {
        status: "Stopped publishing".to_string(),
        publishing: false,
    })
}

async fn get_status(State(state): State<AppState>) -> Json<StatusResponse> {
    let is_publishing = *state.is_publishing.lock().await;
    
    Json(StatusResponse {
        status: if is_publishing { "Publishing" } else { "Stopped" }.to_string(),
        publishing: is_publishing,
    })
}
