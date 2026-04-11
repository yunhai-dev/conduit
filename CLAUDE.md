# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

- `conduit` is a Rust CLI for SSH reverse port forwarding.
- `conduit connect` is the primary public entrypoint; `conduit expose` remains as a compatibility command.
- The current implementation already covers:
  - CLI parsing and config validation
  - SSH reverse tunnel establishment via `russh`
  - bidirectional forwarding between remote forwarded channels and local TCP targets
  - daemon mode on Unix
  - local registry/log-based lifecycle management for daemon-started tunnels
- Runtime flow:
  - `src/main.rs` initializes logging first, including a pre-scan of `--log-file` so daemon mode can write logs before the main app runs.
  - `src/app.rs` parses CLI, converts `ConnectArgs` or `ExposeArgs` into validated `ExposeConfig`, then dispatches either tunnel startup or lifecycle commands.
  - `src/expose/mod.rs` establishes the SSH session, requests remote forwarding, and bridges each accepted forwarded connection to the configured local TCP target.
  - `src/tunnel_registry.rs` persists daemon tunnel metadata and powers `list/status/show/close/stop/restart/delete/logs`.
- `src/expose/types.rs` is the main validation boundary between CLI input and runtime logic. It enforces:
  - exactly one of password/private-key auth inputs
  - valid `--server` and `--local` socket/server addresses
  - valid `connect --remote <SPEC>` parsing
  - a non-empty `--remote-host`
  - existing private key files
  - `--insecure-accept-host-key` for the current V1 flow
- `src/expose/` is organized by responsibility:
  - `types.rs`: validated config and auth types
  - `auth.rs`: converts config auth into runtime auth
  - `ssh_client.rs`: SSH session and remote-forward setup using `russh`
  - `forwarder.rs`: bidirectional byte forwarding between local TCP streams and SSH channel streams
  - `mod.rs`: top-level orchestration entrypoint
- `src/logging.rs` configures `tracing-subscriber` and can write either to stderr or a file.
- `src/daemon.rs` wraps the `daemonize` crate and only supports daemon mode on Unix.
- `src/tunnel_registry.rs` stores local file-backed state for daemon-managed tunnels under `~/.conduit/`.
- `src/error.rs` centralizes user-facing CLI/config/runtime errors in `ConduitError`.

## Common commands

- Build/check:
  - `cargo check`
  - `cargo build`
- Run/help:
  - `cargo run -- --help`
  - `cargo run -- connect --help`
  - `cargo run -- tunnel --help`
  - `cargo run -- connect --remote <user@server:remote_port:local_port> --password <password> --insecure-accept-host-key`
  - `cargo run -- expose --server <host:port> --user <user> --local <127.0.0.1:3000> --remote-port <port> --password <password> --insecure-accept-host-key`
- Lifecycle:
  - `cargo run -- list`
  - `cargo run -- status <id>`
  - `cargo run -- logs <id>`
  - `cargo run -- stop --all`
  - `cargo run -- restart <id> --password <password> --insecure-accept-host-key`
  - `cargo run -- delete <id> --force`
- Formatting:
  - `cargo fmt`
  - `cargo fmt -- --check`
- Lint:
  - `cargo clippy --all-targets --all-features`
- Tests:
  - `cargo test`
  - `cargo test -- --list`
  - `cargo test <test_name_substring>`

## Current command behavior

- `connect` is the preferred entrypoint for new usage; `expose` remains for compatibility.
- `tunnel` is currently an alias-oriented command group over the same local lifecycle operations.
- Lifecycle commands only manage daemon tunnels launched by this local CLI.
- `restart` requires re-supplying `--password` or `--key`, because auth material is not persisted in the local registry.
- `cargo test -- --list` currently reports 16 tests.
- `cargo clippy --all-targets --all-features` succeeds with one warning (`clippy::seek_from_current` in `src/app.rs`).

## Planning files already in use

- Complex multi-module work is tracked in `docs/IMPLEMENTATION_PLAN.md`.
- Detailed staged plans live under `docs/plan/`.
- The current active CLI planning work is tracked in `docs/plan/conduit-cli-roadmap.md`.
- The reverse tunnel runtime implementation history is tracked in `docs/plan/conduit-expose.md`.
- If you implement multi-file feature work here, update the relevant plan files as you go instead of treating them as archival docs.
