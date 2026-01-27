# web-search-mcp

A Model Context Protocol (MCP) server for web search using Azure AI Search.

## ✨ Highlights

- **Azure AI Search Integration** - Leverages Azure AI Search knowledge bases for web search
- **Lightweight** - Minimal dependencies, no browser automation overhead
- **MCP Compatible** - Standard MCP server implementation for AI assistants
- **Simple Configuration** - Environment variable-based configuration

## Installation

```bash
cargo add web-search-mcp
```

## Quick Start

### Running the MCP Server

Set up the required environment variables:

```bash
export AZURE_AI_SEARCH_BASE_URL="https://your-search.search.windows.net"
export AZURE_AI_SEARCH_KB_NAME="your-kb-name"
export AZURE_AI_SEARCH_KNOWLEDGE_SOURCE_NAME="web-search"
export AZURE_AI_SEARCH_API_KEY="your-api-key"
```

Run the server:

```bash
# stdio transport (default)
cargo run --bin mcp-server --features mcp-server

# SSE transport
cargo run --bin mcp-server --features mcp-server -- --transport sse --host 0.0.0.0 --port 3000

# HTTP transport
cargo run --bin mcp-server --features mcp-server -- --transport http --host 0.0.0.0 --port 3000
```

## Configuration

The server requires the following environment variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `AZURE_AI_SEARCH_BASE_URL` | Azure AI Search base URL | `https://web-search-mcp-test.search.windows.net` |
| `AZURE_AI_SEARCH_KB_NAME` | Knowledge base name | `web-search-kb` |
| `AZURE_AI_SEARCH_KNOWLEDGE_SOURCE_NAME` | Knowledge source name | `web-search` |
| `AZURE_AI_SEARCH_API_KEY` | API key for authentication | `your-api-key` |

## MCP Tool

The server exposes a single MCP tool:

### `web_search`

Performs a web search using Azure AI Search.

**Parameters:**
- `query` (string): The search query text

**Returns:**
- `response` (string): The synthesized response text
- `references` (array): Array of reference objects with:
  - `type`: Reference type (e.g., "web")
  - `id`: Reference ID
  - `url`: Source URL
  - `title`: Page title
  - `activitySource`: Activity source indicator
  - `sourceData`: Additional source data (optional)

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
docker run -p 3000:3000 \
  -e AZURE_AI_SEARCH_BASE_URL="https://your-search.search.windows.net" \
  -e AZURE_AI_SEARCH_KB_NAME="your-kb-name" \
  -e AZURE_AI_SEARCH_KNOWLEDGE_SOURCE_NAME="web-search" \
  -e AZURE_AI_SEARCH_API_KEY="your-api-key" \
  web-search-mcp
```

## CLI Options

```bash
Options:
  -t, --transport <TYPE>      Transport type: stdio, sse, http [default: stdio]
  -p, --port <PORT>          Port for SSE or HTTP transport [default: 3000]
      --host <HOST>          Host address to bind to [default: 127.0.0.1]
      --sse-path <PATH>      SSE endpoint path [default: /sse]
      --sse-post-path <PATH> SSE POST path for messages [default: /message]
      --http-path <PATH>     HTTP streamable endpoint path [default: /mcp]
  -h, --help                 Print help
  -V, --version              Print version
```

## Requirements

- Rust 1.70+
- Azure AI Search account with a configured knowledge base

## License

MIT
