use pur::validate_liberally;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

/// LSP Server for Json based LSP config validation
/// validate against the schema -> errors give {instance_path, schema_path, to_string}
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

// Json Schema Type
type Schema = Arc<serde_json::Value>;
type JsonSchemas = Arc<RwLock<HashMap<String, Schema>>>;

#[derive(Debug)]
struct Backend {
    client: Client,
    // rust analyzer uses same pattern with Arc RwLock -- Frequestn Read, Infrequesnt writes
    // wrapped json value in Arc for shared ownership in the heap.. value should not change
    json_schemas: JsonSchemas,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    // TODO load json schema for given config file on either initialize or new document was opened.
    // FOR now only implement intitialize, textDocument{didOpen, didChange, }, and
    // publishDiagnostics
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
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

    // handle did_open, did_change the same way (send whole document at once)
    // later improve this... sync state
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(OnChangeTextDocumentParams {
            uri: params.text_document.uri,
            text: &params.text_document.text,
            version: Some(params.text_document.version),
        })
        .await
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.on_change(OnChangeTextDocumentParams {
            uri: params.text_document.uri,
            text: &params.content_changes[0].text,
            version: Some(params.text_document.version),
        })
        .await
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

struct OnChangeTextDocumentParams<'document_text> {
    uri: Url,
    text: &'document_text str,
    version: Option<i32>,
}

impl Backend {
    /// this is the entry point for validating content
    /// on change is called on document text change... as well as
    async fn on_change<'document_text>(&self, params: OnChangeTextDocumentParams<'document_text>) {
        let schema = self.get_or_load_schema("service.schema").await;
        // todo improve schema_validated_filecontents later

        // match loading schema..
        // if loads, try get diagnostics, if error -> program really just panics on json_schema not
        // being a valid type
        match schema {
            Ok(schema) => {
                let diagnostics = match validate_liberally(&schema, params.text) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Error Schema Validation: {}", e);
                        return;
                    }
                };
                // publish diagnostics to client
                self.client
                    .publish_diagnostics(params.uri, diagnostics, params.version)
                    .await;
            }
            Err(e) => {
                eprintln!("Error @ {} Version:{:?}: {}", params.uri, params.version, e);
                return;
            }
        };
    }

    // for now only load schema hard coded
    // TODO discover schema from text, then search hashmap, then try to load from source somewhere
    async fn get_or_load_schema(&self, key: &str) -> tokio::io::Result<Schema> {
        // search for existing.. if not found add
        {
            let schemas = self.json_schemas.read().await;
            if let Some(schema) = schemas.get(key) {
                // cheap clone only reference
                return Ok(schema.clone());
            }
        }

        // COME BACK HERE LATER FOR EMBEDDING JSON SCHEMAS
        const SERVICE_SCHEMA: &str = include_str!("../schemas/service.schema.json");

        // search file obtain schema
        // TODO unhardcode schema this
        let schema: serde_json::Value = serde_json::from_str(SERVICE_SCHEMA)?;

        // write with lock + clone schema so it can be returned
        let mut schemas = self.json_schemas.write().await;
        schemas
            .entry(key.to_owned())
            .or_insert(Arc::new(schema.clone()));

        Ok(Arc::new(schema))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // load json_schema here for testing TODO make function for this + load to HashMap

    let (service, socket) = LspService::new(|client| Backend {
        client: client,
        json_schemas: JsonSchemas::default(),
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
