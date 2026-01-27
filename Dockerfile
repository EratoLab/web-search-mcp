# cargo-chef version = 0.1.73
# Rust version = 1.91.1
# Bookworm = Debian 12 -> LTS support until 2028
# Pinned for security & build performance reasons
FROM lukemathwalker/cargo-chef:0.1.73-rust-1.91.1-bookworm@sha256:ac22e8377f914b774d0d3a27cb60ab93b9115d39ef3f72e01f2dfbb55433b3eb AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --features mcp-server --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --features mcp-server --bin mcp-server

# Runtime stage - minimal image for web search
# Pinned for security & build performance reasons
FROM debian:bookworm-slim@sha256:e899040a73d36e2b36fa33216943539d9957cba8172b858097c2cabcdb20a3e2

WORKDIR /app

# Install only ca-certificates for HTTPS
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/mcp-server /app/mcp-server

# Create a non-root user for running the application
RUN useradd -m -u 1000 appuser && \
    chown -R appuser:appuser /app

USER appuser

# Environment variables for Azure AI Search (to be set at runtime)
# AZURE_AI_SEARCH_BASE_URL
# AZURE_AI_SEARCH_KB_NAME
# AZURE_AI_SEARCH_KNOWLEDGE_SOURCE_NAME
# AZURE_AI_SEARCH_API_KEY

EXPOSE 3000

CMD ["./mcp-server"]
