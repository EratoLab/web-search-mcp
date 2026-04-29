# web-search-mcp

A Model Context Protocol (MCP) server for web search using configurable providers.

## ✨ Highlights

- **Configurable Providers** - Supports both Azure AI Search web and Perplexity web search backends
- **Lightweight** - Minimal dependencies, no browser automation overhead
- **MCP Compatible** - Standard MCP server implementation for AI assistants
- **Simple Configuration** - Configure via TOML (`./mcp-server.toml` by default)

## Installation

```bash
cargo add web-search-mcp
```

## Quick Start

### Running the MCP Server

Create a `mcp-server.toml` configuration file (or pass `--config` / `MCP_SERVER_CONFIG`).

<details>
<summary>Azure AI Search example config</summary>

```toml
[server]
name = "web-search-mcp"
version = "0.1.0"

[tool]
type = "search"
tool_name = "web_search"
default_provider = "azure_ai_search_web"

[tool.azure_ai_search_web]
base_url = "https://your-search.search.windows.net"
kb_name = "your-kb-name"
knowledge_source_name = "web-search"
knowledge_source_kind = "web"
api_key = "your-azure-api-key"

[transport]
transport = "http"
host = "0.0.0.0"
port = 3000
http_path = "/mcp"
allowed_hosts = ["example.com", "example.com:3000"]
```

</details>

<details>
<summary>Perplexity example config</summary>

```toml
[server]
name = "web-search-mcp"
version = "0.1.0"

[tool]
type = "search"
tool_name = "web_search"
default_provider = "perplexity_search"

[tool.perplexity_search]
base_url = "https://api.perplexity.ai"
api_key = "your-perplexity-api-key"
```

</details>

<details>
<summary>GenAI Responses API example config</summary>

```toml
[server]
name = "web-search-mcp"
version = "0.1.0"

[tool]
type = "search"
tool_name = "web_search"
default_provider = "genai_responses"

[tool.genai_responses]
base_url = "https://{your-resource}.services.ai.azure.com/openai/deployments/{your-deployment}"
model = "gpt-4o"
api_key = "your-genai-api-key"
api_version = "2025-04-01-preview"
api_key_header = "api-key"
reasoning = "none"
system_prompt = "Use the web_search tool and return concise, source-backed results."
tool_max_uses = 3
tool_allowed_domains = ["rust-lang.org", "docs.rs", "github.com"]
```

</details>

### Common configuration

- `[server]` metadata used by the MCP server (name/version).
- `[tool]`:
  - `type = "search"`
  - `tool_name` sets the MCP tool name (default: `web_search`)
  - `default_provider` selects the runtime provider (`azure_ai_search_web`, `perplexity_search`, or `genai_responses`)
- Tool blocks:
  - `[tool.azure_ai_search_web]` for Azure AI Search
  - `[tool.perplexity_search]` for Perplexity
  - `[tool.genai_responses]` for OpenAI Responses API
- `[transport]` configures MCP transport behavior:
  - `transport`: `stdio` or `http` (defaults to `stdio`)
  - `host`: HTTP bind host (defaults to `127.0.0.1`)
  - `port`: HTTP bind port (defaults to `3000`)
  - `http_path`: streamable HTTP endpoint path (defaults to `/mcp`)
  - `allowed_hosts`: optional Host header allowlist for HTTP transport; when unset, rmcp's loopback defaults are used
  - `disable_allowed_hosts`: set to `true` to disable Host header allowlist validation

### Provider-specific configuration

#### Azure AI Search (`[tool.azure_ai_search_web]`)
- `base_url`: search service base URL (for example `https://your-search.search.windows.net`)
- `kb_name`: knowledge base name
- `knowledge_source_name`: knowledge source name (for example `web-search`)
- `knowledge_source_kind`: optional, defaults to `web`; optional provider-specific behavior values supported by your deployment
- `api_key`: API key for your Azure AI Search resource

#### Perplexity (`[tool.perplexity_search]`)
- `base_url`: optional, defaults to `https://api.perplexity.ai`
- `api_key`: API key for Perplexity
- Optional knobs forwarded as request body fields:
  - `country`
  - `max_results`
  - `max_tokens`
  - `max_tokens_per_page`
  - `search_language_filter`
  - `search_domain_filter`
  - `last_updated_after_filter`
  - `last_updated_before_filter`
  - `search_after_date_filter`
  - `search_before_date_filter`
  - `search_recency_filter`

#### GenAI Responses (`[tool.genai_responses]`)
- `base_url`: base URL for your deployment (for example `https://{your-resource}.services.ai.azure.com/openai/deployments/{your-deployment}`)
- `model`: model name (for example `gpt-4o`)
- `api_key`: API key for the configured deployment
- `api_version`: optional API version query parameter (for example `2025-04-01-preview`)
- `api_key_header`: optional header name for API key (defaults to `Authorization`; set `api-key` for Azure AI)
- `system_prompt`: optional system prompt / instructions
- `reasoning`: optional Responses reasoning effort value (defaults to `none`)
- Optional tool settings forwarded to web_search tool:
  - `tool_max_uses`
  - `tool_allowed_domains`
  - `tool_blocked_domains`

Run the server:

```bash
# stdio transport (default)
cargo run --bin mcp-server --features mcp-server -- -c ./mcp-server.toml --transport stdio

# HTTP transport
cargo run --bin mcp-server --features mcp-server -- --transport http --host 0.0.0.0 --port 3000 --http-path /mcp

# HTTP transport
cargo run --bin mcp-server --features mcp-server -- --transport http --host 0.0.0.0 --port 3000
```

## MCP Tools

The server exposes a single MCP tool:

### `web_search`

Performs a web search through the configured provider.

The active provider is resolved from configuration at startup:
- `default_provider` when multiple providers are present
- auto-selected when exactly one provider is configured

**Parameters:**
- `query` (string): The search query text

**Returns:**
- Azure AI Search: `response` (string) and `references` (array)
- Perplexity: `results` (array), `id`, and `server_time`
- GenAI Responses: `query`, `text`, `model`, and optional `response_id`

**Example:**

```json
{
  "query": "What is the capital of France?"
}
```

## Docker

Build and run with Docker:

```bash
# Build the image
docker build -t web-search-mcp .

# Run the container
docker run -p 3000:3000 -v /path/to/mcp-server.toml:/app/mcp-server.toml web-search-mcp
```

## CLI Options

```bash
Options:
  -t, --transport <TYPE>      Transport type: stdio, http
  -p, --port <PORT>          Port for HTTP transport
      --host <HOST>          Host address to bind to
      --sse-path <PATH>      (Deprecated) SSE endpoint path
      --sse-post-path <PATH> (Deprecated) SSE message post path
      --http-path <PATH>     HTTP streamable endpoint path
      --allowed-hosts <HOST> Allowed Host header values; repeat or comma-separate values
      --disable-allowed-hosts
                             Disable HTTP Host header allowlist validation
  -h, --help                 Print help
  -V, --version              Print version
```

## Debugging request timeouts and HTTP traces

- This crate now sets request timeouts to 60 seconds for MCP tool outbound calls. If you still hit
  `"MCP error -32001: Request timed out"`, inspect upstream service latency and increase the timeout
  in provider-specific call settings as needed.
- Enable verbose `reqwest` tracing with:

```bash
RUST_LOG=web_search_mcp=debug,reqwest=trace cargo run --bin mcp-server --features mcp-server -- -c ./mcp-server.toml --transport stdio
```

`reqwest` logs use the `log` crate in this project, so enable its module logging directly via `RUST_LOG`.

## Requirements

- Rust 1.70+
- Azure AI Search account with a configured knowledge base, a Perplexity API key, or a GenAI/OpenAI Responses API config

## Azure AI Search Setup Guide

- Create new `AI Search` Resource in Azure
- Create a new "Knowledge source" of type "Web" (Bing)
- Create a new "Knowledge base"
  - Select the knowledge source you created in the previous step
  - Configure model and reasoning effort

## License

MIT
