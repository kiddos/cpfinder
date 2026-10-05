use std::collections::HashMap;
use std::io;
use std::path::Path;

use clap::ValueEnum;
use glob::glob;

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum SourceType {
    Java,
    Cpp,
    C,
    Rust,
    Javascript,
    Python,
}

impl SourceType {
    pub fn extension(&self) -> &'static str {
        match self {
            SourceType::Java => "java",
            SourceType::Cpp => "cpp",
            SourceType::C => "c",
            SourceType::Rust => "rs",
            SourceType::Javascript => "js",
            SourceType::Python => "py",
        }
    }

    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            SourceType::Java => &["java"],
            SourceType::Cpp => &["cpp", "cc", "cxx", "hpp", "h"],
            SourceType::C => &["c", "h"],
            SourceType::Rust => &["rs"],
            SourceType::Javascript => &["js", "jsx", "ts", "tsx"],
            SourceType::Python => &["py"],
        }
    }

    pub fn all_extensions() -> &'static [&'static str] {
        &[
            "java", "cpp", "cc", "cxx", "hpp", "h", "c", "rs", "js", "jsx", "ts", "tsx", "py",
        ]
    }
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CPLocation {
    pub filepath: String,
    pub start: usize,
    pub end: usize,
    pub line_count: usize,
    pub char_count: usize,
}

#[derive(Default, Debug)]
pub struct TrieNode {
    pub children: HashMap<char, TrieNode>,
    pub occurence: usize,
}

impl TrieNode {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, word: &str) -> usize {
        let mut node = self;
        for char in word.chars() {
            let next_node = node.children.entry(char).or_insert_with(TrieNode::new);
            node = next_node;
        }

        node.occurence += 1;
        node.occurence
    }

    pub fn get_occurrence(&self, word: &str) -> usize {
        let mut node = self;
        for char in word.chars() {
            if let Some(next_node) = node.children.get(&char) {
                node = next_node;
            } else {
                return 0;
            }
        }
        node.occurence
    }
}

pub fn compute_ignore_path(ignore_folders: &str, root_folder: &str) -> Vec<String> {
    let mut glob_path: Vec<String> = Vec::new();
    for f in ignore_folders.split(',') {
        let f = f.trim();
        if f.is_empty() {
            continue;
        }
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
        if let Ok(entries) = glob(&p) {
            for entry in entries {
                match entry {
                    Ok(path) => {
                        s.push(path.display().to_string());
                    }
                    Err(e) => {
                        eprintln!("{:?}", e);
                    }
                }
            }
        }
    }

    s
}

pub fn path_starts_with(path: &str, ignore_folders: &[String]) -> bool {
    for f in ignore_folders {
        if path.starts_with(f) {
            return true;
        }
    }
    false
}

pub fn scan_folders(
    root_path: &Path,
    source_files: &mut Vec<String>,
    list_source_folder: bool,
    ignore_folders: &[String],
) -> Result<(), glob::PatternError> {
    for entry in glob(root_path.to_str().unwrap())? {
        match entry {
            Ok(path) => {
                let source_file = path.display().to_string();
                if path_starts_with(&source_file, ignore_folders) {
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

pub fn is_comment_or_empty(line: &str, in_multiline_comment: &mut bool) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return true;
    }

    if *in_multiline_comment {
        if trimmed.contains("*/") || trimmed.contains("\"\"\"") || trimmed.contains("'''") {
            *in_multiline_comment = false;
        }
        return true;
    }

    if trimmed.starts_with("/*") {
        if !trimmed.contains("*/") {
            *in_multiline_comment = true;
        }
        return true;
    }

    if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
        let quote = &trimmed[..3];
        if !trimmed[3..].contains(quote) {
            *in_multiline_comment = true;
        }
        return true;
    }

    if trimmed.starts_with("//") || trimmed.starts_with('#') {
        return true;
    }

    false
}

pub fn index_content(content: &str, root: &mut TrieNode) {
    let mut in_multiline_comment = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if !is_comment_or_empty(line, &mut in_multiline_comment) {
            root.insert(trimmed);
        }
    }
}

pub fn detect_cp_in_content(
    filepath: &str,
    content: &str,
    root: &TrieNode,
    min_line_count: usize,
    min_char_count: usize,
) -> Vec<CPLocation> {
    let mut cp_locations = Vec::new();
    let mut in_multiline_comment = false;
    let mut cp_found = false;
    let mut start = 0;
    let mut end = 0;
    let mut char_count = 0;

    let lines: Vec<&str> = content.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim();
        let should_index = !is_comment_or_empty(line, &mut in_multiline_comment);

        let next_cp_found = should_index && root.get_occurrence(trimmed) > 1;

        if next_cp_found {
            if !cp_found {
                start = line_num;
            }
            end = line_num;
            char_count += trimmed.len();
            cp_found = true;
        } else {
            if cp_found {
                let range = end - start + 1;
                if range >= min_line_count && char_count >= min_char_count {
                    cp_locations.push(CPLocation {
                        filepath: filepath.to_string(),
                        start,
                        end,
                        line_count: range,
                        char_count,
                    });
                }
            }
            char_count = 0;
            cp_found = false;
        }
    }

    if cp_found {
        let range = end - start + 1;
        if range >= min_line_count && char_count >= min_char_count {
            cp_locations.push(CPLocation {
                filepath: filepath.to_string(),
                start,
                end,
                line_count: range,
                char_count,
            });
        }
    }

    cp_locations
}

pub fn parse(
    filepath: &str,
    root: &mut TrieNode,
    cp_locations: &mut Vec<CPLocation>,
    min_line_count: usize,
    min_char_count: usize,
) -> io::Result<()> {
    let content = std::fs::read_to_string(filepath)?;
    index_content(&content, root);
    let mut locs = detect_cp_in_content(filepath, &content, root, min_line_count, min_char_count);
    cp_locations.append(&mut locs);
    Ok(())
}
