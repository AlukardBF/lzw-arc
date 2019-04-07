pub mod lzw {
    use bitvec::{BigEndian, BitVec};
    use indexmap::IndexSet;
    use std::fs::File;
    use std::io::{BufReader, Read, Write};
    type Index = u32;
    struct Compress {
        // Словарь, для архивации
        dictionary: IndexSet<Vec<u8>>,
        // Текущее количество бит в максимальном значении словаря
        bits_count: u8,
        // Максимальное количество бит, т.е. размер словаря
        max_bits_count: u8,
    }
    struct Decompress {
        // Словарь, для архивации
        dictionary: Vec<Vec<u8>>,
        // Максимальное количество бит, т.е. размер словаря
        max_bits_count: u8,
    }
    impl Compress {
        /// Инициализируем структуру начальными значениями
        fn new(max_bits_count: u8) -> Compress {
            if max_bits_count > 32 || max_bits_count < 9 {
                panic!("Недопустимый размер словаря! Разрешенный: 9 <= n <= 32");
            }
            Compress {
                dictionary: IndexSet::new(),
                bits_count: 0,
                max_bits_count,
            }
        }
        fn compress<R: Read, W: Write>(
            &mut self,
            reader: R,
            writer: &mut W,
        ) -> std::io::Result<()> {
            // Задаем начальный словарь
            self.reset_compress_dictionary();
            // Открываем исходный файл и подключаем его к буферу
            let mut reader = BufReader::new(reader);
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
                if !self.dictionary.contains(&prev) {
                    // Добавляем P в буфер
                    self.append_to_buf(&mut bit_buf, prev[0..prev.len() - 1].to_vec());
                    // Меняем номер последнего ключа в словаре
                    self.add_element_count();
                    // P + C в словарь
                    self.dictionary.insert(prev.clone());
                    // P = C
                    prev.clear();
                    prev.push(current);
                    //Проверяем, может ли добавить что-то в файл
                    while let Some(byte) = pop_byte(&mut bit_buf) {
                        writer.write_all(&[byte])?;
                    }
                }
            }
            // Добавляем в буфер оставшиеся байты
            self.append_to_buf(&mut bit_buf, prev);
            let last_bytes: Vec<u8> = bit_buf.as_slice().iter().cloned().collect();
            // Добавляем в файл последние байты, дополняя их нулями
            writer.write_all(&last_bytes)?;
            Ok(())
        }
        /// Добавляем в буфер кодовое значение из словаря, для дальнейшего добавления в файл
        fn append_to_buf(&self, bit_buf: &mut BitVec<BigEndian, u8>, value: Vec<u8>) {
            let (index, _) = self.dictionary.get_full(&value).expect(
                "Ошибка при получении значения из словаря",
            );
            bit_buf.append(&mut from_index(index as Index, self.bits_count));
        }
        // Увеличиваем счетчик словаря
        fn add_element_count(&mut self) -> bool {
            let bits_count = get_bits_count(self.dictionary.len() as Index) as u8;
            // Сбрасываем словарь, если достигли максимального количества бит
            if self.dictionary.len() + 1 == (1 << self.max_bits_count) as usize {
                self.reset_compress_dictionary();
                true
            } else {
                self.bits_count = bits_count;
                false
            }
        }
        fn reset_compress_dictionary(&mut self) {
            // Инициализируем словарь из всех значений, которые можно хранить
            // в одном байте (0..255)
            self.dictionary.clear();
            for ch in u8::min_value()..=u8::max_value() {
                self.dictionary.insert(vec![ch]);
            }
            self.bits_count = 8;
        }
    }
    impl Decompress {
        /// Инициализируем структуру начальными значениями
        fn new(max_bits_count: u8) -> Decompress {
            if max_bits_count > 32 || max_bits_count < 9 {
                panic!("Недопустимый размер словаря! Разрешенный: 9 <= n <= 32");
            }
            Decompress {
                dictionary: Vec::new(),
                max_bits_count,
            }
        }
        fn decompress<R: Read, W: Write>(
            &mut self,
            reader: R,
            writer: &mut W,
        ) -> std::io::Result<()> {
            // Задаем начальный словарь
            self.reset_decompress_dictionary();
            // Открываем исходный файл и подключаем его к буферу
            let mut reader = BufReader::new(reader);
            // Буфер для считываемого байта
            let mut buf = [0u8; 1];
            // Буфер из бит, для добавления в результирующий поток
            let mut bit_buf: BitVec<BigEndian, u8> = BitVec::with_capacity(64);

            // Текущий считанный индекс кодового слова
            let mut index: usize;
            // Прошлое кодовое слово
            let mut string: Vec<u8> = Vec::new();
            // Количество бит для первого числа
            let mut bits_count = get_bits_count((self.dictionary.len() - 1) as Index);
            // Основной цикл алгоритма
            loop {
                // Считываем из буфера по байту, пока не достигнем нужного,
                // для извлечения индекса, количества бит
                while bit_buf.len() < bits_count {
                    if reader.read(&mut buf)? != buf.len() {
                        // Если встретили конец файла, завершаем работу алгоритма
                        return Ok(());
                    }
                    // Добавляем байт в буфер
                    bit_buf.append(&mut from_index(u32::from(buf[0]), 8));
                }
                // Извлекаем индекс
                let index_tmp: Index = pop_first_bits(&mut bit_buf, bits_count as u8).expect(
                    "Ошибка в извлечении индекса из битового буфера"
                );
                // Меняем тип к usize, чтобы индексировать вектор
                index = index_tmp as usize;
                // Если индекс больше размера массива, значит файл некорректен
                if index > self.dictionary.len() {
                    panic!("Неверный зашифрованный код");
                // Если индекс равен размеру словаря, то кодового слова нет, добавим в словарь
                } else if index == self.dictionary.len() {
                    string.push(string[0]);
                // Если элемент с заданным индексом есть в словаре
                } else if !string.is_empty() {
                    string.push(self.dictionary[index][0]);
                }
                // Добавление в словарь
                if !string.is_empty() {
                    self.dictionary.push(string);
                }
                let code = self.dictionary.get(index).expect(
                    "Ошибка в извлечении кодового слова из словаря"
                );
                // Записываем в файл
                writer.write_all(&code[..])?;
                string = code.to_vec();

                // Сбрасываем словарь, если наполнили его
                if self.dictionary.len() + 1 == 1 << self.max_bits_count as usize {
                    self.reset_decompress_dictionary();
                    // Для первого считываемого байта, возьмем количество бит от размера словаря минус 1
                    bits_count = get_bits_count((self.dictionary.len() - 1) as Index);
                } else {
                    // Количество бит для считывания следующего индекса
                    bits_count = get_bits_count(self.dictionary.len() as Index);
                }
            }
        }
        fn reset_decompress_dictionary(&mut self) {
            // Инициализируем словарь из всех значений, которые можно хранить
            // в одном байте (0..255)
            self.dictionary.clear();
            for ch in u8::min_value()..=u8::max_value() {
                self.dictionary.push(vec![ch]);
            }
        }
    }
    /// Получает количество бит числа, без лидирующих нулей
    fn get_bits_count(length: Index) -> usize {
        let bits_in_type = Index::from(0u8).count_zeros();
        (bits_in_type - length.leading_zeros()) as usize
    }
    /// Преобразует value в BitVec длиной bits
    fn from_index(value: Index, bits: u8) -> BitVec<BigEndian, u8> {
        let mut bv: BitVec<BigEndian, u8> = BitVec::with_capacity(bits as usize);
        for i in (0..bits).rev() {
            // Добавляем i-ый бит в bv
            bv.push(((1 << i) & value) != 0);
        }
        bv
    }
    /// Получает из BitVec байты (u8) для записи в файл
    fn pop_byte(bv: &mut BitVec<BigEndian, u8>) -> Option<u8> {
        if let Some(byte) = pop_first_bits(bv, 8) {
            return Some(byte as u8);
        }
        None
    }
    /// Получает из BitVec число, состоящее из первых bits бит
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

    /// Запускает компрессию файла
    pub fn compress(
        source_file: &str,
        result_file: &str,
        max_bits_count: u8,
    ) -> std::io::Result<()> {
        let mut compress_struct = Compress::new(max_bits_count);
        let reader = File::open(source_file)?;
        let mut writer = File::create(result_file)?;
        compress_struct.compress(reader, &mut writer)?;
        Ok(())
    }
    /// Запускает декомпрессию файла
    pub fn decompress(
        source_file: &str,
        result_file: &str,
        max_bits_count: u8,
    ) -> std::io::Result<()> {
        let mut decompress_struct = Decompress::new(max_bits_count);
        let reader = File::open(source_file)?;
        let mut writer = File::create(result_file)?;
        decompress_struct.decompress(reader, &mut writer)?;
        Ok(())
    }
}
