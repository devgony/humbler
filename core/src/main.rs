use anyhow::Result;
use dotenv::dotenv;
use humbler::humbler::Humbler;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let swagger_ui_url = &env::var("SWAGGER_UI_URL").expect("SWAGGER_UI_URL must be set");
    let openapi_json_url = &env::var("OPENAPI_JSON_URL").expect("OPENAPI_JSON_URL must be set");

    let humbler = Humbler::new(swagger_ui_url.to_string(), openapi_json_url.to_string());

    humbler.run().await.map(|_| ())
}
