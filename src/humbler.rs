use anyhow::Result;
use indexmap::IndexMap;
use openapiv3::{
    ArrayType, Components, MediaType, ObjectType, OpenAPI, Parameter, ReferenceOr, Responses,
    Schema, SchemaKind,
};
use reqwest::Error;
use serde_json::{json, Value};
use std::{collections::HashMap, env, hash::RandomState};

#[derive(Debug)]
struct ApiInfo {
    path: String,
    method: String,
    parameters: HashMap<String, String>,
    request_body: Option<String>,
    response: Option<String>,
    swagger_url: String,
}

pub struct Humbler {
    swagger_ui_url: String,
    openapi_json_url: String,
}

impl Humbler {
    pub fn new(swagger_ui_url: String, openapi_json_url: String) -> Self {
        Self {
            swagger_ui_url,
            openapi_json_url,
        }
    }

    pub async fn run(&self) -> Result<String> {
        let api_infos = self.get_api_infos().await?;

        Ok(render_markdown_table(api_infos))
    }

    async fn get_api_infos(&self) -> Result<Vec<ApiInfo>, anyhow::Error> {
        let openapi = self.get_openapi().await?;
        let api_infos =
            openapi
                .clone()
                .paths
                .into_iter()
                .filter(|(path, reference_or_path_item)| reference_or_path_item.as_item().is_some())
                .flat_map(|(path, reference_or_path_item)| {
                    let path_item = reference_or_path_item.into_item().unwrap();

                    path_item.into_iter().map({
                        let components = openapi.components.clone().unwrap();
                        move |(method, operation)| {
                            let operation_id = operation.operation_id.unwrap();
                            let tag = operation.tags.into_iter().next().unwrap();
                            let swagger_url =
                                format!("{}/{tag}/{operation_id}", self.swagger_ui_url);
                            let parameters =
                                operation
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
                                                    openapiv3::Type::String(_) => "string",
                                                    openapiv3::Type::Number(_) => "number",
                                                    openapiv3::Type::Integer(_) => "integer",
                                                    openapiv3::Type::Boolean(_) => "boolean",
                                                    openapiv3::Type::Array(_) => "array",
                                                    openapiv3::Type::Object(_) => "object",
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
        Ok(api_infos)
    }

    async fn get_openapi(&self) -> Result<OpenAPI, anyhow::Error> {
        let json_str = match self.openapi_json_url.starts_with("http") {
            true => self.json_from_url().await?,
            false => json_from_file()?,
        };
        let openapi: OpenAPI =
            serde_json::from_str(&json_str).expect("Could not deserialize input");
        Ok(openapi)
    }

    async fn json_from_url(&self) -> Result<String, Error> {
        let response = reqwest::get(&self.openapi_json_url).await?;

        response.text().await
    }
}

fn content_to_value(
    content: IndexMap<String, MediaType, RandomState>,
    components: Components,
) -> Option<String> {
    println!("media_type: {:#?}", content);
    content.into_iter().next().map(|(_, media_type)| {
        let ref_or_schema = media_type.schema.unwrap();

        parse_schema(components, ref_or_schema).to_string()
    })
}

fn parse_schema(components: Components, ref_or_schema: ReferenceOr<Schema>) -> Value {
    let schema = match ref_or_schema {
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

    let result = match schema.schema_kind {
        SchemaKind::Type(_type) => match _type {
            openapiv3::Type::String(_) => json!("string"),
            openapiv3::Type::Number(_) => json!("number"),
            openapiv3::Type::Integer(_) => json!("integer"),
            openapiv3::Type::Boolean(_) => json!("boolean"),
            openapiv3::Type::Array(ArrayType { items, .. }) => {
                let items = items.unwrap();
                let schema_type = parse_schema(components.clone(), items.unbox());

                json!([schema_type])
            }
            openapiv3::Type::Object(ObjectType { properties, .. }) => {
                let map = properties
                    .into_iter()
                    .map(|(s, ref_or_schema)| {
                        (s, parse_schema(components.clone(), ref_or_schema.unbox()))
                    })
                    .collect::<serde_json::Map<String, serde_json::Value>>();

                // >>>{"category":"{\"id\":\"integer\",\"name\":\"string\"}","id":"integer","name":"string","photoUrls":"[string]","status":"string","tags":"[{\"id\":\"integer\",\"name\":\"string\"}]"}
                serde_json::Value::Object(map)
            }
        },
        _ => todo!(),
    };

    result
}

fn render_markdown_table(api_infos: Vec<ApiInfo>) -> String {
    let mut markdown = String::new();
    markdown.push_str("| Path | Method | Parameters | Request Body | Response | Swagger URL |\n");
    markdown.push_str("| ---- | ------ | ---------- | ------------ | -------- | ----------- |\n");
    for api_info in api_infos {
        let path = api_info.path;
        let method = api_info.method;
        let mut parameters = api_info
            .parameters
            .into_iter()
            .map(|(name, schema_type)| format!("{}: {}", name, schema_type))
            .collect::<Vec<String>>();

        parameters.sort();

        let parameters = parameters.join(", ");
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

fn json_from_file() -> Result<String> {
    // let file = std::fs::File::open("data/api-docs.json")?;
    let file = std::fs::File::open("data/pet.json")?;
    let reader = std::io::BufReader::new(file);
    let json: Value = serde_json::from_reader(reader)?;

    Ok(json.to_string())
}

mod tests {
    use super::*;
    use dotenv::dotenv;
    use openapiv3::{ArrayType, Schema, SchemaData, SchemaKind, StringType, Type};

    #[tokio::test]
    async fn content_to_value_test() {
        dotenv().ok();
        let swagger_ui_url = &env::var("SWAGGER_UI_URL").expect("SWAGGER_UI_URL must be set");
        let openapi_json_url = &env::var("OPENAPI_JSON_URL").expect("OPENAPI_JSON_URL must be set");

        let humbler = Humbler::new(swagger_ui_url.to_string(), openapi_json_url.to_string());
        let openapi = humbler.get_openapi().await.unwrap();
        let components = openapi.components.clone().unwrap();

        let content = {
            let mut content = IndexMap::new();
            let schema = Schema {
                schema_kind: SchemaKind::Type(Type::Array(ArrayType {
                    items: Some(ReferenceOr::Reference {
                        reference: "#/components/schemas/User".to_owned(),
                    }),
                    min_items: None,
                    max_items: None,
                    unique_items: false,
                })),
                schema_data: SchemaData {
                    nullable: false,
                    read_only: false,
                    write_only: false,
                    deprecated: false,
                    external_docs: None,
                    example: None,
                    title: None,
                    description: None,
                    discriminator: None,
                    default: None,
                    extensions: Default::default(),
                },
            };
            let media_type = MediaType {
                schema: Some(ReferenceOr::Item(schema)),
                example: None,
                examples: Default::default(),
                encoding: Default::default(),
                extensions: Default::default(),
            };
            content.insert("application/json".to_owned(), media_type);

            content
        };
        let actual = content_to_value(content, components);
        let expected = r#"[{"email":"string","firstName":"string","id":"integer","lastName":"string","password":"string","phone":"string","userStatus":"integer","username":"string"}]"#;
        assert_eq!(actual.unwrap().to_string(), expected);
    }
}
