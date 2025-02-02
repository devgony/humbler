use dotenv::from_filename;
use humbler::humbler::Humbler;
use std::env;

#[tokio::test]
async fn render_html() {
    from_filename(".env.test").ok();
    let swagger_ui_url = &env::var("SWAGGER_UI_URL").expect("SWAGGER_UI_URL must be set");
    let openapi_json_url = &env::var("OPENAPI_JSON_URL").expect("OPENAPI_JSON_URL must be set");

    let humbler = Humbler::new(swagger_ui_url.to_string(), openapi_json_url.to_string());

    let actual = humbler.run().await.unwrap();
    // let mut file = File::create("tests/resources/output.md").expect("Unable to create file");
    // file.write_all(actual.as_bytes())
    //     .expect("Unable to write data");
    let expected = include_str!("resources/output.md");

    assert_eq!(actual, expected);
}

#[tokio::test]
async fn filter_on_test() {
    from_filename(".env.test").ok();
    let swagger_ui_url = &env::var("SWAGGER_UI_URL").expect("SWAGGER_UI_URL must be set");
    let openapi_json_url = &env::var("OPENAPI_JSON_URL").expect("OPENAPI_JSON_URL must be set");

    let humbler = Humbler::new(swagger_ui_url.to_string(), openapi_json_url.to_string())
        .filter_on()
        .unwrap();
    let actual = humbler.run().await.unwrap();
    let expected = include_str!("resources/filtered_output.md");

    assert_eq!(actual, expected);
}
