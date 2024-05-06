use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rdkafka::message::Message;
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
};
use rocket::{
    fairing::AdHoc,
    get,
    routes,
    State
};
use serde::Serialize;
use tokio::time::sleep;

#[derive(Serialize)]
struct StoredMessage {
    key: String,
    value: String,
}

struct MessageStore {
    messages: Arc<Mutex<VecDeque<StoredMessage>>>,
}

async fn kafka_consumer_task(
    brokers: String,
    topic: String,
    store: Arc<Mutex<VecDeque<StoredMessage>>>,
) {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "message_service_group")
        .set("bootstrap.servers", &brokers)
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("Consumer creation failed");
    
    let mut retry_count = 0;
    let max_retries = 5;
    let retry_delay = Duration::from_secs(5);

    loop {
        match consumer.subscribe(&[&topic]) {
            Ok(_) => {
                println!("Successfully subscribed to topic: {}", topic);
                break;
            }
            Err(e) => {
                eprintln!("Failed to subscribe to topic: {}. Error: {:?}", topic, e);
                if retry_count >= max_retries {
                    eprintln!(
                        "Exceeded maximum retry attempts for subscribing to topic: {}",
                        topic
                    );
                    return;
                }
                retry_count += 1;
                tokio::time::sleep(retry_delay).await;
            }
        }
    }

    loop {
        let result = consumer.recv().await;
        match result {
            Ok(borrowed_message) => {
                let key = match borrowed_message.key_view::<str>() {
                    Some(Ok(key)) => key.to_owned(),
                    _ => "unknown".to_owned(),
                };
                let value = match borrowed_message.payload_view::<str>() {
                    Some(Ok(value)) => value.to_owned(),
                    _ => "empty".to_owned(),
                };

                let mut store = store.lock().unwrap();
                store.push_back(StoredMessage { key, value });
                if store.len() > 1000 {
                    // Limiting memory usage
                    store.pop_front();
                }
            }
            Err(e) => eprintln!("Kafka error: {}", e),
        }
    }
}

#[get("/messages")]
fn messages(state: &State<MessageStore>) -> String {
    let store = state.messages.lock().unwrap();
    serde_json::to_string(&*store).unwrap_or_else(|_| "[]".to_string())
}

#[tokio::main]
async fn main() {
    rocket::build()
        .attach(AdHoc::on_liftoff("Kafka Consumer", |rocket| {
            let store = rocket.state::<MessageStore>().unwrap().messages.clone();
            Box::pin(async move {
                tokio::spawn(async move {
                    sleep(Duration::from_secs(20)).await;
                    kafka_consumer_task("kafka:9092".to_string(), "messages".to_string(), store)
                        .await;
                });
            })
        }))
        .manage(MessageStore {
            messages: Arc::new(Mutex::new(VecDeque::new())),
        })
        .mount("/", routes![messages])
        .launch()
        .await
        .unwrap();
}
