//! Unit tests for the `cyb›` REPL line parser.

use super::*;

#[test]
fn empty_line_is_no_args() {
    assert_eq!(split_args(""), Vec::<String>::new());
    assert_eq!(split_args("   "), Vec::<String>::new());
}

#[test]
fn whitespace_separates_bare_words() {
    assert_eq!(split_args("send alice 10"), vec!["send", "alice", "10"]);
    assert_eq!(
        split_args("  send   alice  10  "),
        vec!["send", "alice", "10"]
    );
}

#[test]
fn single_quotes_group_a_token_and_are_stripped() {
    assert_eq!(
        split_args("echo 'hello world'"),
        vec!["echo", "hello world"]
    );
}

#[test]
fn double_quotes_group_a_token_and_are_stripped() {
    assert_eq!(
        split_args("echo \"hello world\""),
        vec!["echo", "hello world"]
    );
}

#[test]
fn a_quote_type_is_literal_inside_the_other() {
    assert_eq!(split_args("echo \"it's fine\""), vec!["echo", "it's fine"]);
    assert_eq!(split_args("echo 'say \"hi\"'"), vec!["echo", "say \"hi\""]);
}

#[test]
fn empty_quoted_token_is_an_empty_argument() {
    assert_eq!(split_args("echo ''"), vec!["echo", ""]);
}

#[test]
fn adjacent_quoted_and_bare_segments_join_one_token() {
    assert_eq!(split_args("echo foo'bar'baz"), vec!["echo", "foobarbaz"]);
}

#[test]
fn unicode_content_does_not_panic() {
    assert_eq!(
        split_args("echo 你好 'мир ¢'"),
        vec!["echo", "你好", "мир ¢"]
    );
}

#[test]
fn unterminated_quote_still_yields_the_open_token() {
    // Not a crash: the trailing content is pushed once the line ends.
    assert_eq!(split_args("echo \"abc"), vec!["echo", "abc"]);
}

#[test]
fn builtins_are_recognized_and_others_are_not() {
    assert!(is_builtin("help"));
    assert!(is_builtin("quit"));
    assert!(is_builtin(""));
    assert!(!is_builtin("nonexistent-tool"));
}
