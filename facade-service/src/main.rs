use std::sync::{Arc, Mutex};
use std::time::Duration;

use rand::prelude::SliceRandom;
use rand::{rngs::StdRng, SeedableRng};
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rocket::serde::{Deserialize, Serialize};
use rocket::{get, launch, post, routes, serde::json::Json, Responder, State};
use tokio::time::sleep;

#[derive(Debug, Deserialize)]
struct Message {
    msg: String,
}

#[derive(Debug, Serialize, Clone)]
struct LoggedMessage {
    uuid: String,
    msg: String,
}

struct AppState {
    rng: Arc<Mutex<StdRng>>,
    logging_services: Vec<String>,
    message_services: Vec<String>,
    producer: FutureProducer,
}

impl AppState {
    pub fn select_logging_service(&self) -> String {
        let mut rng = self.rng.lock().unwrap();
        self.logging_services.choose(&mut *rng).unwrap().to_string()
    }
    pub fn select_message_service(&self) -> String {
        let mut rng = self.rng.lock().unwrap();
        self.message_services.choose(&mut *rng).unwrap().to_string()
    }
    pub async fn send_to_kafka(&self, msg: &str) {
        let record = FutureRecord::to("messages").key("").payload(msg);
        let produce_future = self.producer.send(record, Duration::from_secs(0));
        produce_future.await.map_err(|(e, m)| e).unwrap();
    }
}

#[post("/", format = "json", data = "<message>")]
async fn index_post(message: Json<Message>, state: &State<AppState>) -> String {
    let selected_logging_service = state.select_logging_service();

    let client = reqwest::Client::new();
    let uuid = uuid::Uuid::new_v4();

    let logged_message = LoggedMessage {
        uuid: uuid.to_string(),
        msg: message.msg.clone(),
    };

    let url = format!("http://{}/log", selected_logging_service);
    let log_future = client
        .post(&url)
        .json(&logged_message.clone())
        .send()
        .await
        .unwrap();
    state.send_to_kafka(&serde_json::ser::to_string(&logged_message.clone()).unwrap())
        .await;

    logged_message.uuid
}

#[get("/logs")]
async fn index_get(state: &State<AppState>) -> String {
    let selected_service = state.select_logging_service();

    let client = reqwest::Client::new();
    let logging_service_response = client
        .get(format!("http://{}/logs", selected_service))
        .send()
        .await;

    logging_service_response.unwrap().text().await.unwrap()
}

#[get("/messages")]
async fn index_get_messages(state: &State<AppState>) -> String {
    let selected_service = state.select_message_service();

    let client = reqwest::Client::new();
    let logging_service_response = client
        .get(format!("http://{}/messages", selected_service))
        .send()
        .await;

    logging_service_response.unwrap().text().await.unwrap()
}

async fn create_kafka_producer(retries: usize, delay: Duration) -> FutureProducer {
    println!("Creating kafka producer...");
    sleep(Duration::from_secs(20)).await;
    let mut attempt = 0;
    loop {
        match ClientConfig::new()
            .set("bootstrap.servers", "kafka:9092")
            .create::<FutureProducer>()
        {
            Ok(producer) => return producer,
            Err(e) if attempt < retries => {
                println!("Failed to create Kafka producer: {}, retrying...", e);
                sleep(delay).await;
                attempt += 1;
            },
            Err(e) => panic!("Failed to create Kafka producer after {} attempts: {}", retries, e),
        }
    }
}

#[launch]
fn rocket() -> _ {
    let rng = Arc::new(Mutex::new(StdRng::from_entropy()));
    let logging_services = vec![
        "logging-service-1:8001".to_string(),
        "logging-service-2:8001".to_string(),
        "logging-service-3:8001".to_string(),
    ];
    let message_services = vec![
        "messages-service-1:8002".to_string(),
        "messages-service-2:8002".to_string(),
    ];

    let producer = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(create_kafka_producer(5, Duration::from_secs(5)));

    rocket::build()
        .manage(AppState {
            rng,
            logging_services,
            message_services,
            producer,
        })
        .configure(rocket::Config::figment().merge(("port", 8000)))
        .mount("/", routes![index_post, index_get, index_get_messages])
}