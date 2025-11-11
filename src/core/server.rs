use rocket::routes;
use rocket::{
    fs::{FileServer, NamedFile},
    get,
    http::Status,
    response::Redirect,
};

use std::path::{Path, PathBuf};

use crate::core::state::notification::Notification;
use crate::core::state::notification::NotificationKind;
use crate::core::state::state::State;

const KUBESLEEPER_REST_PATH_PREFIX: &str = "/kubesleeper";

// if the user try to access an invalid file path, the
// mounting FileServer routes will not send a 404 and juste
// try to match other routes. It will cause problems
// with our 'catch all' routes mounted on '/'
// so we need a catcher to send 404 before this route
/// a catcher for static file
#[get("/static/<path..>")]
async fn static_catcher(path: PathBuf) -> Status {
    println!("{} Not found", path.to_string_lossy());
    Status::NotFound
}

/// send the waiting page
#[get("/wait")]
async fn wait() -> Option<NamedFile> {
    NamedFile::open(Path::new("static/waiting.html")).await.ok()
}

/// Get status information about kubesleeper
#[get("/")]
fn status() -> Status {
    Status::ServiceUnavailable
}

/// catch all route and redirect to /ks/wait
#[get("/<path..>")]
async fn apps(path: PathBuf) -> Option<Redirect> {
    if path.starts_with(KUBESLEEPER_REST_PATH_PREFIX) {
        return None;
    };

    State::update_from_notification(Notification::new(NotificationKind::Activity))
        .await
        .unwrap();
    Some(Redirect::to(format!("{KUBESLEEPER_REST_PATH_PREFIX}/wait")))
}

pub async fn start() -> anyhow::Result<()> {
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
