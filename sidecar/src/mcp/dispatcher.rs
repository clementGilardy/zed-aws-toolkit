use anyhow::Result;
use serde_json::Value;
use crate::mcp::types::{McpRequest, McpResponse};

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

    pub fn dispatch(&self, req: McpRequest) -> McpResponse {
        match self.handlers.get(&req.tool) {
            Some(handler) => match handler(req.params) {
                Ok(data) => McpResponse::ok(req.id, data),
                Err(e) => McpResponse::err(req.id, format!("{:#}", e)),
            },
            None => McpResponse::err(req.id, format!("unknown tool: {}", req.tool)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_known_tool() {
        let mut d = Dispatcher::new();
        d.register("ping", Box::new(|_| Ok(serde_json::json!({"pong": true}))));
        let req = McpRequest { id: 1, tool: "ping".into(), params: serde_json::json!({}) };
        let resp = d.dispatch(req);
        assert!(resp.ok);
        assert_eq!(resp.data.unwrap()["pong"], true);
    }

    #[test]
    fn dispatch_unknown_tool() {
        let d = Dispatcher::new();
        let req = McpRequest { id: 2, tool: "nope".into(), params: serde_json::json!({}) };
        let resp = d.dispatch(req);
        assert!(!resp.ok);
        assert!(resp.error.unwrap().contains("unknown tool"));
    }
}
