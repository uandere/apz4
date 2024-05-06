use std::env;

use log::info;
use redis::AsyncCommands;
use redis::cluster::ClusterClient;
use rocket::{get, http::Status, launch, post, response::status, routes, serde::json::Json};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct LoggedMessage {
    uuid: String,
    msg: String,
}

#[post("/log", format = "json", data = "<message>")]
async fn log_message(message: Json<LoggedMessage>) -> Result<Json<LoggedMessage>, status::Custom<String>> {
    let cluster_nodes: Vec<String> = vec![
        env::var("REDIS_URL0").unwrap(),
        env::var("REDIS_URL1").unwrap(),
        env::var("REDIS_URL2").unwrap(),
    ];

    let client = ClusterClient::new(cluster_nodes).map_err(|_| status::Custom(Status::InternalServerError, "Failed to connect to Redis Cluster".into()))?;
    let mut con = client.get_async_connection().await.map_err(|_| status::Custom(Status::InternalServerError, "Failed to connect to Redis Cluster".into()))?;

    let logged_message = message.into_inner();
    info!("Logging message: UUID: {}, Msg: {}", logged_message.uuid, logged_message.msg);

    con.set(&logged_message.uuid, &logged_message.msg).await.map_err(|_| status::Custom(Status::InternalServerError, "Failed to log message".into()))?;

    Ok(Json(logged_message))
}

#[get("/logs")]
async fn get_logs() -> Result<Json<Vec<LoggedMessage>>, status::Custom<String>> {
    let cluster_nodes: Vec<String> = vec![
        env::var("REDIS_URL0").unwrap(),
        env::var("REDIS_URL1").unwrap(),
        env::var("REDIS_URL2").unwrap(),
    ];

    let client = ClusterClient::new(cluster_nodes).map_err(|_| status::Custom(Status::InternalServerError, "Failed to connect to Redis Cluster".into()))?;
    let mut con = client.get_async_connection().await.map_err(|_| status::Custom(Status::InternalServerError, "Failed to connect to Redis Cluster".into()))?;

    let keys: Vec<String> = con.keys("*").await.unwrap_or_else(|_| vec![]);
    let mut messages: Vec<LoggedMessage> = Vec::new();
    for key in keys.iter() {
        if let Ok(msg) = con.get::<_, String>(key).await {
            messages.push(LoggedMessage { uuid: key.clone(), msg });
        }
    }

    Ok(Json(messages))
}

#[launch]
fn rocket() -> _ {
    env_logger::init();
    let port: u16 = env::var("ROCKET_PORT").unwrap_or_else(|_| "8001".to_string()).parse().unwrap_or(8001);
    rocket::build().configure(rocket::Config::figment().merge(("port", port))).mount("/", routes![log_message, get_logs])
}
