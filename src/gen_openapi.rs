use std::fs;

use folivafy::api::ApiDoc;
use utoipa::OpenApi;

fn gen_my_openapi() -> String {
    ApiDoc::openapi().to_pretty_json().unwrap()
}

fn main() {
    let doc = gen_my_openapi();
    let _ = fs::write("./auto-openapi.yml", doc);
}
