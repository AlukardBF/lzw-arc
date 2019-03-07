pub mod lzw {
    pub mod archive {
        use bitvec::{BitVec, LittleEndian};
        use std::collections::HashMap;
        use std::fs::File;
        use std::io::{BufReader, Read, Write};
        use std::path::PathBuf;
        pub struct Data {
            // Словарь, для архивации
            dictionary: HashMap<Vec<u8>, BitVec<LittleEndian, u8>>,
            // Номер последнего ключа в словаре
            last_in_dic: u64,
            // Путь к исходному файлу
            source_file: PathBuf,
            // Путь к конечному файлу
            archived_file: PathBuf,
            // Текущее количество бит в максимальном значении словаря
            bits_count: usize,
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
                Data {
                    dictionary: test(), //dictionary
                    last_in_dic: 4, //255
                    bits_count: 3, //8
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
                // Буфер из бит, для добавления в результирующий поток. Сразу выделим память под 30 бит
                let mut bit_buf: BitVec<LittleEndian, u8> = BitVec::with_capacity(30);
                // Основной цикл алгоритма. Считываем по одному байту, пока не закончится файл
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
                        // Добавляем в буфер кодовое значение из словаря, для дальнейшего добавления в файл
                        let mut value = self.dictionary.get(&prev).expect("Ошибка при получении значения из словаря").clone();
                        println!("{:?}", value);
                        bit_buf.append(&mut value);
                        // Меняем номер последнего ключа в словаре
                        // self.last_in_dic = dbg!(self.last_in_dic + 1);
                        self.last_in_dic += 1;
                        // dbg!(test_from_u64(self.last_in_dic));
                        // P + C в словарь
                        // self.dictionary.insert(union, dbg!(from_u64(self.last_in_dic)));
                        self.dictionary.insert(union, from_u64(self.last_in_dic, &mut self.bits_count));
                        // P = C
                        prev = vec![current];
                        //Проверяем, может ли добавить что-то в файл
                        while let Some(byte) = pop_byte(&mut bit_buf) {
                            archived_file.write_all(&[byte])?;
                        }
                    }
                }
                // Добавляем в файл последний байт, дополняя его нулями
                if let Some(byte) = bit_buf.as_slice().first() {
                    archived_file.write_all(&[*byte])?;
                }
                Ok(())
            }
        }
        use bitvec::*;
        fn test() -> HashMap<Vec<u8>, BitVec<LittleEndian, u8>> {
            let mut dictionary: HashMap<Vec<u8>, BitVec<LittleEndian, u8>> = HashMap::new();
            dictionary.insert(vec![97u8], bitvec![LittleEndian, u8; 0, 0, 0]);
            dictionary.insert(vec![98u8], bitvec![LittleEndian, u8; 0, 0, 1]);
            dictionary.insert(vec![99u8], bitvec![LittleEndian, u8; 0, 1, 0]);
            dictionary.insert(vec![100u8], bitvec![LittleEndian, u8; 0, 1, 1]);
            dictionary.insert(vec![101u8], bitvec![LittleEndian, u8; 1, 0, 0]);
            dictionary
        }
        /// From byte (u8) to BitVec
        fn from_byte(byte: u8) -> BitVec<LittleEndian, u8> {
            // Выделяем память в BitVec под 8 бит
            let mut bv: BitVec<LittleEndian, u8> = BitVec::with_capacity(8);
            for i in (0..8).rev() {
                // Добавляем i-ый бит в bv (big-endian ordered)
                bv.push(((1 << i) & byte) != 0);
            }
            bv
        }        
        /// Переводит число в BitVec. Обрезает лидирующие нули, однако не больше чем bits
        /// Так же изменяет переданное число bits, сигнализируя о увеличившемся количестве бит в числе
        fn from_u64(value: u64, bits: &mut usize) -> BitVec<LittleEndian, u8> {
            let u64_bits_count = 64;
            // Количество бит в числе, без лидирующих нулей
            let mut bits_count: usize = u64_bits_count - value.leading_zeros() as usize;
            //
            if bits_count < *bits {
                bits_count = *bits;
            } else {
                *bits = bits_count;
            }
            let mut bv: BitVec<LittleEndian, u8> = BitVec::with_capacity(bits_count);
            for i in 0..bits_count {
                // Добавляем i-ый бит в bv (big-endian ordered)
                bv.push(((1 << i) & value) != 0);
            }
            bv
        }
        pub fn test_from_u64(value: u64) -> BitVec<LittleEndian, u8> {
            let u64_bits_count = 64;
            // Количество бит в числе, без лидирующих нулей
            let bits_count: usize = u64_bits_count - value.leading_zeros() as usize;
            let mut bv: BitVec<LittleEndian, u8> = BitVec::with_capacity(bits_count);
            for i in (0..bits_count).rev() {
                // Добавляем i-ый бит в bv (big-endian ordered)
                bv.push(((1 << i) & value) != 0);
            }
            bv
        }
        // Получаем из BitVec байты (u8) для записи в файл
        fn _bitvec_to_rev_vec(bv: BitVec<LittleEndian, u8>) -> Vec<u8> {
            bv.as_slice().iter().rev().cloned().collect()
        }
        fn pop_byte(bv: &mut BitVec<LittleEndian, u8>) -> Option<u8> {
            let byte: u8;
            if bv.len() >= 8 {
                let bv2 = bv.split_off(8);
                byte = bv.as_slice()[0];
                *bv = bv2;
                return Some(byte);
            }
            None
        }
    }
}
fn main() -> std::io::Result<()> {
    lzw::archive::Data::new("test", "output").archive()?;
    // dbg!(lzw::archive::test_from_u64(6));
    Ok(())
}
