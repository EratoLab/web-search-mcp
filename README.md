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

### Common configuration

- `[server]` metadata used by the MCP server (name/version).
- `[tool]`:
  - `type = "search"`
  - `tool_name` sets the MCP tool name (default: `web_search`)
  - `default_provider` selects the runtime provider (`azure_ai_search_web` or `perplexity_search`)
- Tool blocks:
  - `[tool.azure_ai_search_web]` for Azure AI Search
  - `[tool.perplexity_search]` for Perplexity

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

Run the server:

```bash
# stdio transport (default)
cargo run --bin mcp-server --features mcp-server -- -c ./mcp-server.toml --transport stdio

# SSE transport
cargo run --bin mcp-server --features mcp-server -- --transport sse --host 0.0.0.0 --port 3000

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
  -t, --transport <TYPE>      Transport type: stdio, sse, http
  -p, --port <PORT>          Port for SSE or HTTP transport
      --host <HOST>          Host address to bind to
      --sse-path <PATH>      SSE endpoint path
      --sse-post-path <PATH> SSE POST path for messages
      --http-path <PATH>     HTTP streamable endpoint path
  -h, --help                 Print help
  -V, --version              Print version
```

## Requirements

- Rust 1.70+
- Azure AI Search account with a configured knowledge base, or a Perplexity API key

## Azure AI Search Setup Guide

- Create new `AI Search` Resource in Azure
- Create a new "Knowledge source" of type "Web" (Bing)
- Create a new "Knowledge base"
  - Select the knowledge source you created in the previous step
  - Configure model and reasoning effort

## License

MIT
