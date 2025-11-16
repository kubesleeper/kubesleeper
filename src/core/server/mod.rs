use rocket::{fs::FileServer, routes};

use crate::core::server::routes::{apps, static_catcher, status, wait};

mod routes;

const KUBESLEEPER_REST_PATH_PREFIX: &str = "/kubesleeper";

pub mod error {
    #[derive(Debug, thiserror::Error)]
    pub enum ServerError {
        #[error("ServerError : {0}")]
        ServerError(#[from] rocket::Error),
    }
}


pub async fn start() -> Result<(), error::ServerError> {
    rocket::build()
        .mount("/", routes![apps])
        .mount(
            KUBESLEEPER_REST_PATH_PREFIX.to_string() + "/static",
            FileServer::from("static"),
        )
        .mount(
            KUBESLEEPER_REST_PATH_PREFIX,
            routes![wait, status, static_catcher],
        )
        .launch()
        .await?;
    Ok(())
}