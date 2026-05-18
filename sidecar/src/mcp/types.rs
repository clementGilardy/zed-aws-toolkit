use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct McpRequest {
    pub id: u64,
    pub tool: String,
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct McpResponse {
    pub id: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl McpResponse {
    pub fn ok(id: u64, data: Value) -> Self {
        Self { id, ok: true, data: Some(data), error: None }
    }

    pub fn err(id: u64, error: impl Into<String>) -> Self {
        Self { id, ok: false, data: None, error: Some(error.into()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_request() {
        let json = r#"{"id":1,"tool":"list_accounts","params":{}}"#;
        let req: McpRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.id, 1);
        assert_eq!(req.tool, "list_accounts");
    }

    #[test]
    fn serialize_ok_response() {
        let resp = McpResponse::ok(1, serde_json::json!({"accounts": []}));
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains("\"ok\":true"));
        assert!(s.contains("\"accounts\""));
    }

    #[test]
    fn serialize_err_response() {
        let resp = McpResponse::err(2, "not authenticated");
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains("\"ok\":false"));
        assert!(s.contains("not authenticated"));
    }
}
