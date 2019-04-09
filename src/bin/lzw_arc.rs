use clap::{crate_version, App, Arg};
use lzw_arc::lzw;
fn main() -> std::io::Result<()> {
    let matches = App::new("LZW Archiver")
        .version(crate_version!())
        .author("Dmitriy H. <alukard.develop@gmail.com>")
        .about("lzw file archiver with aes encryption")
        .arg(
            Arg::with_name("mode")
                .help("a for compress and e for extract")
                .index(1)
                .possible_values(&["a", "e"])
                .required(true),
        )
        .arg(Arg::with_name("input_file").index(2).required(true))
        .arg(Arg::with_name("result_file").index(3).required(true))
        .arg(
            Arg::with_name("bits_count")
                .help("dictionary bits count, in other words, dictionary size")
                .takes_value(true)
                .short("b")
                .long("bits")
                .required(false)
                .default_value("16"),
        )
        .arg(
            Arg::with_name("password")
                .help("password, enable aes encryption")
                .takes_value(true)
                .short("p")
                .long("pass")
                .required(false),
        )
        .get_matches();

    let source_file = matches.value_of("input_file").unwrap();
    let result_file = matches.value_of("result_file").unwrap();
    let bits_count: usize = matches.value_of("bits_count").unwrap().parse().unwrap();
    match matches.value_of("mode").unwrap() {
        "a" => {
            if let Some(pass) = matches.value_of("password") {
                lzw::compress_aes(source_file, result_file, bits_count, pass)?;
            } else {
                lzw::compress(source_file, result_file, bits_count)?;
            }
        }
        "e" => {
            if let Some(pass) = matches.value_of("password") {
                lzw::decompress_aes(source_file, result_file, bits_count, pass)?;
            } else {
                lzw::decompress(source_file, result_file, bits_count)?;
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}