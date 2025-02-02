use crate::config::load_config;
use crate::utils::option::OptionExt;
use anyhow::Result;
use indexmap::IndexMap;
use openapiv3::{
    ArrayType, Components, MediaType, ObjectType, OpenAPI, Parameter, ReferenceOr, Responses,
    Schema, SchemaKind,
};
use reqwest::Error;
use serde_json::{json, Map, Value};
use std::hash::RandomState;

#[derive(Debug)]
struct ApiInfo {
    path: String,
    method: String,
    parameters: Vec<(String, Value)>,
    request_body: Option<String>,
    response: Option<String>,
    swagger_url: String,
}

pub struct Humbler {
    swagger_ui_url: String,
    openapi_json_url: String,
    filter_keywords: Vec<String>,
}

impl Humbler {
    pub fn new(swagger_ui_url: String, openapi_json_url: String) -> Self {
        Self {
            swagger_ui_url,
            openapi_json_url,
            filter_keywords: Vec::new(),
        }
    }

    pub fn filter_on(mut self) -> Result<Self> {
        let config = load_config(".humbler.toml")?;
        self.filter_keywords = config.filter_keywords;

        Ok(self)
    }

    pub async fn run(&self) -> Result<String> {
        let api_infos = self.get_api_infos().await?;

        Ok(render_markdown_table(api_infos))
    }

    async fn get_api_infos(&self) -> Result<Vec<ApiInfo>, anyhow::Error> {
        let openapi = self.get_openapi().await?;
        openapi
            .paths
            .into_iter()
            .filter(|(path, _)| {
                self.filter_keywords
                    .iter()
                    .all(|keyword| path.contains(keyword))
            })
            .filter(|(_, reference_or_path_item)| reference_or_path_item.as_item().is_some())
            .map(|(path, reference_or_path_item)| {
                let path_item = reference_or_path_item
                    .into_item()
                    .to_result("PathItem not found")?;

                Ok(path_item.into_iter().map({
                    let components = openapi
                        .components
                        .as_ref()
                        .to_result("Components not found")?;

                    move |(method, operation)| {
                        let operation_id =
                            operation.operation_id.to_result("OperationId not found")?;
                        let tag = operation
                            .tags
                            .into_iter()
                            .next()
                            .to_result("Tag not found")?;
                        let swagger_url = format!("{}/{tag}/{operation_id}", self.swagger_ui_url);
                        let parameters = operation
                            .parameters
                            .into_iter()
                            .filter_map(|param| {
                                let param = param.into_item()?;
                                match param {
                                    Parameter::Query { parameter_data, .. }
                                    | Parameter::Path { parameter_data, .. } => {
                                        let name = parameter_data.name;
                                        let schema_type = match parameter_data.format {
                                            openapiv3::ParameterSchemaOrContent::Schema(schema) => {
                                                Parser::new().parse_schema(components, schema)
                                            }
                                            openapiv3::ParameterSchemaOrContent::Content(_) => {
                                                todo!()
                                            }
                                        };

                                        Some(schema_type.map(|schema_type| (name, schema_type)))
                                    }
                                    // skip header parameters for now, no todo
                                    Parameter::Header { .. } => None,
                                    _ => {
                                        todo!()
                                    }
                                }
                            })
                            .collect::<Result<Vec<(String, Value)>>>()?;
                        let request_body = operation
                            .request_body
                            .and_then(|request_body| {
                                let content = request_body.into_item()?.content;

                                content_to_value(content, components)
                            })
                            .transpose()?;

                        let Responses { responses, .. } = operation.responses;

                        let response = responses
                            .into_iter()
                            .map(|(_, response)| {
                                let content = response.into_item()?.content;

                                content_to_value(content, components)
                            })
                            .next()
                            .flatten()
                            .transpose()?;

                        Ok(ApiInfo {
                            path: path.clone(),
                            method: method.to_string(),
                            parameters,
                            request_body,
                            response, // if response has only Description:OK, then it is None for now
                            swagger_url,
                        })
                    }
                }))
            })
            .collect::<Result<Vec<_>>>()? // TODO: decrease collecting to once
            .into_iter()
            .flatten()
            .collect::<Result<Vec<ApiInfo>>>()
    }

    async fn get_openapi(&self) -> Result<OpenAPI, anyhow::Error> {
        let json_str = match self.openapi_json_url.starts_with("http") {
            true => self.json_from_url().await?,
            false => json_from_file(&self.openapi_json_url)?,
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
    components: &Components,
) -> Option<Result<String>> {
    content.into_iter().next().map(|(_, media_type)| {
        let ref_or_schema = media_type.schema.to_result("Schema not found")?;

        Parser::new()
            .parse_schema(components, ref_or_schema)
            .map(|v| v.to_string())
    })
}

struct Parser {
    stack: Vec<String>,
}

impl Parser {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    fn parse_schema(
        &mut self,
        components: &Components,
        ref_or_schema: ReferenceOr<Schema>,
    ) -> Result<Value> {
        let schema = match ref_or_schema {
            ReferenceOr::Reference { reference } => {
                let key = reference
                    .split('/')
                    .last()
                    .to_result(format!("Key not found in: {reference}"))?;

                if self.stack.contains(&key.to_string()) {
                    return Ok(json!(key));
                }

                let schema = components
                    .schemas
                    .iter()
                    .find(|(k, _)| k == &key)
                    .to_result(format!("key: {key} not found in components"))?
                    .1
                    .to_owned()
                    .into_item()
                    .to_result(format!("key: {key} is not a schema item"))?;

                self.stack.push(key.to_string());

                schema
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
                    let items = items.to_result("Items not found")?;
                    let schema_type = self.parse_schema(components, items.unbox())?;

                    json!([schema_type])
                }
                openapiv3::Type::Object(ObjectType { properties, .. }) => {
                    let map = properties
                        .into_iter()
                        .map(|(s, ref_or_schema)| {
                            self.parse_schema(components, ref_or_schema.unbox())
                                .map(|v| (s, v))
                        })
                        .collect::<Result<Map<String, Value>>>()?;

                    serde_json::Value::Object(map)
                }
            },
            _ => todo!(),
        };

        Ok(result)
    }
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
            .map(|(name, schema_type)| format!(r#""{}": {}"#, name, schema_type))
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

fn json_from_file(path: &str) -> Result<String> {
    // let file = std::fs::File::open("data/api-docs.json")?;
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let json: Value = serde_json::from_reader(reader)?;

    Ok(json.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::from_filename;
    use openapiv3::{ArrayType, Schema, SchemaData, SchemaKind, Type};
    use std::env;

    #[tokio::test]
    async fn content_to_value_test() {
        from_filename(".env.test").ok();
        let swagger_ui_url = &env::var("SWAGGER_UI_URL").expect("SWAGGER_UI_URL must be set");
        let openapi_json_url = "data/pet.json";

        let humbler = Humbler::new(swagger_ui_url.to_string(), openapi_json_url.to_string());
        let openapi = humbler.get_openapi().await.unwrap();

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
        let actual = content_to_value(content, &openapi.components.unwrap())
            .unwrap()
            .unwrap()
            .to_string();
        let expected = r#"[{"email":"string","firstName":"string","id":"integer","lastName":"string","password":"string","phone":"string","userStatus":"integer","username":"string"}]"#;
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn parse_recursive_schema() {
        from_filename(".env.test").ok();
        let swagger_ui_url = &env::var("SWAGGER_UI_URL").expect("SWAGGER_UI_URL must be set");
        let openapi_json_url = "data/pet.json";

        let humbler = Humbler::new(swagger_ui_url.to_string(), openapi_json_url.to_string());
        let api_infos = humbler.get_api_infos().await.unwrap();
        let post_pet = api_infos
            .into_iter()
            .find(|api_info| api_info.path == "/pet" && api_info.method == "post")
            .unwrap();

        let actual = post_pet.request_body.unwrap();
        let expected = r#"{"category":{"id":"integer","name":"string"},"children":["Pet"],"id":"integer","name":"string","photoUrls":["string"],"status":"string","tags":[{"id":"integer","name":"string"}]}"#;
        assert_eq!(actual, expected);
    }
}
