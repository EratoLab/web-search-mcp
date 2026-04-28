//! Web Search MCP Server
//!
//! This binary provides a Model Context Protocol (MCP) server for web search.
//! It supports configurable providers to perform web searches that can be used by AI assistants and
//! other MCP clients.

use clap::{Parser, ValueEnum};
use log::{debug, info};
use rmcp::{ServiceExt, transport::stdio};
use std::io::{stdin, stdout};
use std::sync::Arc;
use web_search_mcp::mcp::BrowserServer;

#[cfg(feature = "mcp-server")]
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Transport {
    /// Standard input/output transport (default)
    Stdio,
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

    /// Port for HTTP transport
    #[arg(long, short = 'p')]
    port: Option<u16>,

    /// Host address to bind to (default: 127.0.0.1)
    #[arg(long)]
    host: Option<String>,

    /// HTTP path (default: /mcp)
    #[arg(long)]
    http_path: Option<String>,
    /// SSE endpoint path (deprecated, retained for compatibility with legacy config)
    #[arg(long)]
    sse_path: Option<String>,
    /// SSE POST path (deprecated, retained for compatibility with legacy config)
    #[arg(long)]
    sse_post_path: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Web Search MCP Server v{}", env!("CARGO_PKG_VERSION"));

    let config = Arc::new(web_search_mcp::config::ServerConfig::load(cli.config).await?);

    let transport_type = match cli.transport {
        Some(Transport::Stdio) => "stdio".to_string(),
        Some(Transport::Http) => "http".to_string(),
        None => config.transport.transport.clone(),
    };

    let host = cli.host.unwrap_or_else(|| config.transport.host.clone());
    let port = cli.port.unwrap_or(config.transport.port);
    let http_path = cli
        .http_path
        .unwrap_or_else(|| config.transport.http_path.clone());

    if cli.sse_path.is_some() || cli.sse_post_path.is_some() {
        info!("sse_path/sse_post_path are deprecated and ignored; use --transport http instead");
    }

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
        "http" => {
            info!("Transport: HTTP streamable");
            info!("Host: {}", host);
            info!("Port: {}", port);
            info!("HTTP path: {}", http_path);

            let bind_addr = format!("{}:{}", host, port);

            // Create service factory closure
            let service_factory = move || {
                BrowserServer::new().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
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

            let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
            #[cfg(unix)]
            let shutdown = {
                let mut sigterm =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
                let mut sigint =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
                async move {
                    tokio::select! {
                        _ = sigterm.recv() => {}
                        _ = sigint.recv() => {}
                    }
                }
            };

            #[cfg(windows)]
            let shutdown = {
                let mut ctrl_c = tokio::signal::windows::ctrl_c()?;
                let mut ctrl_break = tokio::signal::windows::ctrl_break()?;
                async move {
                    tokio::select! {
                        _ = ctrl_c.recv() => {}
                        _ = ctrl_break.recv() => {}
                    }
                }
            };

            #[cfg(not(any(unix, windows)))]
            let shutdown = {
                async move {
                    std::future::pending::<()>().await;
                }
            };

            tokio::select! {
                serve_result = axum::serve(listener, router) => {
                    serve_result?;
                }
                _ = shutdown => {
                    info!("Received shutdown signal, stopping HTTP transport");
                }
            }
        }
        other => {
            return Err(format!("Invalid transport type: {}", other).into());
        }
    }

    Ok(())
}
