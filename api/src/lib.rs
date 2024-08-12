use http_core::{*, util::*};



pub fn router(parts: &Parts, _: Body) -> Result {
    let path = normalize_path(parts.uri.path());

    match (&parts.method, path) {
        (GET, "/") => {
            json!({ "name": "John" }).into_response()
        }
        _ => NOT_FOUND,
    }
}





