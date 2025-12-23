use rocket::{fs::FileServer, routes};
use std::num::NonZeroU16;
use tracing::info;

use crate::core::{
    server::routes::{apps, static_catcher, wait},
};

mod routes;

const KUBESLEEPER_REST_PATH_PREFIX: &str = "/kubesleeper";

pub mod error {
    #[derive(Debug, thiserror::Error)]
    pub enum ServerError {
        #[error("ServerError : {0}")]
        ServerError(#[from] rocket::Error),
    }
}

pub async fn start(port: NonZeroU16) -> Result<(), error::ServerError> {
    info!("Starting server");
    let config = rocket::Config::figment().merge(("port", port));
    // .merge(("log_level", rocket::log::LogLevel::Critical));
    rocket::build()
        .configure(config)
        .mount("/", routes![apps])
        .mount(
            KUBESLEEPER_REST_PATH_PREFIX.to_string() + "/static",
            FileServer::from("static"),
        )
        .mount(KUBESLEEPER_REST_PATH_PREFIX, routes![wait, static_catcher])
        .launch()
        .await?;
    Ok(())
}
