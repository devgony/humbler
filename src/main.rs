use openapiv3::{OpenAPI, Parameter};
use reqwest::Error;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
struct ApiInfo {
    path: String,
    method: String,
    input: HashMap<String, String>,
    // parameters: Vec<ReferenceOr<Parameter>>,
    // request_body: Option<ReferenceOr<RequestBody>>,
    // responses: Responses,
    output: Value,
    swagger_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let url = "http://localhost:7771/hcp/v3/api-docs";
    let swagger_url_base = "http://localhost:7771/hcp/swagger-ui/index.html#";
    let response = reqwest::get(url).await?;
    let json_str = response.text().await?;
    // let json: Value = response.json().await?;
    let openapi: OpenAPI = serde_json::from_str(&json_str).expect("Could not deserialize input");

    // println!("{:?}", openapi);

    openapi
        .paths
        .into_iter()
        // .filter(|(path, reference_or_path_item)| path == "/hcp/api/pms/pms-projects")
        .filter(|(path, reference_or_path_item)| path == "/hcp/api/pms/pms-hcp-prj-map/{hcpPrjCd}")
        .filter(|(path, reference_or_path_item)| reference_or_path_item.as_item().is_some())
        .for_each(|(path, reference_or_path_item)| {
            let path_item = reference_or_path_item.into_item().unwrap();
            // let operation = path_item.into_iter().next().unwrap().1;
            //
            path_item.into_iter().for_each(|(method, operation)| {
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

                                (name, schema_type)
                            }
                            x => {
                                // println!("{:?}", x);
                                todo!()
                            }
                        }
                        // let name = param.name;
                        // let schema_type = param.schema.unwrap().schema_type;
                    })
                    .collect::<HashMap<String, &str>>();
                if let Some(request_body) = operation.request_body {
                    let request_body = request_body.into_item().unwrap();
                    let content = request_body.content.into_iter().next().unwrap().1;

                    // println!("{:#?}", content);
                    if let Some(schema) = content.schema {
                        println!("{:#?}", schema);
                    }

                    // let schema = content.schema.unwrap();
                    // let schema_type = match schema.into_item().unwrap().schema_kind {
                    //     openapiv3::SchemaKind::Type(_type) => match _type {
                    //         openapiv3::Type::String(_) => "String",
                    //         openapiv3::Type::Number(_) => "Number",
                    //         openapiv3::Type::Integer(_) => "Integer",
                    //         openapiv3::Type::Boolean(_) => "Boolean",
                    //         openapiv3::Type::Array(_) => "Array",
                    //         openapiv3::Type::Object(_) => "Object",
                    //     },
                    //     _ => todo!(),
                    // };
                    // println!("{:?}", schema_type);
                }
                // let request_body = operation.request_body.unwrap().into_item().unwrap().content;
            });
        });

    // let mut api_infos: Vec<ApiInfo> = Vec::new();
    // if let Some(paths) = json.get("paths") {
    //     if let Some(paths_obj) = paths.as_object() {
    //         for (path, methods) in paths_obj {
    //             if !path.contains("/hcp/api/pms/pms-projects") {
    //                 continue;
    //             }
    //
    //             if let Some(methods_obj) = methods.as_object() {
    //                 for (method, details) in methods_obj {
    //                     let input = details
    //                         .get("parameters")
    //                         .unwrap()
    //                         .as_array()
    //                         .unwrap()
    //                         .iter()
    //                         .map(|param| {
    //                             let name = param.get("name").unwrap().as_str().unwrap().to_string();
    //                             let schema_type = param
    //                                 .get("schema")
    //                                 .unwrap()
    //                                 .get("type")
    //                                 .unwrap()
    //                                 .as_str()
    //                                 .unwrap()
    //                                 .to_string();
    //
    //                             (name, schema_type)
    //                         })
    //                         .collect::<HashMap<String, String>>();
    //                     let schema = details
    //                         .get("responses")
    //                         .unwrap()
    //                         .get("200")
    //                         .unwrap()
    //                         .get("content")
    //                         .unwrap()
    //                         .as_object()
    //                         .unwrap()
    //                         .iter()
    //                         .next()
    //                         .unwrap()
    //                         .1
    //                         .get("schema")
    //                         .unwrap();
    //
    //                     let _ref = match schema.get("type").unwrap().as_str().unwrap() == "array" {
    //                         true => {
    //                             schema
    //                                 .get("items")
    //                                 .unwrap()
    //                                 .as_object()
    //                                 .unwrap()
    //                                 .into_iter()
    //                                 .next()
    //                                 .unwrap()
    //                                 .1
    //                         }
    //                         false => schema.get("$ref").unwrap(),
    //                     };
    //
    //                     let reference_id = _ref.as_str().unwrap().split("/").last().unwrap();
    //
    //                     let output_schema = json
    //                         .get("components")
    //                         .unwrap()
    //                         .get("schemas")
    //                         .unwrap()
    //                         .get(reference_id)
    //                         .unwrap();
    //
    //                     let output = output_schema.get("properties").unwrap().to_owned();
    //
    //                     let tag = details
    //                         .get("tags")
    //                         .unwrap()
    //                         .as_array()
    //                         .unwrap()
    //                         .iter()
    //                         .next()
    //                         .unwrap()
    //                         .as_str()
    //                         .unwrap();
    //                     let operation_id = details.get("operationId").unwrap().as_str().unwrap();
    //                     let swagger_url = format!("{swagger_url_base}/{tag}/{operation_id}");
    //                     let api_info = ApiInfo {
    //                         path: path.clone(),
    //                         method: method.clone(),
    //                         input,
    //                         output,
    //                         swagger_url,
    //                     };
    //                     api_infos.push(api_info);
    //                 }
    //             }
    //         }
    //     }
    // }

    // Print the collected API information
    // for api_info in api_infos {
    //     println!("{:?}", api_info);
    // }

    Ok(())
}
