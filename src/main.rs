use std::cmp::min;
use std::path::Path;

use clap::{Parser, Subcommand};
use colored::*;
use std::collections::HashMap;
use cpfinder::finder::{
    compute_ignore_path, group_cp_locations, parse, scan_folders, CPLocation, SourceType, TrieNode,
};
use cpfinder::lsp::run_lsp_server;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(help = "root folder to scan")]
    root: Option<String>,

    #[arg(help = "source file type")]
    source_type: Option<SourceType>,

    #[arg(
        long,
        default_value_t = 10,
        help = "minimum number of lines to considered as copy paste"
    )]
    min_line_count: usize,

    #[arg(
        long,
        default_value_t = 120,
        help = "minimum characters to considered as copy paste"
    )]
    min_char_count: usize,

    #[arg(
        long,
        default_value = "thirdparty,test,node_modules",
        help = "folders to ignore"
    )]
    ignore_folders: String,

    #[arg(long, default_value_t = false, help = "list source files")]
    list_source_folder: bool,

    #[arg(long, default_value_t = 30, help = "top number of results to list")]
    list_top_result: usize,

    #[arg(long, default_value_t = false, help = "run as language server")]
    lsp: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "run as language server")]
    Lsp,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.lsp || matches!(args.command, Some(Commands::Lsp)) {
        run_lsp_server().await;
        return;
    }

    let (root_folder, source_type) = match (args.root, args.source_type) {
        (Some(r), Some(st)) => (r, st),
        _ => {
            eprintln!("Error: <ROOT> and <SOURCE_TYPE> are required unless running in --lsp mode.");
            std::process::exit(1);
        }
    };

    let root_path = Path::new(&root_folder).join(format!("**/*.{}", source_type.to_string()));

    let ignore_folders = compute_ignore_path(&args.ignore_folders, &root_folder);

    let mut source_files: Vec<String> = Vec::new();
    scan_folders(
        &root_path,
        &mut source_files,
        args.list_source_folder,
        &ignore_folders,
    )
    .ok();

    let n = source_files.len();
    println!("found {} source files of {}", n, source_type);

    let mut root = TrieNode::new();
    let mut cp_locations: Vec<CPLocation> = Vec::new();
    let mut file_contents: HashMap<String, String> = HashMap::new();

    if n > 0 {
        for source_file in &source_files {
            if let Ok(content) = std::fs::read_to_string(source_file) {
                file_contents.insert(source_file.clone(), content);
            }
            parse(
                source_file,
                &mut root,
                &mut cp_locations,
                args.min_line_count,
                args.min_char_count,
            )
            .ok();
        }
    }

    let clone_groups = group_cp_locations(&cp_locations, &file_contents);

    println!("top {} result:", args.list_top_result.to_string().blue());
    for group in &clone_groups[0..min(clone_groups.len(), args.list_top_result)] {
        let loc_strs: Vec<String> = group
            .locations
            .iter()
            .map(|l| {
                format!(
                    "{} (lines {})",
                    l.filepath.red(),
                    format!("{}-{}", l.start, l.end).purple()
                )
            })
            .collect();
        println!("Clone Group: {}", loc_strs.join(" and "));
    }
}
