pub mod lzw {
    pub mod archive {
        use std::collections::{HashMap};
        use bitvec::{BitVec, LittleEndian};
        use std::path::PathBuf;
        use std::fs::File;
        use std::io::{Read, BufReader};
        pub struct Data {
            // Словарь, для архивации
            dictionary: HashMap<Vec<u8>, BitVec<LittleEndian, u8>>,
            // Предыдущий символ (строка)
            // previous: Vec<u8>,
            // Текущий символ (строка)
            // current: Vec<u8>,
            // Номер последнего ключа в словаре
            last_in_dic: u64,
            // Путь к исходному файлу
            source_file: PathBuf,
            // Путь к конечному файлу
            archived_file: PathBuf,
        }
        impl Data {
            /// Инициализируем структуру начальными значениями
            pub fn new(source_file: &str, archived_file: &str) -> Data {
                let mut dictionary: HashMap<Vec<u8>, BitVec<LittleEndian, u8>> = HashMap::new();
                // Инициализируем словарь из всех значений, которые можно хранить
                // в одном байте (0..255)
                for ch in u8::min_value()..=u8::max_value() {
                    dictionary.insert(vec![ch], from_byte(ch));
                }
                Data { dictionary: dictionary,
                    // previous: vec![],
                    // current: vec![],
                    last_in_dic: 255,
                    source_file: PathBuf::from(source_file),
                    archived_file: PathBuf::from(archived_file),
                }
            }
            pub fn archive(&mut self) -> std::io::Result<()> {
                // Открываем исходный файл и подключаем его к буферу
                let source_file = File::open(self.source_file.as_path())?;
                let mut reader = BufReader::new(source_file);
                // Выходной поток
                let mut archived_file = File::create(self.archived_file.as_path())?;
                // Буфер для считываемого байта
                let mut buf = [0u8; 1];
                // Предыдущая строка
                let mut prev: Vec<u8> = vec![];
                // Буфер из бит, для добавления в результирующий поток
                while reader.read(&mut buf)? == buf.len() {
                    // Текущий символ
                    let current: u8 = buf[0];
                    // union = prev + current
                    let mut union = prev.clone();
                    union.push(current);
                    // Набор байт уже присутствует в словаре?
                    if self.dictionary.contains_key(&union) {
                        prev = union;
                    } else {
                        /* Добавить в файл */
                        self.last_in_dic += 1;
                        self.dictionary.insert(union, from_u64(self.last_in_dic));
                    }
                }
                Ok(())
            }
        }
        /// From byte (u8) to bitvec
        fn from_byte(byte: u8) -> BitVec<LittleEndian, u8> {
            // Выделяем память в BitVec под 8 бит
            let mut bv: BitVec<LittleEndian, u8> = BitVec::with_capacity(8);
            for i in (0..8).rev() {
                // Добавляем i-ый бит в bv (big-endian ordered)
                bv.push(((1 << i) & byte) != 0);
            }
            bv
        }
        /// From u64 to BitVec (without leading zeros)
        pub fn from_u64(value: u64) -> BitVec<LittleEndian, u8> {
            let u64_bits_count = 64;
            // Количество бит в числе, без лидирующих нулей
            let bits_count: usize = u64_bits_count - value.leading_zeros() as usize;
            let mut bv: BitVec<LittleEndian, u8> = BitVec::with_capacity(bits_count);
            for i in 0..bits_count {
                // Добавляем i-ый бит в bv (big-endian ordered)
                bv.push(((1 << i) & value) != 0);
            }
            bv
        }
        pub fn bitvec_to_rev_vec(bv: BitVec<LittleEndian, u8>) -> Vec<u8> {
            bv.as_slice().iter().rev().cloned().collect()
        }
    }
}
fn main() -> std::io::Result<()> {
    let mut archive = lzw::archive::Data::new("test", "output");
    archive.archive()?;
    // println!("{:?}", lzw::archive::from_u64(97));
    // println!("{:?}", lzw::archive::from_u64(97).as_slice());
    // println!("{:?}", lzw::archive::from_u64(98));
    // println!("{:?}", lzw::archive::from_u64(98).as_slice());
    // println!("{:?}", lzw::archive::from_u64(24930));
    // println!("{:?}", lzw::archive::from_u64(24930).as_slice());
    // println!("{:b}", 24930);

    // let mut vec: Vec<u8> = lzw::archive::from_u64(24930).as_slice().to_vec();
    // let vec: Vec<u8> = lzw::archive::from_u64(24930).as_slice().iter().rev().cloned().collect();
    // vec.reverse();
    Ok(())
}