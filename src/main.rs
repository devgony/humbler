use anyhow::Result;
use dotenv::dotenv;
use indexmap::IndexMap;
use openapiv3::{Components, MediaType, OpenAPI, Parameter, ReferenceOr, Responses};
use reqwest::Error;
use serde_json::Value;
use std::{collections::HashMap, env, hash::RandomState};

#[derive(Debug)]
struct ApiInfo {
    path: String,
    method: String,
    parameters: HashMap<String, String>,
    request_body: Option<Value>,
    response: Option<Value>,
    swagger_url: String,
}

async fn json_from_url() -> Result<String, Error> {
    let url = env::var("OPENAPI_JSON_URL").expect("OPENAPI_JSON_URL must be set");
    let response = reqwest::get(url).await?;

    response.text().await
}

fn json_from_file() -> Result<String> {
    // let file = std::fs::File::open("data/api-docs.json")?;
    let file = std::fs::File::open("data/pet.json")?;
    let reader = std::io::BufReader::new(file);
    let json: Value = serde_json::from_reader(reader)?;

    Ok(json.to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let swagger_ui_url = &env::var("SWAGGER_UI_URL").expect("SWAGGER_UI_URL must be set");
    let json_str = json_from_url().await?;
    // let json_str = json_from_file()?;
    let openapi: OpenAPI = serde_json::from_str(&json_str).expect("Could not deserialize input");

    let api_infos = openapi
        .clone()
        .paths
        .into_iter()
        // .filter(|(path, reference_or_path_item)| path == "/hcp/api/pms/pms-projects")
        // .filter(|(path, reference_or_path_item)| path == "/hcp/api/pms/pms-hcp-prj-map/{hcpPrjCd}")
        .filter(|(path, reference_or_path_item)| reference_or_path_item.as_item().is_some())
        .flat_map(|(path, reference_or_path_item)| {
            let path_item = reference_or_path_item.into_item().unwrap();
            // let operation = path_item.into_iter().next().unwrap().1;
            //
            //
            // there can be multiple operations for a path: put, get, post, delete, etc.
            path_item.into_iter().map({
                let components = openapi.components.clone().unwrap();
                move |(method, operation)| {
                    let operation_id = operation.operation_id.unwrap();
                    let tag = operation.tags.into_iter().next().unwrap();
                    let swagger_url = format!("{swagger_ui_url}/{tag}/{operation_id}");
                    let parameters = operation
                        .parameters
                        .into_iter()
                        .filter_map(|param| {
                            let param = param.into_item().unwrap();
                            match param {
                                Parameter::Query { parameter_data, .. }
                                | Parameter::Path { parameter_data, .. } => {
                                    let name = parameter_data.name;
                                    let schema_type = match parameter_data.format {
                                        openapiv3::ParameterSchemaOrContent::Schema(schema) => {
                                            match schema.into_item().unwrap().schema_kind {
                                                openapiv3::SchemaKind::Type(_type) => match _type {
                                                    openapiv3::Type::String(_) => "String",
                                                    openapiv3::Type::Number(_) => "Number",
                                                    openapiv3::Type::Integer(_) => "Integer",
                                                    openapiv3::Type::Boolean(_) => "Boolean",
                                                    openapiv3::Type::Array(_) => "Array",
                                                    openapiv3::Type::Object(_) => "Object",
                                                },
                                                _ => todo!(),
                                            }
                                        }
                                        openapiv3::ParameterSchemaOrContent::Content(_) => todo!(),
                                    };

                                    Some((name, schema_type.to_owned()))
                                }
                                // skip header parameters for now, no todo
                                Parameter::Header { .. } => None,
                                x => {
                                    todo!()
                                }
                            }
                            // let name = param.name;
                            // let schema_type = param.schema.unwrap().schema_type;
                        })
                        .collect::<HashMap<String, String>>();
                    let request_body = operation.request_body.and_then(|request_body| {
                        let content = request_body.into_item().unwrap().content;

                        content_to_value(content, components.clone())
                    });

                    let Responses {
                        default,
                        responses,
                        extensions,
                    } = operation.responses;

                    let response = responses
                        .into_iter()
                        .map(|(status_code, response)| {
                            let content = response.into_item().unwrap().content;

                            content_to_value(content, components.clone())
                        })
                        .next()
                        .flatten();

                    ApiInfo {
                        path: path.clone(),
                        method: method.to_string(),
                        parameters,
                        request_body,
                        response, // if response has only Description:OK, then it is None for now
                        swagger_url,
                    }
                    // let request_body = operation.request_body.unwrap().into_item().unwrap().content;
                }
            })
        })
        .collect::<Vec<ApiInfo>>();

    let markdown = render_markdown_table(api_infos);
    println!("{:#?}", markdown);

    Ok(())
}

fn content_to_value(
    content: IndexMap<String, MediaType, RandomState>,
    components: Components,
) -> Option<Value> {
    content.into_iter().next().map(|(_, media_type)| {
        let schema = match media_type.schema.unwrap() {
            ReferenceOr::Reference { reference } => {
                let key = reference.split("/").last().unwrap();
                components
                    .schemas
                    .iter()
                    .find(|(k, _)| k == &key)
                    .unwrap()
                    .1
                    .clone()
                    .into_item()
                    .unwrap()
            }
            ReferenceOr::Item(schema) => schema,
        };
        let schema_json: Value = serde_json::to_value(&schema).unwrap();

        schema_json
    })
}

fn render_markdown_table(api_infos: Vec<ApiInfo>) -> String {
    let mut markdown = String::new();
    markdown.push_str("| Path | Method | Parameters | Request Body | Response | Swagger URL |\n");
    markdown.push_str("| ---- | ------ | ---------- | ------------ | -------- | ----------- |\n");
    for api_info in api_infos {
        let path = api_info.path;
        let method = api_info.method;
        let parameters = api_info
            .parameters
            .into_iter()
            .map(|(name, schema_type)| format!("{}: {}", name, schema_type))
            .collect::<Vec<String>>()
            .join(", ");
        let request_body = api_info
            .request_body
            .map(|request_body| request_body.to_string())
            .unwrap_or_default();
        let response = api_info
            .response
            .map(|response| response.to_string())
            .unwrap_or_default();
        let swagger_url = api_info.swagger_url;
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            path, method, parameters, request_body, response, swagger_url
        ));
    }
    markdown
}
