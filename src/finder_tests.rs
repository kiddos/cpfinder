#[cfg(test)]
mod tests {
    use crate::finder::*;

    #[test]
    fn test_trie_insert_and_get() {
        let mut trie = TrieNode::new();
        assert_eq!(trie.get_occurrence("hello"), 0);

        trie.insert("hello");
        assert_eq!(trie.get_occurrence("hello"), 1);

        trie.insert("hello");
        assert_eq!(trie.get_occurrence("hello"), 2);
    }

    #[test]
    fn test_is_comment_or_empty() {
        let mut in_multiline = false;

        assert!(is_comment_or_empty("   ", &mut in_multiline));
        assert!(is_comment_or_empty("// single line comment", &mut in_multiline));
        assert!(is_comment_or_empty("# python comment", &mut in_multiline));

        assert!(!is_comment_or_empty("let x = 10;", &mut in_multiline));

        assert!(is_comment_or_empty("/* start multiline", &mut in_multiline));
        assert!(in_multiline);

        assert!(is_comment_or_empty("middle line", &mut in_multiline));
        assert!(in_multiline);

        assert!(is_comment_or_empty("end multiline */", &mut in_multiline));
        assert!(!in_multiline);

        assert!(!is_comment_or_empty("let y = 20;", &mut in_multiline));
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
        assert!(!locs_a.is_empty());

        let locs_b = detect_cp_in_content("file_b.rs", code_b, &trie, 3, 20);
        assert!(!locs_b.is_empty());
    }
}
