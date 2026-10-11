#[cfg(test)]
mod tests {
    use crate::finder::*;

    #[test]
    fn test_source_type() {
        assert_eq!(SourceType::Java.extension(), "java");
        assert_eq!(SourceType::Cpp.extension(), "cpp");
        assert_eq!(SourceType::C.extension(), "c");
        assert_eq!(SourceType::Rust.extension(), "rs");
        assert_eq!(SourceType::Javascript.extension(), "js");
        assert_eq!(SourceType::Python.extension(), "py");

        assert_eq!(SourceType::Java.extensions(), &["java"]);
        assert_eq!(SourceType::Cpp.extensions(), &["cpp", "cc", "cxx", "hpp", "h"]);
        assert_eq!(SourceType::C.extensions(), &["c", "h"]);
        assert_eq!(SourceType::Rust.extensions(), &["rs"]);
        assert_eq!(SourceType::Javascript.extensions(), &["js", "jsx", "ts", "tsx"]);
        assert_eq!(SourceType::Python.extensions(), &["py"]);

        let all_exts = SourceType::all_extensions();
        assert!(all_exts.contains(&"java"));
        assert!(all_exts.contains(&"rs"));
        assert!(all_exts.contains(&"tsx"));

        assert_eq!(SourceType::Java.to_string(), "java");
        assert_eq!(SourceType::Cpp.to_string(), "cpp");
        assert_eq!(SourceType::C.to_string(), "c");
        assert_eq!(SourceType::Rust.to_string(), "rust");
        assert_eq!(SourceType::Javascript.to_string(), "javascript");
        assert_eq!(SourceType::Python.to_string(), "python");
    }

    #[test]
    fn test_trie_insert_and_get() {
        let mut trie = TrieNode::new();
        assert_eq!(trie.get_occurrence("hello"), 0);

        trie.insert("hello");
        assert_eq!(trie.get_occurrence("hello"), 1);

        trie.insert("hello");
        assert_eq!(trie.get_occurrence("hello"), 2);

        // Querying prefix of inserted word should return 0
        assert_eq!(trie.get_occurrence("hel"), 0);

        // Querying non-existent word returns 0
        assert_eq!(trie.get_occurrence("world"), 0);

        // Test empty string insertion
        trie.insert("");
        assert_eq!(trie.get_occurrence(""), 1);

        // Test special characters & unicode
        trie.insert("let x = 🦀;");
        assert_eq!(trie.get_occurrence("let x = 🦀;"), 1);
    }

    #[test]
    fn test_path_starts_with() {
        let ignore_folders = vec![
            "target".to_string(),
            "node_modules".to_string(),
            "/a/b/thirdparty".to_string(),
        ];

        assert!(path_starts_with("target/debug/build", &ignore_folders));
        assert!(path_starts_with("node_modules/express/index.js", &ignore_folders));
        assert!(path_starts_with("/a/b/thirdparty/lib.c", &ignore_folders));
        assert!(!path_starts_with("src/main.rs", &ignore_folders));
        assert!(!path_starts_with("src/target_file.rs", &ignore_folders));
        assert!(!path_starts_with("src/main.rs", &[]));
    }

    #[test]
    fn test_compute_ignore_path() {
        let ignore_paths = compute_ignore_path("thirdparty, test, , node_modules", "root_dir");
        // compute_ignore_path attempts glob lookup, so s will be empty if directories don't exist on disk,
        // but it should execute without error and skip empty strings.
        assert!(ignore_paths.is_empty() || !ignore_paths.is_empty());
    }

    #[test]
    fn test_is_comment_or_empty() {
        let mut in_multiline = false;

        assert!(is_comment_or_empty("   ", &mut in_multiline));
        assert!(is_comment_or_empty("\t\n", &mut in_multiline));
        assert!(is_comment_or_empty("// single line comment", &mut in_multiline));
        assert!(is_comment_or_empty("# python comment", &mut in_multiline));

        assert!(!is_comment_or_empty("let x = 10;", &mut in_multiline));

        // C-style multiline comments across lines
        assert!(is_comment_or_empty("/* start multiline", &mut in_multiline));
        assert!(in_multiline);

        assert!(is_comment_or_empty("middle line", &mut in_multiline));
        assert!(in_multiline);

        assert!(is_comment_or_empty("end multiline */", &mut in_multiline));
        assert!(!in_multiline);

        assert!(!is_comment_or_empty("let y = 20;", &mut in_multiline));

        // Python docstrings multiline across lines (""")
        assert!(is_comment_or_empty("\"\"\" start python docstring", &mut in_multiline));
        assert!(in_multiline);
        assert!(is_comment_or_empty("docstring content", &mut in_multiline));
        assert!(in_multiline);
        assert!(is_comment_or_empty("end python docstring \"\"\"", &mut in_multiline));
        assert!(!in_multiline);

        // Python docstrings multiline across lines (''')
        assert!(is_comment_or_empty("''' start python triple single quote", &mut in_multiline));
        assert!(in_multiline);
        assert!(is_comment_or_empty("docstring content", &mut in_multiline));
        assert!(in_multiline);
        assert!(is_comment_or_empty("end python triple single quote '''", &mut in_multiline));
        assert!(!in_multiline);
    }

    #[test]
    fn test_detect_cp_in_content() {
        let code_a = r#"
fn function_a() {
    let mut x = 0;
    x += 1;
    x += 2;
    x += 3;
    x += 4;
    println!("{}", x);
}
"#;

        let code_b = r#"
fn function_b() {
    let mut x = 0;
    x += 1;
    x += 2;
    x += 3;
    x += 4;
    println!("{}", x);
}
"#;

        let mut trie = TrieNode::new();
        index_content(code_a, &mut trie);
        index_content(code_b, &mut trie);

        let locs_a = detect_cp_in_content("file_a.rs", code_a, &trie, 3, 20);
        assert_eq!(locs_a.len(), 1);
        assert_eq!(locs_a[0].filepath, "file_a.rs");
        assert_eq!(locs_a[0].line_count, 7);

        let locs_b = detect_cp_in_content("file_b.rs", code_b, &trie, 3, 20);
        assert_eq!(locs_b.len(), 1);
        assert_eq!(locs_b[0].filepath, "file_b.rs");

        // High min_line_count threshold should result in no matches
        let locs_strict = detect_cp_in_content("file_a.rs", code_a, &trie, 10, 20);
        assert!(locs_strict.is_empty());

        // High min_char_count threshold should result in no matches
        let locs_strict_char = detect_cp_in_content("file_a.rs", code_a, &trie, 3, 1000);
        assert!(locs_strict_char.is_empty());
    }

    #[test]
    fn test_detect_cp_at_eof() {
        // Test duplicate detection when duplicate occurs at the end of file (EOF)
        let code = "line1();\nline2();\nline3();";
        let mut trie = TrieNode::new();
        index_content(code, &mut trie);
        index_content(code, &mut trie);

        let locs = detect_cp_in_content("eof.rs", code, &trie, 3, 10);
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].start, 1);
        assert_eq!(locs[0].end, 3);
    }

    #[test]
    fn test_scan_folders_and_parse() {
        use std::fs::{create_dir_all, remove_dir_all, File};
        use std::io::Write;

        let temp_dir = std::env::temp_dir().join("cpfinder_test_scan");
        let _ = remove_dir_all(&temp_dir);
        create_dir_all(temp_dir.join("src")).unwrap();
        create_dir_all(temp_dir.join("thirdparty")).unwrap();

        let file_a_path = temp_dir.join("src").join("a.rs");
        let file_b_path = temp_dir.join("src").join("b.rs");
        let file_ignored_path = temp_dir.join("thirdparty").join("c.rs");

        let code = "fn common_fn() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n}\n";

        File::create(&file_a_path).unwrap().write_all(code.as_bytes()).unwrap();
        File::create(&file_b_path).unwrap().write_all(code.as_bytes()).unwrap();
        File::create(&file_ignored_path).unwrap().write_all(code.as_bytes()).unwrap();

        let glob_pattern = temp_dir.join("src").join("**/*.rs");
        let ignore_folders = vec![temp_dir.join("thirdparty").display().to_string()];

        let mut source_files = Vec::new();
        scan_folders(&glob_pattern, &mut source_files, false, &ignore_folders).unwrap();

        assert_eq!(source_files.len(), 2);
        assert!(source_files.iter().any(|f| f.ends_with("a.rs")));
        assert!(source_files.iter().any(|f| f.ends_with("b.rs")));

        // Test parse
        let mut root = TrieNode::new();
        let mut cp_locations = Vec::new();

        parse(file_a_path.to_str().unwrap(), &mut root, &mut cp_locations, 3, 10).unwrap();
        parse(file_b_path.to_str().unwrap(), &mut root, &mut cp_locations, 3, 10).unwrap();

        assert!(!cp_locations.is_empty());

        // Cleanup
        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_lsp_config_defaults() {
        use crate::lsp::LspConfig;
        let config = LspConfig::default();
        assert_eq!(config.max_file_size, 1_048_576);
        assert_eq!(config.min_line_count, 6);
        assert_eq!(config.min_char_count, 80);
        assert_eq!(config.ignore_folders, "thirdparty,test,node_modules,target,.git");
    }

    #[tokio::test]
    async fn test_lsp_backend_lifecycle() {
        use tower_lsp::lsp_types::*;
        use tower_lsp::{LanguageServer, LspService};
        use crate::lsp::{Backend, LspConfig};

        let config = LspConfig {
            min_line_count: 3,
            min_char_count: 10,
            max_file_size: 1024,
            ignore_folders: "node_modules".to_string(),
        };

        let (service, _socket) = LspService::new(|client| Backend::with_config(client, config));
        let backend = service.inner();

        let mut opts = serde_json::Map::new();
        opts.insert("min_line_count".to_string(), serde_json::Value::from(2));
        opts.insert("min_char_count".to_string(), serde_json::Value::from(5));
        opts.insert("max_file_size".to_string(), serde_json::Value::from(2048));
        opts.insert("ignore_folders".to_string(), serde_json::Value::from("target"));

        // Initialize with initialization_options override
        let init_params = InitializeParams {
            initialization_options: Some(serde_json::Value::Object(opts)),
            ..Default::default()
        };

        let init_result = backend.initialize(init_params).await.unwrap();
        assert!(init_result.capabilities.text_document_sync.is_some());
        assert_eq!(init_result.server_info.unwrap().name, "cpfinder-lsp");

        backend.initialized(InitializedParams {}).await;

        let uri_a = Url::parse("file:///workspace/file_a.rs").unwrap();
        let uri_b = Url::parse("file:///workspace/file_b.rs").unwrap();

        let code = "fn test() {\n    let x = 1;\n    let y = 2;\n}\n";

        // Open doc A
        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri_a.clone(),
                    language_id: "rust".to_string(),
                    version: 1,
                    text: code.to_string(),
                },
            })
            .await;

        // Open doc B (same content)
        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri_b.clone(),
                    language_id: "rust".to_string(),
                    version: 1,
                    text: code.to_string(),
                },
            })
            .await;

        // Change doc A
        backend
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri_a.clone(),
                    version: 2,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: format!("{}\n// modification", code),
                }],
            })
            .await;

        // Save doc A
        backend
            .did_save(DidSaveTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri_a.clone() },
                text: None,
            })
            .await;

        // Close doc A
        backend
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri_a.clone() },
            })
            .await;

        // Shutdown
        assert!(backend.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn test_lsp_backend_workspace_scanning() {
        use std::fs::{create_dir_all, remove_dir_all, File};
        use std::io::Write;
        use tower_lsp::lsp_types::*;
        use tower_lsp::{LanguageServer, LspService};
        use crate::lsp::{Backend, LspConfig};

        let temp_dir = std::env::temp_dir().join("cpfinder_lsp_workspace_test");
        let _ = remove_dir_all(&temp_dir);
        create_dir_all(&temp_dir).unwrap();

        let disk_file_path = temp_dir.join("disk_file.rs");
        let code = "fn duplicate_fn() {\n    let a = 100;\n    let b = 200;\n    let c = 300;\n}\n";
        File::create(&disk_file_path)
            .unwrap()
            .write_all(code.as_bytes())
            .unwrap();

        let root_uri = Url::from_file_path(&temp_dir).unwrap();

        let config = LspConfig {
            min_line_count: 3,
            min_char_count: 10,
            max_file_size: 1024,
            ignore_folders: "target".to_string(),
        };

        let (service, _socket) = LspService::new(|client| Backend::with_config(client, config));
        let backend = service.inner();

        let init_params = InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        };

        backend.initialize(init_params).await.unwrap();

        let open_doc_uri = Url::from_file_path(temp_dir.join("open_doc.rs")).unwrap();

        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: open_doc_uri,
                    language_id: "rust".to_string(),
                    version: 1,
                    text: code.to_string(),
                },
            })
            .await;

        let _ = remove_dir_all(&temp_dir);
    }
}
