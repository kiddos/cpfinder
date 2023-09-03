use std::cmp::min;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

use clap::Parser;
use clap::ValueEnum;
use colored::*;
use glob::glob;

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum SourceType {
    Java,
    Cpp,
    C,
    Rust,
    Javascript,
    Python,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    root: String,

    #[arg(help = "source file type")]
    source_type: SourceType,

    #[arg(
        long,
        default_value_t = 6,
        help = "minimum number of lines to considered as copy paste"
    )]
    min_line_count: usize,

    #[arg(
        long,
        default_value_t = 80,
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
}

fn compute_ignore_path(ignore_folders: String, root_folder: &str) -> Vec<String> {
    let mut glob_path: Vec<String> = Vec::new();
    for f in ignore_folders.split(",") {
        glob_path.push(
            Path::new(root_folder)
                .join("**")
                .join(f)
                .display()
                .to_string(),
        );
        glob_path.push(Path::new(root_folder).join(f).display().to_string());
    }

    let mut s: Vec<String> = Vec::new();
    for p in glob_path {
        for entry in glob(&p).expect("fail to glob ignore path") {
            match entry {
                Ok(path) => {
                    s.push(path.display().to_string());
                }
                Err(e) => {
                    println!("{:?}", e);
                }
            }
        }
    }

    s
}

fn path_starts_with(path: &str, ignore_folders: &Vec<String>) -> bool {
    for f in ignore_folders {
        if path.starts_with(f) {
            return true;
        }
    }

    false
}

fn scan_folders(
    root_path: &Path,
    source_files: &mut Vec<String>,
    list_source_folder: bool,
    ignore_folders: &Vec<String>,
) -> Result<(), glob::PatternError> {
    for entry in glob(root_path.to_str().unwrap())? {
        match entry {
            Ok(path) => {
                let source_file = path.display().to_string();
                if path_starts_with(&source_file, &ignore_folders) {
                    continue;
                }

                if list_source_folder {
                    println!("{}", path.display());
                }
                source_files.push(source_file);
            }
            Err(e) => println!("{:?}", e),
        }
    }

    Ok(())
}

struct TrieNode {
    children: HashMap<char, TrieNode>,
    occurence: usize,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            occurence: 0,
        }
    }

    fn insert(&mut self, word: &str) -> usize {
        let mut node = self;
        for char in word.chars() {
            let next_node = node.children.entry(char).or_insert(TrieNode::new());
            node = next_node;
        }

        node.occurence += 1;
        node.occurence
    }
}

struct CPLocation {
    filepath: String,
    start: usize,
    end: usize,
}

fn parse(
    filepath: &str,
    root: &mut TrieNode,
    cp_locations: &mut Vec<CPLocation>,
    min_line_count: usize,
    min_char_count: usize,
) -> io::Result<()> {
    let file = File::open(filepath)?;
    let mut reader = BufReader::new(file);

    let mut comments = false;
    let mut cp_found = false;
    let mut start = 0;
    let mut end = 0;
    let mut line_num = 1;
    let mut char_count = 0;
    loop {
        let mut line = String::new();
        let len = reader.read_line(&mut line)?;
        if len == 0 {
            break;
        }

        line = line.trim().to_string();

        if line.starts_with("/*") {
            comments = true;
        }
        if line.ends_with("*/") {
            comments = false;
        }

        if line.starts_with("//") {
            comments = true;
        }

        let should_index = !comments && !line.is_empty();

        let next_cp_found;
        if should_index {
            let o = root.insert(&line);
            if o > 1 {
                next_cp_found = true;
            } else {
                next_cp_found = false;
            }
        } else {
            next_cp_found = false;
        }

        if next_cp_found {
            if !cp_found {
                start = line_num;
            }
            end = line_num;
            char_count += line.len();

            cp_found = true;
        } else {
            if cp_found {
                let range = end - start + 1;
                if range >= min_line_count && char_count >= min_char_count {
                    cp_locations.push(CPLocation {
                        filepath: filepath.to_string(),
                        start,
                        end,
                    })
                }
            }

            char_count = 0;
            cp_found = false;
        }

        line_num += 1;
    }

    Ok(())
}

fn main() {
    let args = Args::parse();

    let root_folder = args.root;
    let root_path = Path::new(&root_folder).join(format!("**/*.{}", args.source_type.to_string()));
    // println!("{}", root_path.display());

    let ignore_folders = compute_ignore_path(args.ignore_folders, &root_folder);
    // println!("ignore folders:");
    // for f in &ignore_folders {
    //     println!("{}", f);
    // }

    let mut source_files: Vec<String> = Vec::new();
    scan_folders(
        &root_path,
        &mut source_files,
        args.list_source_folder,
        &ignore_folders,
    )
    .ok();

    let n = source_files.len();
    println!("found {} source files of java", n);

    let mut root = TrieNode::new();
    let mut cp_locations: Vec<CPLocation> = Vec::new();
    if n > 0 {
        for i in 0..n {
            parse(
                &source_files[i],
                &mut root,
                &mut cp_locations,
                args.min_line_count,
                args.min_char_count,
            )
            .ok();
        }
    }

    cp_locations.sort_by_key(|l| l.end - l.start + 1);
    cp_locations.reverse();
    println!("top {} result:", args.list_top_result.to_string().blue());
    for l in &cp_locations[0..min(cp_locations.len(), args.list_top_result)] {
        println!(
            "{}: line {}~{}",
            l.filepath.red(),
            l.start.to_string().purple(),
            l.end.to_string().purple()
        );
    }
}
