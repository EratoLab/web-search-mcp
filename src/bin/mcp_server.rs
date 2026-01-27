//! Web Search MCP Server
//!
//! This binary provides a Model Context Protocol (MCP) server for web search.
//! It uses Azure AI Search to perform web searches that can be used by AI assistants and other MCP clients.

use web_search_mcp::mcp::BrowserServer;
use clap::{Parser, ValueEnum};
use log::{debug, info};
use rmcp::{ServiceExt, transport::stdio};
use std::io::{stdin, stdout};

#[cfg(feature = "mcp-server")]
use rmcp::transport::{
    sse_server::{SseServer, SseServerConfig},
    streamable_http_server::{StreamableHttpService, session::local::LocalSessionManager},
};

#[cfg(feature = "mcp-server")]
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Transport {
    /// Standard input/output transport (default)
    Stdio,
    /// Server-Sent Events transport
    Sse,
    /// HTTP streamable transport
    Http,
}

#[derive(Parser)]
#[command(name = "web-search-mcp")]
#[command(version)]
#[command(about = "Web Search MCP server using Azure AI Search", long_about = None)]
struct Cli {
    /// Transport type to use
    #[arg(long, short = 't', value_enum, default_value = "stdio")]
    transport: Transport,

    /// Port for SSE or HTTP transport (default: 3000)
    #[arg(long, short = 'p', default_value = "3000")]
    port: u16,

    /// Host address to bind to (default: 127.0.0.1)
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// SSE endpoint path (default: /sse)
    #[arg(long, default_value = "/sse")]
    sse_path: String,

    /// SSE POST path for messages (default: /message)
    #[arg(long, default_value = "/message")]
    sse_post_path: String,

    /// HTTP streamable endpoint path (default: /mcp)
    #[arg(long, default_value = "/mcp")]
    http_path: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Web Search MCP Server v{}", env!("CARGO_PKG_VERSION"));

    // Check that required environment variables are set
    let required_vars = [
        "AZURE_AI_SEARCH_BASE_URL",
        "AZURE_AI_SEARCH_KB_NAME",
        "AZURE_AI_SEARCH_KNOWLEDGE_SOURCE_NAME",
        "AZURE_AI_SEARCH_API_KEY",
    ];

    for var in &required_vars {
        if std::env::var(var).is_err() {
            return Err(format!("Required environment variable {} is not set", var).into());
        }
    }

    info!("Azure AI Search configuration loaded");

    // Route to appropriate transport
    match cli.transport {
        Transport::Stdio => {
            info!("Transport: stdio");
            info!("Ready to accept MCP connections via stdio");
            let (_read, _write) = (stdin(), stdout());
            let service = BrowserServer::new()
                .map_err(|e| format!("Failed to create web search server: {}", e))?;
            let server = service.serve(stdio()).await?;

            // Set up signal handler for graceful shutdown
            #[cfg(unix)]
            {
                let mut sigterm =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
                let mut sigint =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

                tokio::select! {
                    quit_reason = server.waiting() => {
                        debug!("Server quit with reason: {:?}", quit_reason);
                    }
                    _ = sigterm.recv() => {
                        info!("Received SIGTERM, shutting down gracefully...");
                    }
                    _ = sigint.recv() => {
                        info!("Received SIGINT (Ctrl+C), shutting down gracefully...");
                    }
                }
            }

            #[cfg(windows)]
            {
                let mut ctrl_c = tokio::signal::windows::ctrl_c()?;
                let mut ctrl_break = tokio::signal::windows::ctrl_break()?;

                tokio::select! {
                    quit_reason = server.waiting() => {
                        debug!("Server quit with reason: {:?}", quit_reason);
                    }
                    _ = ctrl_c.recv() => {
                        info!("Received Ctrl+C, shutting down gracefully...");
                    }
                    _ = ctrl_break.recv() => {
                        info!("Received Ctrl+Break, shutting down gracefully...");
                    }
                }
            }

            #[cfg(not(any(unix, windows)))]
            {
                let quit_reason = server.waiting().await;
                debug!("Server quit with reason: {:?}", quit_reason);
            }
        }
        Transport::Sse => {
            info!("Transport: SSE");
            info!("Host: {}", cli.host);
            info!("Port: {}", cli.port);
            info!("SSE path: {}", cli.sse_path);
            info!("SSE POST path: {}", cli.sse_post_path);

            let bind_addr = format!("{}:{}", cli.host, cli.port);

            // Create SSE server configuration
            let config = SseServerConfig {
                bind: bind_addr.parse()?,
                sse_path: cli.sse_path.clone(),
                post_path: cli.sse_post_path.clone(),
                ct: CancellationToken::new(),
                sse_keep_alive: None,
            };

            // Create SSE server and router
            let (sse_server, router) = SseServer::new(config);

            info!(
                "Ready to accept MCP connections at http://{}{}",
                bind_addr, cli.sse_path
            );

            // Register service factory for each connection
            let _cancellation_token = sse_server.with_service(move || {
                BrowserServer::new()
                    .expect("Failed to create web search server")
            });

            // Start HTTP server with SSE router
            let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
            axum::serve(listener, router.into_make_service()).await?;
        }
        Transport::Http => {
            info!("Transport: HTTP streamable");
            info!("Host: {}", cli.host);
            info!("Port: {}", cli.port);
            info!("HTTP path: {}", cli.http_path);

            let bind_addr = format!("{}:{}", cli.host, cli.port);

            // Create service factory closure
            let service_factory = move || {
                BrowserServer::new()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            };

            let http_service = StreamableHttpService::new(
                service_factory,
                LocalSessionManager::default().into(),
                Default::default(),
            );

            let router = axum::Router::new().nest_service(&cli.http_path, http_service);

            info!(
                "Ready to accept MCP connections at http://{}{}",
                bind_addr, cli.http_path
            );

            let listener = tokio::net::TcpListener::bind(bind_addr).await?;
            axum::serve(listener, router).await?;
        }
    }

    Ok(())
}
