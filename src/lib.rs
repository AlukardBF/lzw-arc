pub mod lzw {
    pub mod archive {
        use bitvec::{bitvec, BigEndian, BitVec};
        use std::collections::HashMap;
        use std::fs::File;
        use std::io::{BufReader, Read, Write};
        use std::path::PathBuf;
        type Index = u32;
        pub struct Compress {
            // Для компрессии HashMap, для декомпрессии BTreeMap (вектор?)
            // Словарь, для архивации
            dictionary: HashMap<Vec<u8>, Index>,
            // Номер последнего ключа в словаре
            last_in_dic: Index,
            // Путь к исходному файлу
            source_file: PathBuf,
            // Путь к конечному файлу
            result_file: PathBuf,
            // Текущее количество бит в максимальном значении словаря
            bits_count: u8,
            // Максимальное количество бит, т.е. размер словаря
            max_bits_count: u8,
        }
        pub struct Decompress {
            // Для компрессии HashMap, для декомпрессии BTreeMap (вектор?)
            // Словарь, для архивации
            dictionary: Vec<Vec<u8>>,
            // Номер последнего ключа в словаре
            last_in_dic: Index,
            // Путь к исходному файлу
            source_file: PathBuf,
            // Путь к конечному файлу
            result_file: PathBuf,
            // Текущее количество бит в максимальном значении словаря
            bits_count: u8,
            // Максимальное количество бит, т.е. размер словаря
            max_bits_count: u8,
        }
        impl Compress {
            /// Инициализируем структуру начальными значениями
            pub fn new(source_file: &str, result_file: &str, max_bits_count: u8) -> Compress {
                if max_bits_count > 32 || max_bits_count < 9 {
                    panic!("Недопустимый размер словаря! Разрешенный: 9 <= n <= 32");
                }
                if !std::path::Path::new(source_file).exists() {
                    panic!("Исходный файл не существует!");
                }
                // let dictionary = reset_compress_dictionary();
                Compress {
                    dictionary: HashMap::new(),
                    last_in_dic: 0,
                    bits_count: 0,
                    source_file: PathBuf::from(source_file),
                    result_file: PathBuf::from(result_file),
                    max_bits_count,
                }
            }
            #[cfg(debug_assertions)]
            pub fn new_test(source_file: &str, result_file: &str, max_bits_count: u8) -> Compress {
                if max_bits_count > 32 || max_bits_count < 9 {
                    panic!("Недопустимый размер словаря! Разрешенный: 9 <= n <= 32");
                }
                if !std::path::Path::new(source_file).exists() {
                    panic!("Исходный файл не существует!");
                }
                Compress {
                    dictionary: test_compress(),
                    last_in_dic: 4,
                    bits_count: 3,
                    source_file: PathBuf::from(source_file),
                    result_file: PathBuf::from(result_file),
                    max_bits_count,
                }
            }
            pub fn compress(&mut self) -> std::io::Result<()> {
                self.reset_compress_dictionary();
                // Открываем исходный файл и подключаем его к буферу
                let source_file = File::open(self.source_file.as_path())?;
                let mut reader = BufReader::new(source_file);
                // Выходной поток
                let mut result_file = File::create(self.result_file.as_path())?;
                // Буфер для считываемого байта
                let mut buf = [0u8; 1];
                // Предыдущая строка
                let mut prev: Vec<u8> = Vec::with_capacity(64);
                // Буфер из бит, для добавления в результирующий поток
                let mut bit_buf: BitVec<BigEndian, u8> = BitVec::with_capacity(64);
                // Основной цикл алгоритма. Считываем по одному байту, пока не закончится файл
                while reader.read(&mut buf)? == buf.len() {
                    // Текущий символ
                    let current: u8 = buf[0];
                    prev.push(current);
                    // Набор байт уже присутствует в словаре?
                    if !self.dictionary.contains_key(&prev) {
                        // Добавляем P в буфер
                        self.append_to_buf(&mut bit_buf, prev[0..prev.len() - 1].to_vec());
                        // Меняем номер последнего ключа в словаре
                        self.add_element_count();
                        // P + C в словарь
                        self.dictionary.insert(prev.clone(), self.last_in_dic);
                        // P = C
                        prev.clear();
                        prev.push(current);
                        //Проверяем, может ли добавить что-то в файл
                        while let Some(byte) = pop_byte(&mut bit_buf) {
                            result_file.write_all(&[byte])?;
                        }
                    }
                }
                // Добавляем в буфер оставшиеся байты
                self.append_to_buf(&mut bit_buf, prev);
                let last_bytes: Vec<u8> = bit_buf.as_slice().iter().cloned().collect();
                // Добавляем в файл последние байты, дополняя их нулями
                result_file.write_all(&last_bytes)?;
                Ok(())
            }
            /// Добавляем в буфер кодовое значение из словаря, для дальнейшего добавления в файл
            fn append_to_buf(&self, bit_buf: &mut BitVec<BigEndian, u8>, value: Vec<u8>) {
                let bv = *self.dictionary.get(&value).expect(
                    "Ошибка при получении значения из словаря",
                );
                bit_buf.append(&mut from_index(bv, self.bits_count));
            }
            // Увеличиваем счетчик словаря
            fn _old_add_element_count(&mut self) {
                self.last_in_dic += 1;
                let bits_count = 32 - self.last_in_dic.leading_zeros() as u8;
                // Сбрасываем словарь, если достигли максимального количества бит
                if bits_count > self.max_bits_count {
                    self.reset_compress_dictionary();
                    self.bits_count = 8;
                    self.last_in_dic = 255;
                } else {
                    self.bits_count = bits_count;
                }
            }
            fn add_element_count(&mut self) -> bool {
                // let bits_count = 32 - self.last_in_dic.leading_zeros() as u8;
                let bits_count = get_bits_count(self.dictionary.len() as Index) as u8;
                // Сбрасываем словарь, если достигли максимального количества бит
                // if bits_count > self.max_bits_count {
                if self.dictionary.len() + 1 == (1 << self.max_bits_count) as usize {
                    self.reset_compress_dictionary();
                    self.last_in_dic += 1;
                    true
                } else {
                    self.bits_count = bits_count;
                    self.last_in_dic += 1;
                    false
                }
            }
            fn reset_compress_dictionary(&mut self) {
                // Инициализируем словарь из всех значений, которые можно хранить
                // в одном байте (0..255)
                self.dictionary.clear();
                for ch in u8::min_value()..=u8::max_value() {
                    self.dictionary.insert(vec![ch], u32::from(ch));
                }
                self.bits_count = 8;
                self.last_in_dic = 255;
            }
        }
        impl Decompress {
            /// Инициализируем структуру начальными значениями
            pub fn new(source_file: &str, result_file: &str, max_bits_count: u8) -> Decompress {
                if max_bits_count > 32 || max_bits_count < 9 {
                    panic!("Недопустимый размер словаря! Разрешенный: 9 <= n <= 32");
                }
                if !std::path::Path::new(source_file).exists() {
                    panic!("Исходный файл не существует!");
                }
                Decompress {
                    dictionary: Vec::new(),
                    last_in_dic: 0,
                    bits_count: 0,
                    source_file: PathBuf::from(source_file),
                    result_file: PathBuf::from(result_file),
                    max_bits_count,
                }
            }
            #[cfg(debug_assertions)]
            pub fn new_test(source_file: &str, result_file: &str, max_bits_count: u8) -> Decompress {
                if max_bits_count > 32 || max_bits_count < 9 {
                    panic!("Недопустимый размер словаря! Разрешенный: 9 <= n <= 32");
                }
                if !std::path::Path::new(source_file).exists() {
                    panic!("Исходный файл не существует!");
                }
                Decompress {
                    dictionary: Vec::new(),
                    last_in_dic: 0,
                    bits_count: 0,
                    source_file: PathBuf::from(source_file),
                    result_file: PathBuf::from(result_file),
                    max_bits_count,
                }
            }
            pub fn _old_decompress(&mut self) -> std::io::Result<()> {
                // Открываем исходный файл и подключаем его к буферу
                let source_file = File::open(self.source_file.as_path())?;
                let mut reader = BufReader::new(source_file);
                // Выходной поток
                let mut result_file = File::create(self.result_file.as_path())?;
                // Буфер для считываемого байта
                let mut buf = [0u8; 1];
                // Буфер из бит, для добавления в результирующий поток
                let mut bit_buf: BitVec<BigEndian, u8> = BitVec::with_capacity(64);
                // Инициализация. Считываем первый байт
                if reader.read(&mut buf)? != buf.len() {
                    panic!("Передан пустой файл");
                }
                // Считываем первый индекс
                bit_buf.append(&mut from_index(buf[0] as Index, 8));
                // Извлекаем индекс
                let index: Index = pop_first_bits(&mut bit_buf, self.bits_count).expect(
                    "Ошибка в извлечении индекса из битового буфера"
                );
                // Записываем его в результирующий файл
                let mut index = index as usize;
                let first_byte = &self.dictionary[index];
                result_file.write_all(&first_byte[..])?;
                // Добавляем к текущему
                self.add_element_count();
                // Основной цикл алгоритма
                loop {
                    let prev_index = index;
                    // Считываем из буфера по байту, пока не достигнем нужного,
                    // для извлечения индекса, количества бит
                    while bit_buf.len() < self.bits_count as usize {
                        if reader.read(&mut buf)? != buf.len() {
                            // Если в файле не хватило данных для нового индекса,
                            // Значит встретили конец, добитый нулями до полного байта
                            // Заканчиваем выполнение алгоритма
                            return Ok(())
                        }
                        bit_buf.append(&mut from_index(buf[0] as Index, 8));
                    }
                    // Извлекаем индекс
                    let index_tmp: Index = pop_first_bits(&mut bit_buf, self.bits_count).expect(
                        "Ошибка в извлечении индекса из битового буфера"
                    );
                    // Уменьшаем счетчик количества доступных в буфере бит
                    // Меняем тип к usize, чтобы индексировать вектор
                    index = index_tmp as usize;
                    // Если считанный индекс существует
                    if self.dictionary.len() > index {
                        // println!("TRUE len: {0}, index: {1}, prev_index: {2}", self.dictionary.len(), index, prev_index);
                        // Write the string at <index> to the result
                        let bytes = &self.dictionary[index];
                        result_file.write_all(&bytes[..])?;
                        // B ← first byte of the string at <index>
                        let mut string: Vec<u8> = match self.dictionary.get(prev_index) {
                            Some(string) => string.clone(),
                            None => Vec::with_capacity(1),
                        };
                        // let mut string = self.dictionary[prev_index].clone();
                        string.push(bytes[0]);
                        // Add <old>B to the dictionary
                        self.add_element_count();
                        self.dictionary.push(string);
                    } else {
                        // println!("FALSE len: {0}, index: {1}, prev_index: {2}", self.dictionary.len(), index, prev_index);
                        // Add <old>B to the dictionary
                        // let mut string = self.dictionary[prev_index].clone();
                        let mut string: Vec<u8> = match self.dictionary.get(prev_index) {
                            Some(string) => string.clone(),
                            None => Vec::with_capacity(1),
                        };
                        // B ← first byte of the string at <old>
                        let first_byte = string[0];
                        // Write the string for <old>B to the output
                        string.push(first_byte);
                        result_file.write_all(&string[..])?;
                        self.add_element_count();
                        self.dictionary.push(string);
                    }
                }
                // Ok(())
            }
            pub fn decompress(&mut self) -> std::io::Result<()> {
                self.reset_decompress_dictionary();
                // Открываем исходный файл и подключаем его к буферу
                let source_file = File::open(self.source_file.as_path())?;
                let mut reader = BufReader::new(source_file);
                // Выходной поток
                let mut result_file = File::create(self.result_file.as_path())?;
                // Буфер для считываемого байта
                let mut buf = [0u8; 1];
                // Буфер из бит, для добавления в результирующий поток
                let mut bit_buf: BitVec<BigEndian, u8> = BitVec::with_capacity(64);

                let mut index: usize;
                let mut string: Vec<u8> = Vec::new();
                // Количество бит для первого числа
                let mut bits_count = get_bits_count((self.dictionary.len() - 1) as Index);
                // Основной цикл алгоритма
                loop {
                    
                    // Считываем из буфера по байту, пока не достигнем нужного,
                    // для извлечения индекса, количества бит
                    while bit_buf.len() < bits_count as usize {
                        if reader.read(&mut buf)? != buf.len() {
                            // Если в файле не хватило данных для нового индекса,
                            // Значит встретили конец, добитый нулями до полного байта
                            // Заканчиваем выполнение алгоритма
                            return Ok(())
                        }
                        bit_buf.append(&mut from_index(buf[0] as Index, 8));
                    }
                    // Извлекаем индекс
                    let index_tmp: Index = pop_first_bits(&mut bit_buf, bits_count as u8).expect(
                        "Ошибка в извлечении индекса из битового буфера"
                    );
                    // Меняем тип к usize, чтобы индексировать вектор
                    index = index_tmp as usize;

                    // let mut decomp_code = string.clone();
                    // Если считанный индекс существует
                    if index > self.dictionary.len() {
                        panic!("Неверный зашифрованный код");
                    } else if index == self.dictionary.len() {
                        string.push(string[0]);
                    } else if !string.is_empty() {
                        string.push(self.dictionary.get(index).unwrap()[0]);
                    }
                    if !string.is_empty() {
                        self.dictionary.push(string);
                    }                    
                    let code = self.dictionary.get(index).expect(
                        "Ошибка в извлечении индекса из битового буфера"
                    );
                    result_file.write_all(&code[..])?;
                    string = code.to_vec();
                    
                    // Сбрасываем словарь, если наполнили его
                    if self.dictionary.len() + 1 == 1 << self.max_bits_count as usize {
                        self.reset_decompress_dictionary();
                        bits_count = get_bits_count((self.dictionary.len() - 1) as Index);
                    } else {
                        // Количество бит для считывания следующего индекса
                        bits_count = get_bits_count(self.dictionary.len() as Index);
                    }
                }
                // Ok(())
            }
            // Увеличиваем счетчик словаря
            fn _old_add_element_count(&mut self) -> bool {
                self.last_in_dic += 1;
                let bits_count = 32 - self.last_in_dic.leading_zeros() as u8;
                // Сбрасываем словарь, если достигли максимального количества бит
                if bits_count > self.max_bits_count {
                    self.reset_decompress_dictionary();
                    self.bits_count = 8;
                    self.last_in_dic = 255;
                    true
                } else {
                    self.bits_count = bits_count;
                    false
                }
            }
            fn add_element_count(&mut self) -> bool {
                self.last_in_dic += 1;
                let bits_count = 32 - self.last_in_dic.leading_zeros() as u8;

                // Сбрасываем словарь, если достигли максимального количества бит
                if bits_count > self.max_bits_count {
                    self.reset_decompress_dictionary();
                    true
                } else {
                    self.bits_count = bits_count;
                    false
                }
            }
            fn reset_decompress_dictionary(&mut self) {
                // Инициализируем словарь из всех значений, которые можно хранить
                // в одном байте (0..255)
                self.dictionary.clear();
                for ch in u8::min_value()..=u8::max_value() {
                    self.dictionary.push(vec![ch]);
                }
                self.bits_count = 8;
                self.last_in_dic = 255;
            }
            fn test_reset_decompress_dictionary(&mut self) {
                self.dictionary.clear();
                self.dictionary.push(vec![97u8]);
                self.dictionary.push(vec![98u8]);
                self.dictionary.push(vec![99u8]);
                self.dictionary.push(vec![100u8]);
                self.dictionary.push(vec![101u8]);
                self.bits_count = 3;
                self.last_in_dic = 4;
            }
        }
        /*fn reset_compress_dictionary() -> HashMap<Vec<u8>, u32> {
            let mut dictionary: HashMap<Vec<u8>, u32> = HashMap::new();
            // Инициализируем словарь из всех значений, которые можно хранить
            // в одном байте (0..255)
            for ch in u8::min_value()..=u8::max_value() {
                dictionary.insert(vec![ch], u32::from(ch));
            }
            dictionary
        }*/
        #[cfg(debug_assertions)]
        fn test_compress() -> HashMap<Vec<u8>, u32> {
            let mut dictionary: HashMap<Vec<u8>, u32> = HashMap::new();
            dictionary.insert(vec![97u8], 0);
            dictionary.insert(vec![98u8], 1);
            dictionary.insert(vec![99u8], 2);
            dictionary.insert(vec![100u8], 3);
            dictionary.insert(vec![101u8], 4);
            dictionary
        }
        #[cfg(debug_assertions)]
        fn test_decompress() -> Vec<Vec<u8>> {
            let mut dictionary: Vec<Vec<u8>> = Vec::new();
            dictionary.push(vec![97u8]);
            dictionary.push(vec![98u8]);
            dictionary.push(vec![99u8]);
            dictionary.push(vec![100u8]);
            dictionary.push(vec![101u8]);
            dictionary
        }
        /// Переводит число в BitVec. Обрезает лидирующие нули, однако не больше чем bits
        fn _from_u32(value: u32, bits: u8) -> BitVec<BigEndian, u8> {
            let mut bv: BitVec<BigEndian, u8> = BitVec::with_capacity(bits as usize);
            for i in (0..bits).rev() {
                // Добавляем i-ый бит в bv
                bv.push(((1 << i) & value) != 0);
            }
            bv
        }
        fn get_bits_count(length: Index) -> u32 {
            let bits_in_type = Index::from(0u8).count_zeros();
            bits_in_type - length.leading_zeros()
        }
        fn from_index(value: Index, bits: u8) -> BitVec<BigEndian, u8> {
            let mut bv: BitVec<BigEndian, u8> = BitVec::with_capacity(bits as usize);
            for i in (0..bits).rev() {
                // Добавляем i-ый бит в bv
                bv.push(((1 << i) & value) != 0);
            }
            bv
        }
        // Получаем из BitVec байты (u8) для записи в файл
        fn pop_byte(bv: &mut BitVec<BigEndian, u8>) -> Option<u8> {
            if let Some(byte) = pop_first_bits(bv, 8) {
                return Some(byte as u8);
            }
            None
        }
        // Получаем из BitVec число, состоящее из первых bits бит
        fn pop_first_bits(bv: &mut BitVec<BigEndian, u8>, bits: u8) -> Option<Index> {
            let bits = bits as usize;
            // Если есть что получить из буфера
            if bv.len() >= bits {
                let bv2 = bv.split_off(bits);
                let mut index: Index = 0;
                // Преобразовываем BitVec в Index
                for (i, j) in (0..bv.len()).rev().enumerate() {
                    index |= (bv[j] as Index) << i;
                }
                *bv = bv2;
                return Some(index);
            }
            None
        }
    }
}
