use reqwest::Error;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
struct ApiInfo {
    path: String,
    method: String,
    input: HashMap<String, String>,
    output: Value,
    swagger_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let url = "http://localhost:7771/hcp/v3/api-docs";
    let swagger_url_base = "http://localhost:7771/hcp/swagger-ui/index.html#";
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
                        let input = details
                            .get("parameters")
                            .unwrap()
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|param| {
                                let name = param.get("name").unwrap().as_str().unwrap().to_string();
                                let schema_type = param
                                    .get("schema")
                                    .unwrap()
                                    .get("type")
                                    .unwrap()
                                    .as_str()
                                    .unwrap()
                                    .to_string();

                                (name, schema_type)
                            })
                            .collect::<HashMap<String, String>>();
                        let schema = details
                            .get("responses")
                            .unwrap()
                            .get("200")
                            .unwrap()
                            .get("content")
                            .unwrap()
                            .as_object()
                            .unwrap()
                            .iter()
                            .next()
                            .unwrap()
                            .1
                            .get("schema")
                            .unwrap();

                        let _ref = match schema.get("type").unwrap().as_str().unwrap() == "array" {
                            true => {
                                schema
                                    .get("items")
                                    .unwrap()
                                    .as_object()
                                    .unwrap()
                                    .into_iter()
                                    .next()
                                    .unwrap()
                                    .1
                            }
                            false => schema.get("$ref").unwrap(),
                        };

                        let reference_id = _ref.as_str().unwrap().split("/").last().unwrap();

                        let output_schema = json
                            .get("components")
                            .unwrap()
                            .get("schemas")
                            .unwrap()
                            .get(reference_id)
                            .unwrap();

                        let output = output_schema.get("properties").unwrap().to_owned();

                        let tag = details
                            .get("tags")
                            .unwrap()
                            .as_array()
                            .unwrap()
                            .iter()
                            .next()
                            .unwrap()
                            .as_str()
                            .unwrap();
                        let operation_id = details.get("operationId").unwrap().as_str().unwrap();
                        let swagger_url = format!("{swagger_url_base}/{tag}/{operation_id}");
                        let api_info = ApiInfo {
                            path: path.clone(),
                            method: method.clone(),
                            input,
                            output,
                            swagger_url,
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
