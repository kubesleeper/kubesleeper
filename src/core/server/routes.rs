
use rocket::http::ContentType;
use rocket::{Request};
use rocket::{
    fs::{NamedFile},
    get,
    http::Status,
    response::Redirect,
};
use rocket::response::{Responder, Response};

use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::core::server::{KUBESLEEPER_REST_PATH_PREFIX};
use crate::core::state::notification::Notification;
use crate::core::state::notification::NotificationKind;
use crate::core::state::state::State;






pub enum AppResponse {
    Success(Redirect),
    Ignored,
    InternalError(String),
}
impl<'r> Responder<'r, 'static> for AppResponse {
    fn respond_to(self, _req: &'r Request<'_>) -> rocket::response::Result<'static> {
        match self {
            AppResponse::Success(redirect) => redirect.respond_to(_req),
            AppResponse::Ignored => {
                Response::build().status(Status::NotFound).ok()
            }
            AppResponse::InternalError(message) => {
                let html_content = format!(
                    r#"<!DOCTYPE html><html lang=\"en\">
                    <head><title>Kubesleeper</title></head>
                    <body style="padding: 2rem;">
                    <h1 style="font-family: sans-serif;">{}</h1>        
                    <code>{}</code></body></html>
                    "#,
                    "500",
                    message.replace("\n", "<br>")
                );
                let html_bytes = html_content.into_bytes();

                Response::build()
                    .status(Status::InternalServerError)
                    .header(ContentType::HTML)
                    .sized_body(html_bytes.len(), Cursor::new(html_bytes))
                    .ok()
            }
        }
    }
}



// if the user try to access an invalid file path, the
// mounting FileServer routes will not send a 404 and juste
// try to match other routes. It will cause problems
// with our 'catch all' routes mounted on '/'
// so we need a catcher to send 404 before this route
/// a catcher for static file
#[get("/static/<path..>")]
pub async fn static_catcher(path: PathBuf) -> Status {
    println!("{} Not found", path.to_string_lossy());
    Status::NotFound
}

/// send the waiting page
#[get("/wait")]
pub async fn wait() -> Option<NamedFile> {
    NamedFile::open(Path::new("static/waiting.html")).await.ok()
}

/// Get status information about kubesleeper
#[get("/")]
pub fn status() -> Status {
    Status::ServiceUnavailable
}

/// catch all route and redirect to /ks/wait
#[get("/<path..>")]
pub async fn apps(path: PathBuf) -> AppResponse {
    if path.starts_with(KUBESLEEPER_REST_PATH_PREFIX) {
        return AppResponse::Ignored;
    };

    let update = State::update_from_notification(Notification::new(NotificationKind::Activity)).await;
    match update {
        Ok(_) => AppResponse::Success(Redirect::to(format!("{KUBESLEEPER_REST_PATH_PREFIX}/wait"))),
        Err(e) => {
            AppResponse::InternalError(e.to_string())
        } 
    }
}


