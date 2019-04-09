use lzw_arc::lzw;
use same_file::is_same_file;
use std::fs::remove_file;

#[test]
fn compress_test() {
    lzw::compress("test-file", "compress_test", 16).unwrap();
    let is_same = is_same_file("test-compressed", "compress_test").unwrap();
    remove_file("compress_test").unwrap();
    assert!(is_same);
}
#[test]
fn decompress_test() {
    lzw::decompress("test-compressed", "decompress_test", 16).unwrap();
    let is_same = is_same_file("test-file", "decompress_test").unwrap();
    remove_file("decompress_test").unwrap();
    assert!(is_same);
}
#[test]
fn aes_test() {
    lzw::compress_aes("test-file", "aes_test", 16, "secret").unwrap();
    lzw::decompress_aes("aes_test", "aes_test_result", 16, "secret").unwrap();
    let is_same = is_same_file("test-file", "aes_test_result").unwrap();
    remove_file("aes_test").unwrap();
    remove_file("aes_test_result").unwrap();
    assert!(is_same);
}
