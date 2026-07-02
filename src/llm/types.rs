use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub struct Message {
    pub role: String,
    pub parts: Vec<ContentPart>,
}

#[derive(Clone, Debug)]
pub enum ContentPart {
    Text(String),
    FunctionCall { name: String, args: Value },
    FunctionResponse { name: String, response: Value },
}

impl Message {
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            parts: vec![ContentPart::Text(text.into())],
        }
    }

    pub fn model_text(text: impl Into<String>) -> Self {
        Self {
            role: "model".to_string(),
            parts: vec![ContentPart::Text(text.into())],
        }
    }

    pub fn model_function_call(name: String, args: Value) -> Self {
        Self {
            role: "model".to_string(),
            parts: vec![ContentPart::FunctionCall { name, args }],
        }
    }

    pub fn tool_result(name: impl Into<String>, result: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            parts: vec![ContentPart::FunctionResponse {
                name: name.into(),
                response: json!({ "result": result.into() }),
            }],
        }
    }
}

#[derive(Serialize)]
pub struct GeminiRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<SystemInstruction>,

    pub contents: Vec<GeminiMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

#[derive(Serialize)]
pub struct SystemInstruction {
    pub parts: Vec<RequestPart>,
}

#[derive(Serialize)]
pub struct GeminiMessage {
    pub role: String,
    pub parts: Vec<RequestPart>,
}

#[derive(Serialize)]
pub struct RequestPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(rename = "functionCall", skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,

    #[serde(rename = "functionResponse", skip_serializing_if = "Option::is_none")]
    pub function_response: Option<FunctionResponseBody>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FunctionCall {
    pub name: String,
    pub args: Value,
}

#[derive(Serialize)]
pub struct FunctionResponseBody {
    pub name: String,
    pub response: Value,
}

#[derive(Serialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Serialize)]
pub struct Tool {
    pub function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Deserialize, Debug)]
pub struct GeminiResponse {
    pub candidates: Vec<Candidate>,
}

#[derive(Deserialize, Debug)]
pub struct Candidate {
    pub content: ResponseContent,
}

#[derive(Deserialize, Debug)]
pub struct ResponseContent {
    pub parts: Vec<ResponsePart>,
    #[allow(dead_code)]
    pub role: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ResponsePart {
    pub text: Option<String>,

    #[serde(rename = "functionCall")]
    pub function_call: Option<FunctionCall>,
}

pub fn message_to_gemini(message: &Message) -> GeminiMessage {
    let role = if message.role == "assistant" {
        "model".to_string()
    } else {
        message.role.clone()
    };

    let parts = message
        .parts
        .iter()
        .map(|part| match part {
            ContentPart::Text(text) => RequestPart {
                text: Some(text.clone()),
                function_call: None,
                function_response: None,
            },
            ContentPart::FunctionCall { name, args } => RequestPart {
                text: None,
                function_call: Some(FunctionCall {
                    name: name.clone(),
                    args: args.clone(),
                }),
                function_response: None,
            },
            ContentPart::FunctionResponse { name, response } => RequestPart {
                text: None,
                function_call: None,
                function_response: Some(FunctionResponseBody {
                    name: name.clone(),
                    response: response.clone(),
                }),
            },
        })
        .collect();

    GeminiMessage { role, parts }
}
