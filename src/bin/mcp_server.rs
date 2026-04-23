//! Web Search MCP Server
//!
//! This binary provides a Model Context Protocol (MCP) server for web search.
//! It supports configurable providers to perform web searches that can be used by AI assistants and
//! other MCP clients.

use web_search_mcp::mcp::BrowserServer;
use clap::{Parser, ValueEnum};
use log::{debug, info};
use rmcp::{ServiceExt, transport::stdio};
use std::io::{stdin, stdout};
use std::sync::Arc;

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
#[command(about = "Web Search MCP server with configurable providers", long_about = None)]
struct Cli {
    /// Path to MCP server configuration file
    #[arg(long, short = 'c')]
    config: Option<String>,

    /// Transport type to use
    #[arg(long, short = 't', value_enum)]
    transport: Option<Transport>,

    /// Port for SSE or HTTP transport
    #[arg(long, short = 'p')]
    port: Option<u16>,

    /// Host address to bind to (default: 127.0.0.1)
    #[arg(long)]
    host: Option<String>,

    /// SSE endpoint path (default: /sse)
    #[arg(long)]
    sse_path: Option<String>,

    /// SSE POST path for messages (default: /message)
    #[arg(long)]
    sse_post_path: Option<String>,

    /// HTTP streamable endpoint path (default: /mcp)
    #[arg(long)]
    http_path: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Web Search MCP Server v{}", env!("CARGO_PKG_VERSION"));

    let config = Arc::new(web_search_mcp::config::ServerConfig::load(cli.config).await?);

    let transport_type = match cli.transport {
        Some(Transport::Stdio) => "stdio".to_string(),
        Some(Transport::Sse) => "sse".to_string(),
        Some(Transport::Http) => "http".to_string(),
        None => config.transport.transport.clone(),
    };

    let host = cli.host.unwrap_or_else(|| config.transport.host.clone());
    let port = cli.port.unwrap_or(config.transport.port);
    let sse_path = cli.sse_path.unwrap_or_else(|| config.transport.sse_path.clone());
    let sse_post_path = cli
        .sse_post_path
        .unwrap_or_else(|| config.transport.sse_post_path.clone());
    let http_path = cli
        .http_path
        .unwrap_or_else(|| config.transport.http_path.clone());

    info!("Loaded configuration for {}", config.server.name);
    web_search_mcp::config::set_global_config(config).await;

    // Route to appropriate transport
    match transport_type.as_str() {
        "stdio" => {
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
        "sse" => {
            info!("Transport: SSE");
            info!("Host: {}", host);
            info!("Port: {}", port);
            info!("SSE path: {}", sse_path);
            info!("SSE POST path: {}", sse_post_path);

            let bind_addr = format!("{}:{}", host, port);

            // Create SSE server configuration
            let config = SseServerConfig {
                bind: bind_addr.parse()?,
                sse_path: sse_path.clone(),
                post_path: sse_post_path.clone(),
                ct: CancellationToken::new(),
                sse_keep_alive: None,
            };

            // Create SSE server and router
            let (sse_server, router) = SseServer::new(config);

            info!(
                "Ready to accept MCP connections at http://{}{}",
                bind_addr, sse_path
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
        "http" => {
            info!("Transport: HTTP streamable");
            info!("Host: {}", host);
            info!("Port: {}", port);
            info!("HTTP path: {}", http_path);

            let bind_addr = format!("{}:{}", host, port);

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

            let router = axum::Router::new().nest_service(&http_path, http_service);

            info!(
                "Ready to accept MCP connections at http://{}{}",
                bind_addr, http_path
            );

            let listener = tokio::net::TcpListener::bind(bind_addr).await?;
            axum::serve(listener, router).await?;
        }
        other => {
            return Err(format!("Invalid transport type: {}", other).into());
        }
    }

    Ok(())
}
