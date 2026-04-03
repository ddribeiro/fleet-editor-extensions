//! Fleet GitOps Language Server Protocol implementation.
//!
//! Provides real-time diagnostics, completion, hover, code actions, and
//! other editor intelligence for Fleet GitOps YAML files via tower-lsp.

pub mod backend;
pub mod code_actions;
pub mod completion;
pub mod diagnostics;
pub mod fleet;
pub mod hover;
pub mod position;
pub mod schema;
pub mod semantic_tokens;
pub mod symbols;
pub mod workspace;

use anyhow::Result;
use tower_lsp::{LspService, Server};

use backend::FleetLspBackend;
use flint_lint::Linter;

/// Start the LSP server using stdio transport.
pub async fn start_server() -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| FleetLspBackend::new(client, Linter::new()));

    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}
