use reqwest::Error;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
struct ApiInfo {
    path: String,
    method: String,
    input: Option<Value>,
    output: Option<Value>,
    swagger_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let url = "http://localhost:7771/hcp/v3/api-docs";
    let response = reqwest::get(url).await?;
    let json: Value = response.json().await?;

    let mut api_infos: Vec<ApiInfo> = Vec::new();

    if let Some(paths) = json.get("paths") {
        if let Some(paths_obj) = paths.as_object() {
            for (path, methods) in paths_obj {
                if !path.contains("/hcp/api/pms/pms-projects") {
                    continue;
                }

                if let Some(methods_obj) = methods.as_object() {
                    for (method, details) in methods_obj {
                        let input = details.get("parameters").cloned();
                        let output = details.get("responses").cloned();
                        let api_info = ApiInfo {
                            path: path.clone(),
                            method: method.clone(),
                            input,
                            output,
                            swagger_url: url.to_string(),
                        };
                        api_infos.push(api_info);
                    }
                }
            }
        }
    }

    // Print the collected API information
    for api_info in api_infos {
        println!("{:?}", api_info);
    }

    Ok(())
}
