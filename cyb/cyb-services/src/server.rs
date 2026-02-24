use serde::Deserialize;
use std::sync::Arc;
use warp::http::StatusCode;
use warp::Filter;

use crate::db::{run_command, DbState};

#[derive(Deserialize)]
struct RunCommandBody {
    command: String,
    immutable: bool,
}

pub async fn start_server(state: Arc<DbState>) {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["POST"])
        .allow_headers(vec!["Content-Type"]);

    let run_command_route = warp::path("run_command")
        .and(warp::post())
        .and(warp::body::json())
        .map({
            let state = state.clone();
            move |body: RunCommandBody| {
                let mut db = state.db.lock().unwrap();

                let response_message = run_command(&mut db, &body.command, body.immutable);

                match response_message {
                    Ok(result) => warp::reply::with_status(result, StatusCode::OK),
                    Err(error) => {
                        warp::reply::with_status(error, StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            }
        });

    let routes = run_command_route.with(cors);

    warp::serve(routes).run(([127, 0, 0, 1], 3031)).await;
}
