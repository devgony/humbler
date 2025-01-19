use anyhow::Result;
use humbler::utils::ReferenceOrExt;
use indexmap::IndexMap;
use openapiv3::{Components, MediaType, OpenAPI, Parameter, PathItem, ReferenceOr, Responses};
use reqwest::Error;
use serde_json::Value;
use std::{collections::HashMap, hash::RandomState};

#[derive(Debug)]
struct ApiInfo {
    path: String,
    method: String,
    parameters: HashMap<String, String>,
    request_body: Option<Value>,
    // responses: Responses,
    response: Option<Value>,
    swagger_url: String,
}

async fn json_from_url() -> Result<String, Error> {
    let url = "http://localhost:7771/hcp/v3/api-docs";
    let response = reqwest::get(url).await?;

    response.text().await
}

fn json_from_file() -> Result<String> {
    let file = std::fs::File::open("data/api-docs.json")?;
    let reader = std::io::BufReader::new(file);
    let json: Value = serde_json::from_reader(reader)?;

    Ok(json.to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    let swagger_url_base = "http://localhost:7771/hcp/swagger-ui/index.html";
    // let json_str = json_from_url().await?;
    let json_str = json_from_file()?;

    // let json: Value = response.json().await?;
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
                    let swagger_url = format!("{swagger_url_base}/{tag}/{operation_id}");
                    let parameters = operation
                        .parameters
                        .into_iter()
                        .map(|param| {
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

                                    (name, schema_type.to_owned())
                                }
                                _ => {
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
                        .unwrap();

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

    println!("{:#?}", api_infos);

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
