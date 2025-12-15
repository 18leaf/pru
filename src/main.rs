use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

/// LSP Server for Json based LSP config validation
/// Validation Pipeline -> attempt to parse json with serde_json -> can omit an error here .. STOP
/// and share error (add line/col if available)
///
/// next, validate against the schema -> errors give {instance_path, schema_path, to_string}
///
/// Point Json Error Pointer to an LSP Range struct (at first highlight high level path), expand to
/// ranges/attempt to find explicit range where error occurs (wait until other functionalities
/// work well)
///
/// create a diagnostic struct to share include stuff like.. range, severity, source, message
///
/// # Notes for now
/// - hard-code the schema path in test cases/have a json field at the top calld $"schema" with
/// accurate schema

#[derive(Debug)]
struct Backend {
    client: Client,
    // add json schemas here as hashmap.
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    // TODO load json schema for given config file on either initialize or new document was opened.
    // FOR now only implement intitialize, textDocument{didOpen, didChange, }, and
    // publishDiagnostics
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        // add supported capabilities here
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
            CompletionItem::new_simple("Bye".to_string(), "More detail".to_string()),
        ])))
    }

    async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("You're hovering!".to_string())),
            range: None,
        }))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
