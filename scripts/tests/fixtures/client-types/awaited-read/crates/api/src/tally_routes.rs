const TALLY_PATH: &str = "/api/tally";

pub fn router() -> Router {
    Router::new().route(TALLY_PATH, get(tally))
}

async fn tally(RawQuery(query): RawQuery) -> Response {
    if query.is_some() {
        return Json(json!({ "reason": "no query is read" })).into_response();
    }
    Json(json!({ "points": 3, "streak": 2 })).into_response()
}
