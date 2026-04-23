default:
    just --list

check:
    cargo check
    cargo check --features mcp-server

test:
    cargo clippy
    cargo clippy --features mcp-server
