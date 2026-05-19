use anyhow::Result;
use serde_json::{json, Value};

use crate::mcp::tools_manifest::all_tools;
use crate::mcp::types::{JsonRpcMessage, JsonRpcResponse};

pub type ToolHandler = Box<dyn Fn(Value) -> Result<Value> + Send + Sync>;

pub struct Dispatcher {
    handlers: std::collections::HashMap<String, ToolHandler>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self { handlers: std::collections::HashMap::new() }
    }

    pub fn register(&mut self, name: impl Into<String>, handler: ToolHandler) {
        self.handlers.insert(name.into(), handler);
    }

    /// Dispatch a JSON-RPC 2.0 message. Returns `None` for notifications (no response needed).
    pub fn dispatch(&self, msg: JsonRpcMessage) -> Option<JsonRpcResponse> {
        let method = msg.method.as_deref().unwrap_or("");

        // Notifications have no id — must not respond.
        if msg.id.is_none() {
            return None;
        }

        let id = msg.id.clone();

        match method {
            "initialize" => Some(JsonRpcResponse::result(id, json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "zed-aws-toolkit", "version": "0.5.0" }
            }))),

            "tools/list" => Some(JsonRpcResponse::result(id, json!({
                "tools": all_tools()
            }))),

            "tools/call" => {
                let params = msg.params.unwrap_or(json!({}));
                let tool_name = match params["name"].as_str() {
                    Some(n) => n.to_owned(),
                    None => return Some(JsonRpcResponse::error(id, -32602, "missing params.name")),
                };
                let arguments = params["arguments"].clone();
                let arguments = if arguments.is_null() { json!({}) } else { arguments };

                match self.handlers.get(&tool_name) {
                    Some(handler) => match handler(arguments) {
                        Ok(data) => {
                            let text = serde_json::to_string(&data).unwrap_or_else(|e| e.to_string());
                            Some(JsonRpcResponse::result(id, json!({
                                "content": [{ "type": "text", "text": text }]
                            })))
                        }
                        Err(e) => Some(JsonRpcResponse::error(id, -32000, format!("{:#}", e))),
                    },
                    None => Some(JsonRpcResponse::error(id, -32601, format!("unknown tool: {tool_name}"))),
                }
            }

            // ping / keep-alive
            "ping" => Some(JsonRpcResponse::result(id, json!({}))),

            _ => Some(JsonRpcResponse::error(id, -32601, format!("method not found: {method}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(method: &str, id: Option<i64>, params: Option<Value>) -> JsonRpcMessage {
        JsonRpcMessage {
            jsonrpc: Some("2.0".into()),
            id: id.map(|i| json!(i)),
            method: Some(method.into()),
            params,
        }
    }

    #[test]
    fn initialize_returns_protocol_version() {
        let d = Dispatcher::new();
        let resp = d.dispatch(make_msg("initialize", Some(0), Some(json!({})))).unwrap();
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert!(result["capabilities"]["tools"].is_object());
        assert_eq!(result["serverInfo"]["name"], "zed-aws-toolkit");
    }

    #[test]
    fn tools_list_returns_22_tools() {
        let d = Dispatcher::new();
        let resp = d.dispatch(make_msg("tools/list", Some(1), None)).unwrap();
        let tools = &resp.result.unwrap()["tools"];
        assert_eq!(tools.as_array().unwrap().len(), 22);
    }

    #[test]
    fn tools_call_known_tool() {
        let mut d = Dispatcher::new();
        d.register("ping_tool", Box::new(|_| Ok(json!({"pong": true}))));
        let resp = d.dispatch(make_msg(
            "tools/call",
            Some(2),
            Some(json!({"name": "ping_tool", "arguments": {}})),
        )).unwrap();
        let content = &resp.result.unwrap()["content"][0];
        assert_eq!(content["type"], "text");
        assert!(content["text"].as_str().unwrap().contains("pong"));
    }

    #[test]
    fn tools_call_unknown_tool_returns_error() {
        let d = Dispatcher::new();
        let resp = d.dispatch(make_msg(
            "tools/call",
            Some(3),
            Some(json!({"name": "nope", "arguments": {}})),
        )).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32601);
    }

    #[test]
    fn notification_returns_none() {
        let d = Dispatcher::new();
        let notif = JsonRpcMessage {
            jsonrpc: Some("2.0".into()),
            id: None,
            method: Some("notifications/initialized".into()),
            params: None,
        };
        assert!(d.dispatch(notif).is_none());
    }

    #[test]
    fn unknown_method_returns_error() {
        let d = Dispatcher::new();
        let resp = d.dispatch(make_msg("unknown/method", Some(4), None)).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32601);
    }
}
