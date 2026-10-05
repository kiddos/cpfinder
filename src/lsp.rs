use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::finder::{
    compute_ignore_path, detect_cp_in_content, index_content, path_starts_with, SourceType, TrieNode,
};

#[derive(Debug)]
pub struct LspConfig {
    pub min_line_count: usize,
    pub min_char_count: usize,
    pub ignore_folders: String,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            min_line_count: 6,
            min_char_count: 80,
            ignore_folders: "thirdparty,test,node_modules,target,.git".to_string(),
        }
    }
}

pub struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
    workspace_folders: Arc<RwLock<Vec<PathBuf>>>,
    config: Arc<RwLock<LspConfig>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::default(),
            workspace_folders: Arc::default(),
            config: Arc::default(),
        }
    }

    async fn scan_and_publish_diagnostics(&self) {
        let documents = self.documents.read().await;
        let workspace_folders = self.workspace_folders.read().await;
        let config = self.config.read().await;

        let mut disk_files: HashMap<PathBuf, String> = HashMap::new();

        // Discover files in workspace folders if workspace_folders is non-empty
        for folder in workspace_folders.iter() {
            let ignore_paths = compute_ignore_path(&config.ignore_folders, folder.to_str().unwrap_or(""));
            for ext in SourceType::all_extensions() {
                let pattern = folder.join(format!("**/*.{}", ext));
                if let Ok(entries) = glob::glob(pattern.to_str().unwrap_or("")) {
                    for entry in entries.flatten() {
                        let path_str = entry.display().to_string();
                        if path_starts_with(&path_str, &ignore_paths) {
                            continue;
                        }
                        if let Ok(content) = std::fs::read_to_string(&entry) {
                            disk_files.insert(entry, content);
                        }
                    }
                }
            }
        }

        // Overlay open documents over disk_files
        let mut all_files: HashMap<Url, String> = HashMap::new();
        for (path, content) in disk_files {
            if let Ok(url) = Url::from_file_path(&path) {
                all_files.insert(url, content);
            }
        }
        for (url, content) in documents.iter() {
            all_files.insert(url.clone(), content.clone());
        }

        // Build global trie
        let mut root = TrieNode::new();
        for content in all_files.values() {
            index_content(content, &mut root);
        }

        // Generate diagnostics for all open documents (and files in workspace)
        for (url, content) in all_files.iter() {
            // Only publish diagnostics for documents that are currently open or part of workspace
            let is_open = documents.contains_key(url);
            if !is_open {
                continue;
            }

            let path_str = url.path();
            let cp_locs = detect_cp_in_content(
                path_str,
                content,
                &root,
                config.min_line_count,
                config.min_char_count,
            );

            let mut diagnostics = Vec::new();
            let lines: Vec<&str> = content.lines().collect();

            for loc in cp_locs {
                let start_line = (loc.start.saturating_sub(1)) as u32;
                let end_line = (loc.end.saturating_sub(1)) as u32;

                let start_col = 0;
                let end_col = if (end_line as usize) < lines.len() {
                    lines[end_line as usize].len() as u32
                } else {
                    0
                };

                let range = Range {
                    start: Position::new(start_line, start_col),
                    end: Position::new(end_line, end_col),
                };

                let diagnostic = Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::HINT),
                    code: None,
                    code_description: None,
                    source: Some("cpfinder".to_string()),
                    message: format!(
                        "Duplicated code detected across project ({} lines, {} chars)",
                        loc.line_count, loc.char_count
                    ),
                    related_information: None,
                    tags: None,
                    data: None,
                };

                diagnostics.push(diagnostic);
            }

            self.client.publish_diagnostics(url.clone(), diagnostics, None).await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let mut folders = Vec::new();
        if let Some(workspace_folders) = params.workspace_folders {
            for folder in workspace_folders {
                if let Ok(path) = folder.uri.to_file_path() {
                    folders.push(path);
                }
            }
        } else if let Some(root_uri) = params.root_uri {
            if let Ok(path) = root_uri.to_file_path() {
                folders.push(path);
            }
        }

        {
            let mut wf = self.workspace_folders.write().await;
            *wf = folders;
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "cpfinder-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "cpfinder LSP server initialized!")
            .await;
        self.scan_and_publish_diagnostics().await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;

        self.documents.write().await.insert(uri, text);
        self.scan_and_publish_diagnostics().await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            self.documents.write().await.insert(uri, change.text);
            self.scan_and_publish_diagnostics().await;
        }
    }

    async fn did_save(&self, _params: DidSaveTextDocumentParams) {
        self.scan_and_publish_diagnostics().await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.write().await.remove(&uri);
        self.client.publish_diagnostics(uri, vec![], None).await;
        self.scan_and_publish_diagnostics().await;
    }
}

pub async fn run_lsp_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
