use lzw_arc::lzw;
use same_file::is_same_file;
use std::fs::remove_file;

#[test]
fn compress_test() {
    lzw::compress("test-file", "test-output", 16).unwrap();
    let is_same = is_same_file("test-compressed", "test-output").unwrap();
    remove_file("test-output").unwrap();
    assert!(is_same);
}
#[test]
fn decompress_test() {
    lzw::decompress("test-compressed", "test-output", 16).unwrap();
    let is_same = is_same_file("test-file", "test-output").unwrap();
    remove_file("test-output").unwrap();
    assert!(is_same);
}
