# CLAUDE.md

## GitHub Token

PAT stocké dans `.github_pat` (jamais commité — dans `.gitignore`).

```bash
source .github_pat  # expose $GITHUB_PAT
```

## Commit conventions

**Toujours** inclure le numéro d'issue GitHub dans les commits :

```
feat(auth): implement sso_login flow closes #3
fix(auth): support SSO v2 format closes #1
```

Format : `<type>(<scope>): <description> closes #N`

Après merge d'un milestone : fermer les issues correspondantes via l'API GitHub MCP (`mcp__github__issue_write` avec `state: closed`).

## Issues par milestone

| Milestone | Issues |
|-----------|--------|
| v0.1 — Auth & SSO | #1 #2 #3 #4 (fermées) |
| v0.2 — S3 | #5 #6 |
| v0.3 — Lambda & CloudWatch | #7 #8 |
| v0.4 — ECS / ECR | #9 #10 |
| v0.5 — Packaging | #11 #12 |

## Project

Zed editor MCP server extension for AWS.
Spec : `docs/superpowers/specs/2026-05-18-zed-aws-plugin-design.md`
Repo GitHub : https://github.com/clementGilardy/zed-aws-toolkit

## Architecture

- `extension/` — WASM crate (thin launcher, `context_server_command`)
- `sidecar/` — native Rust binary (MCP JSON-RPC over stdio, AWS SDK)
- Auth : AWS IAM Identity Center SSO v2 (`sso_session` format), multi-account
- Cargo binary : `~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo`
