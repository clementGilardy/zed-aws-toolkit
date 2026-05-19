mod mcp;
mod auth;
mod tools;
mod services;

use auth::state::new_shared_state;
use mcp::{Dispatcher, McpRequest};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let state = new_shared_state();
    let mut dispatcher = Dispatcher::new();
    tools::auth::register(&mut dispatcher, state.clone());
    tools::s3::register(&mut dispatcher, state.clone());
    tools::lambda::register(&mut dispatcher, state.clone());
    tools::cloudwatch::register(&mut dispatcher, state);

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin).lines();
    let mut writer = tokio::io::BufWriter::new(stdout);

    while let Some(line) = reader.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<McpRequest>(&line) {
            Ok(req) => dispatcher.dispatch(req),
            Err(e) => mcp::McpResponse::err(0, format!("parse error: {e}")),
        };
        let mut out = serde_json::to_string(&response)?;
        out.push('\n');
        writer.write_all(out.as_bytes()).await?;
        writer.flush().await?;
    }
    Ok(())
}
