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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CPLocation {
    pub filepath: String,
    pub start: usize,
    pub end: usize,
    pub line_count: usize,
    pub char_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneGroup {
    pub locations: Vec<CPLocation>,
    pub line_count: usize,
    pub char_count: usize,
}

#[derive(Default, Debug)]
pub struct TrieNode {
    pub children: HashMap<char, TrieNode>,
    pub occurence: usize,
    pub locations: Vec<(String, usize)>, // (filepath, line_number)
}

impl TrieNode {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, word: &str, filepath: &str, line_num: usize) -> usize {
        let mut node = self;
        for char in word.chars() {
            let next_node = node.children.entry(char).or_insert_with(TrieNode::new);
            node = next_node;
        }

        node.occurence += 1;
        node.locations.push((filepath.to_string(), line_num));
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

    pub fn get_node(&self, word: &str) -> Option<&TrieNode> {
        let mut node = self;
        for char in word.chars() {
            if let Some(next_node) = node.children.get(&char) {
                node = next_node;
            } else {
                return None;
            }
        }
        Some(node)
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

pub fn index_content(filepath: &str, content: &str, root: &mut TrieNode) {
    let mut in_multiline_comment = false;
    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim();
        if !is_comment_or_empty(line, &mut in_multiline_comment) {
            root.insert(trimmed, filepath, line_num);
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

pub fn group_cp_locations(
    cp_locations: &[CPLocation],
    files: &HashMap<String, String>,
) -> Vec<CloneGroup> {
    use std::collections::HashSet;

    let mut cp_map: HashMap<(String, usize), &CPLocation> = HashMap::new();
    for loc in cp_locations {
        for line in loc.start..=loc.end {
            cp_map.insert((loc.filepath.clone(), line), loc);
        }
    }

    let mut visited: HashSet<(CPLocation, CPLocation)> = HashSet::new();
    let mut clone_groups: Vec<CloneGroup> = Vec::new();

    for loc_a in cp_locations {
        let content_a = match files.get(&loc_a.filepath) {
            Some(c) => c,
            None => continue,
        };
        let lines_a: Vec<&str> = content_a.lines().collect();

        for line_a_num in loc_a.start..=loc_a.end {
            if line_a_num > lines_a.len() {
                continue;
            }
            let line_a_str = lines_a[line_a_num - 1].trim();
            let mut in_comm = false;
            if is_comment_or_empty(line_a_str, &mut in_comm) {
                continue;
            }

            for loc_b in cp_locations {
                if loc_a == loc_b {
                    continue;
                }
                if visited.contains(&(loc_a.clone(), loc_b.clone()))
                    || visited.contains(&(loc_b.clone(), loc_a.clone()))
                {
                    continue;
                }

                let content_b = match files.get(&loc_b.filepath) {
                    Some(c) => c,
                    None => continue,
                };
                let lines_b: Vec<&str> = content_b.lines().collect();

                for line_b_num in loc_b.start..=loc_b.end {
                    if line_b_num > lines_b.len() {
                        continue;
                    }
                    let line_b_str = lines_b[line_b_num - 1].trim();

                    if line_a_str == line_b_str {
                        let mut offset = 0;
                        let mut match_len = 0;
                        let mut match_chars = 0;

                        while line_a_num + offset <= loc_a.end && line_b_num + offset <= loc_b.end {
                            let idx_a = line_a_num + offset - 1;
                            let idx_b = line_b_num + offset - 1;
                            if idx_a >= lines_a.len() || idx_b >= lines_b.len() {
                                break;
                            }
                            let str_a = lines_a[idx_a].trim();
                            let str_b = lines_b[idx_b].trim();

                            let mut comm_a = false;
                            let empty_a = is_comment_or_empty(str_a, &mut comm_a);
                            let mut comm_b = false;
                            let empty_b = is_comment_or_empty(str_b, &mut comm_b);

                            if empty_a && empty_b {
                                offset += 1;
                                match_len += 1;
                                continue;
                            }

                            if str_a == str_b {
                                offset += 1;
                                match_len += 1;
                                match_chars += str_a.len();
                            } else {
                                break;
                            }
                        }

                        if match_len >= loc_a.line_count.min(loc_b.line_count) / 2 || match_len >= 3 {
                            let match_loc_a = CPLocation {
                                filepath: loc_a.filepath.clone(),
                                start: line_a_num,
                                end: line_a_num + match_len - 1,
                                line_count: match_len,
                                char_count: match_chars,
                            };
                            let match_loc_b = CPLocation {
                                filepath: loc_b.filepath.clone(),
                                start: line_b_num,
                                end: line_b_num + match_len - 1,
                                line_count: match_len,
                                char_count: match_chars,
                            };

                            let mut group_locs = vec![match_loc_a, match_loc_b];
                            group_locs.sort_by(|x, y| x.filepath.cmp(&y.filepath).then(x.start.cmp(&y.start)));

                            clone_groups.push(CloneGroup {
                                locations: group_locs,
                                line_count: match_len,
                                char_count: match_chars,
                            });

                            visited.insert((loc_a.clone(), loc_b.clone()));
                            visited.insert((loc_b.clone(), loc_a.clone()));
                            break;
                        }
                    }
                }

                if visited.contains(&(loc_a.clone(), loc_b.clone())) {
                    break;
                }
            }
        }
    }

    if clone_groups.is_empty() && cp_locations.len() >= 2 {
        for i in 0..cp_locations.len() {
            for j in (i + 1)..cp_locations.len() {
                let loc_a = &cp_locations[i];
                let loc_b = &cp_locations[j];
                let line_cnt = loc_a.line_count.min(loc_b.line_count);
                let char_cnt = loc_a.char_count.min(loc_b.char_count);
                let mut group_locs = vec![loc_a.clone(), loc_b.clone()];
                group_locs.sort_by(|x, y| x.filepath.cmp(&y.filepath).then(x.start.cmp(&y.start)));
                clone_groups.push(CloneGroup {
                    locations: group_locs,
                    line_count: line_cnt,
                    char_count: char_cnt,
                });
            }
        }
    }

    clone_groups.sort_by_key(|g| g.line_count);
    clone_groups.reverse();
    clone_groups
}

pub fn parse(
    filepath: &str,
    root: &mut TrieNode,
    cp_locations: &mut Vec<CPLocation>,
    min_line_count: usize,
    min_char_count: usize,
) -> io::Result<()> {
    let content = std::fs::read_to_string(filepath)?;
    index_content(filepath, &content, root);
    let mut locs = detect_cp_in_content(filepath, &content, root, min_line_count, min_char_count);
    cp_locations.append(&mut locs);
    Ok(())
}
