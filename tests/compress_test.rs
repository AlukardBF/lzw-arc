use lzw_arc::lzw;
use sha1::{Sha1, Digest};
use std::fs::{remove_file, File};

#[test]
fn compress_test() {

    lzw::compress("test-file", "compress_test", 16).unwrap();
    // Source hash
    let mut file = File::open("test-compressed").unwrap();
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher).unwrap();
    let source_hash = hasher.result();
    // Result hash
    let mut file = File::open("compress_test").unwrap();
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher).unwrap();
    let result_hash = hasher.result();
    
    remove_file("compress_test").unwrap();
    assert_eq!(source_hash, result_hash);
}
#[test]
fn decompress_test() {
    lzw::decompress("test-compressed", "decompress_test", 16).unwrap();
    // Source hash
    let mut file = File::open("test-file").unwrap();
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher).unwrap();
    let source_hash = hasher.result();
    // Result hash
    let mut file = File::open("decompress_test").unwrap();
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher).unwrap();
    let result_hash = hasher.result();

    remove_file("decompress_test").unwrap();
    assert_eq!(source_hash, result_hash);
}
#[test]
fn aes_test() {
    lzw::compress_aes("test-file", "aes_test", 16, "secret").unwrap();
    lzw::decompress_aes("aes_test", "aes_test_result", 16, "secret").unwrap();
    // Source hash
    let mut file = File::open("test-file").unwrap();
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher).unwrap();
    let source_hash = hasher.result();
    // Result hash
    let mut file = File::open("aes_test_result").unwrap();
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher).unwrap();
    let result_hash = hasher.result();
    
    remove_file("aes_test").unwrap();
    remove_file("aes_test_result").unwrap();
    assert_eq!(source_hash, result_hash);
}