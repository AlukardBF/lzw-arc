use lzw_arc::lzw;
fn _test_main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut source_path = String::from("source");
    let mut destination_path = String::from("destination");
    if args.len() >= 2 {
        source_path = args[1].clone();
    }
    if args.len() >= 3 {
        destination_path = args[2].clone();
    }
    println!("Выберите режим работы: ");
    println!("1. Архивация");
    //println!("2. Распаковка");
    println!("> ");
    let mut mode_string = String::with_capacity(3);
    std::io::stdin()
        .read_line(&mut mode_string)
        .expect("Ошибка при чтении выбора");
    let mode = mode_string.trim().parse::<isize>().unwrap();
    match mode {
        1 => lzw::archive::Compress::new(&source_path, &destination_path, 16).compress()?,
        //2 => tea_cypher::decrypt(get_secret, &source_path, &destination_path)?,
        _ => panic!("Неправильный выбор режима"),
    }
    Ok(())
}
use bitvec::bitvec;
fn main() -> std::io::Result<()> {
    lzw::archive::Compress::new("test-track.m4a", "output", 16).compress()?;
    lzw::archive::Decompress::new("output", "test-output", 16).decompress()?;
    // let bv = bitvec![0, 1, 1, 1];
    // let mut index = 0u8;
    // for (i, j) in (0..bv.len()).rev().enumerate() {
    //     index |= (bv[j] as u8) << i;
    // }
    // dbg!(index);
    Ok(())
}