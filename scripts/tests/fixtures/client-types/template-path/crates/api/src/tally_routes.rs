const TALLY_PATH: &str = "/api/tally/{id}";

pub fn router() -> Router {
    Router::new().route(TALLY_PATH, get(tally))
}

async fn tally(Path(id): Path<String>) -> Response {
    Json(json!({ "points": 3, "streak": 2 })).into_response()
}
