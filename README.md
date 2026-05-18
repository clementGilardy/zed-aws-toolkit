# zed-aws-toolkit

> AWS Toolkit extension for [Zed editor](https://zed.dev) — inspired by the [AWS Toolkit for JetBrains](https://plugins.jetbrains.com/plugin/11349-aws-toolkit).

Expose AWS services as MCP tools directly in the Zed Agent Panel. Authenticate once via AWS IAM Identity Center (Azure AD federated), switch between accounts, and interact with S3, Lambda, CloudWatch Logs, and ECS/ECR without leaving your editor.

---

## Features

- **SSO Authentication** — AWS IAM Identity Center with Azure AD federation. Browser-based login, automatic token refresh, multi-account support.
- **S3** — Browse buckets and objects, upload/download files, generate presigned URLs.
- **Lambda** — List functions, invoke with custom payloads, stream logs.
- **CloudWatch Logs** — Tail log streams, search across log groups.
- **ECS / ECR** — Inspect clusters, services, running tasks, and container images.

All features are exposed as **MCP tools** in the Zed Agent Panel — usable via natural language or direct tool calls.

---

## Architecture

```
Zed Agent Panel
  └─ MCP protocol (JSON-RPC over stdio)
       └─ sidecar (native Rust binary)
            ├─ auth/       IAM Identity Center SSO, token cache, multi-account
            └─ services/   S3 · Lambda · CloudWatch · ECS · ECR
```

The Zed extension (WASM) is a thin wrapper that launches the sidecar. All AWS SDK logic lives in the native sidecar, avoiding WASM sandbox limitations.

---

## Requirements

- [Zed](https://zed.dev) v0.180+
- AWS account with IAM Identity Center configured
- Azure AD federated with IAM Identity Center (or any supported IdP)
- `~/.aws/config` with SSO profiles configured

---

## Installation

> The extension is not yet published. See [Contributing](#contributing) to run it locally.

Once published:

1. Open Zed → `zed: extensions`
2. Search for `aws-toolkit`
3. Install

---

## Usage

### Login

```
Use the sso_login tool with your AWS profile name to authenticate.
```

### Switch account

```
Use switch_account to change the active AWS account/role.
```

### Example interactions

```
List all S3 buckets in my account
Invoke the process-orders Lambda with payload {"env": "prod"}
Tail the /aws/lambda/process-orders log stream
List running ECS tasks in the production cluster
```

---

## Roadmap

| Milestone | Status |
|-----------|--------|
| v0.1 — Auth & SSO | 🚧 In progress |
| v0.2 — S3 | ⏳ Planned |
| v0.3 — Lambda & CloudWatch | ⏳ Planned |
| v0.4 — ECS / ECR | ⏳ Planned |
| v0.5 — Zed extension packaging | ⏳ Planned |

---

## Contributing

Contributions are welcome — bug reports, feature requests, and pull requests.

### Getting started

1. **Fork** the repository and clone your fork
2. Install [Rust](https://rustup.rs) (stable toolchain)
3. Add the WASM target: `rustup target add wasm32-wasip1`
4. Build the sidecar: `cargo build -p sidecar`
5. Build the extension: `cargo build -p zed-aws-toolkit --target wasm32-wasip1`
6. Install as a dev extension in Zed: open `zed: install dev extension` and point to the repo root

### Project structure

```
zed-aws-toolkit/
├── extension.toml       # Zed extension manifest
├── Cargo.toml           # Workspace root
├── src/                 # WASM extension crate (thin launcher)
└── sidecar/             # Native MCP server
    └── src/
        ├── main.rs
        ├── mcp/         # JSON-RPC protocol
        ├── auth/        # SSO, token cache, multi-account
        └── services/    # s3, lambda, cloudwatch, ecs
```

### Guidelines

- **One concern per PR** — keep changes focused and reviewable
- **Tests for the sidecar** — unit tests per service module; integration tests with localstack for S3
- **No breaking changes to MCP tool signatures** without a deprecation notice
- Open an issue before starting large changes to align on approach

### Reporting issues

Use [GitHub Issues](https://github.com/clementGilardy/zed-aws-toolkit/issues). Include:
- Zed version
- OS and architecture
- Relevant `~/.aws/config` profile structure (redact account IDs if needed)
- Steps to reproduce

---

## License

MIT
