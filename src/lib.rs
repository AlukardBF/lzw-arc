// #![feature(async_await, await_macro, futures_api)]
pub mod lzw {
    use bitvec::{BigEndian, BitVec};
    use indexmap::IndexSet;
    use small_aes_rs::{AesCtx, Block, AES_BLOCKLEN};
    use std::fs::File;
    use std::io::{BufReader, BufWriter, Read, Write};
    type Index = u32;

    // Модуль генерации, проверки ключа шифрования
    mod derive {
        use ring::{digest, pbkdf2};
        use small_aes_rs::AES_KEYLEN;
        use std::num::NonZeroU32;

        const KEY_LEN: usize = AES_KEYLEN;
        type CypherKey = [u8; KEY_LEN];
        // Алгоритм генерации псевдо-случайных чисел
        static DIGEST_ALG: &'static digest::Algorithm = &digest::SHA256;
        // Соль
        const SALT: CypherKey = [
            0xd6, 0x26, 0x98, 0xda, 0xf4, 0xdc, 0x50, 0x52, 0x24, 0xf2, 0x27, 0xd1, 0xfe, 0x39,
            0x01, 0x8a,
        ];
        pub fn derive_key(secret: &str) -> CypherKey {
            // Ключ
            let mut key = [0u8; KEY_LEN];
            // Количество итераций
            let iterations = NonZeroU32::new(100_000).unwrap();
            // Генерируем ключ
            pbkdf2::derive(DIGEST_ALG, iterations, &SALT, secret.as_bytes(), &mut key);
            key
        }
    }

    struct Compress {
        // Словарь, для архивации
        dictionary: IndexSet<Vec<u8>>,
        // Текущее количество бит в максимальном значении словаря
        bits_count: u8,
        // Максимальное количество бит, т.е. размер словаря
        max_bits_count: u8,
        // Предыдущая строка
        prev: Vec<u8>,
        // Буфер из бит, для добавления в результирующий поток
        bit_buf: BitVec<BigEndian, u8>,
    }
    struct Decompress {
        // Словарь, для архивации
        dictionary: Vec<Vec<u8>>,
        bits_count: usize,
        // Максимальное количество бит, т.е. размер словаря
        max_bits_count: u8,
        // Текущий считанный индекс кодового слова
        index: usize,
        // Прошлое кодовое слово
        string: Vec<u8>,
        // Буфер из бит, для добавления в результирующий поток
        bit_buf: BitVec<BigEndian, u8>,
    }
    impl Default for Compress {
        fn default() -> Compress {
            // Инициализируем словарь из всех значений, которые можно хранить
            // в одном байте (0..255)
            // Выделяем памяти в словаре под 65536 значений (для размера словаря по-умолчанию в 16 бит)
            let mut dictionary: IndexSet<Vec<u8>> =
                IndexSet::with_capacity(u16::max_value() as usize);
            for ch in u8::min_value()..=u8::max_value() {
                dictionary.insert(vec![ch]);
            }
            Compress {
                dictionary,
                bits_count: 8,
                max_bits_count: 16,
                prev: Vec::with_capacity(64),
                bit_buf: BitVec::with_capacity(32),
            }
        }
    }
    impl Default for Decompress {
        fn default() -> Decompress {
            // Инициализируем словарь из всех значений, которые можно хранить
            // в одном байте (0..255)
            // Выделяем памяти в словаре под 65536 значений (для размера словаря по-умолчанию в 16 бит)
            let mut dictionary: Vec<Vec<u8>> = Vec::with_capacity(u16::max_value() as usize);
            for ch in u8::min_value()..=u8::max_value() {
                dictionary.push(vec![ch]);
            }
            Decompress {
                dictionary,
                bits_count: 8,
                max_bits_count: 16,
                index: 0,
                string: Vec::new(),
                bit_buf: BitVec::with_capacity(64),
            }
        }
    }
    impl Compress {
        fn new(max_bits_count: u8) -> Self {
            if max_bits_count > 32 || max_bits_count < 9 {
                panic!("Недопустимый размер словаря! Разрешенный: 9 <= n <= 32");
            }
            Self {
                max_bits_count,
                ..Default::default()
            }
        }
        fn compress<R: Read, W: Write>(
            &mut self,
            reader: R,
            writer: &mut W,
        ) -> std::io::Result<()> {
            // Создаем буфер для reader, что ускорит чтение
            let mut reader = BufReader::new(reader);
            // Создаем буфер для writer, что ускорит запись
            let mut writer = BufWriter::new(writer);
            // Буфер для считываемого байта
            let mut buf = [0u8; 1];
            // Основной цикл алгоритма. Считываем по одному байту, пока не закончится файл
            while reader.read(&mut buf)? != 0 {
                // Текущий символ
                let current: u8 = buf[0];
                self.prev.push(current);
                // Набор байт уже присутствует в словаре?
                if !self.dictionary.contains(&self.prev) {
                    // Добавляем P в буфер
                    self.append_to_buf(self.prev[0..self.prev.len() - 1].to_vec());
                    // Меняем номер последнего ключа в словаре
                    self.add_element_count();
                    // P + C в словарь
                    self.dictionary.insert(self.prev.clone());
                    // P = C
                    self.prev.clear();
                    self.prev.push(current);
                    while let Some(byte) = pop_byte(&mut self.bit_buf) {
                        writer.write_all(&[byte])?;
                    }
                }
            }
            Ok(())
        }
        /// Добавляет оставшиеся в буфере байты в заданный поток
        fn last_bytes<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
            // Добавляем в буфер оставшиеся байты
            self.append_to_buf(self.prev.to_vec());
            let last_bytes: Vec<u8> = self.bit_buf.as_slice().to_vec();
            // Добавляем в файл последние байты, дополняя их нулями
            writer.write_all(&last_bytes)?;
            Ok(())
        }
        /// Добавляем в буфер кодовое значение из словаря, для дальнейшего добавления в файл
        fn append_to_buf(&mut self, value: Vec<u8>) {
            let (index, _) = self.dictionary.get_full(&value).expect(
                "Ошибка при получении значения из словаря",
            );
            self.bit_buf
                .append(&mut from_index(index as Index, self.bits_count));
        }
        // Увеличиваем счетчик словаря
        fn add_element_count(&mut self) -> bool {
            let bits_count = get_bits_count(self.dictionary.len() as Index) as u8;
            // Сбрасываем словарь, если достигли максимального количества бит
            if self.dictionary.len() + 1 == (1 << self.max_bits_count) as usize {
                self.reset_dictionary();
                true
            } else {
                self.bits_count = bits_count;
                false
            }
        }
        fn reset_dictionary(&mut self) {
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
        fn new(max_bits_count: u8) -> Self {
            if max_bits_count > 32 || max_bits_count < 9 {
                panic!("Недопустимый размер словаря! Разрешенный: 9 <= n <= 32");
            }
            Self {
                max_bits_count,
                ..Default::default()
            }
        }
        fn decompress<R: Read, W: Write>(
            &mut self,
            reader: R,
            writer: &mut W,
        ) -> std::io::Result<()> {
            // Создаем буфер для reader, что ускорит чтение
            let mut reader = BufReader::new(reader);
            // Создаем буфер для writer, что ускорит запись
            let mut writer = BufWriter::new(writer);
            // Буфер для считываемого байта
            let mut buf = [0u8; 1];
            // Основной цикл алгоритма
            loop {
                // Считываем из буфера по байту, пока не достигнем нужного,
                // для извлечения индекса, количества бит
                while self.bit_buf.len() < self.bits_count {
                    if reader.read(&mut buf)? != buf.len() {
                        // Если встретили конец файла, завершаем работу алгоритма
                        return Ok(());
                    }
                    // Добавляем байт в буфер
                    self.bit_buf.append(&mut from_index(u32::from(buf[0]), 8));
                }
                // Извлекаем индекс
                let index_tmp: Index = pop_first_bits(&mut self.bit_buf, self.bits_count as u8).expect(
                    "Ошибка в извлечении индекса из битового буфера"
                );
                // Меняем тип к usize, чтобы индексировать вектор
                self.index = index_tmp as usize;
                // Если индекс больше размера массива, значит файл некорректен
                if self.index > self.dictionary.len() {
                    panic!("Неверный зашифрованный код");
                // Если индекс равен размеру словаря, то кодового слова нет, добавим в словарь
                } else if self.index == self.dictionary.len() {
                    self.string.push(self.string[0]);
                // Если элемент с заданным индексом есть в словаре
                } else if !self.string.is_empty() {
                    self.string.push(self.dictionary[self.index][0]);
                }
                // Добавление в словарь
                if !self.string.is_empty() {
                    self.dictionary.push(self.string.clone());
                }
                let code = self.dictionary.get(self.index).expect(
                    "Ошибка в извлечении кодового слова из словаря"
                );
                // Записываем в файл
                writer.write_all(&code[..])?;
                self.string = code.to_vec();
                // Сбрасываем словарь, если наполнили его
                if self.dictionary.len() + 1 == 1 << self.max_bits_count as usize {
                    self.reset_dictionary();
                    // Для первого считываемого байта, возьмем количество бит от размера словаря минус 1
                    self.bits_count = get_bits_count((self.dictionary.len() - 1) as Index);
                } else {
                    // Количество бит для считывания следующего индекса
                    self.bits_count = get_bits_count(self.dictionary.len() as Index);
                }
            }
        }
        fn reset_dictionary(&mut self) {
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
        max_bits_count: usize,
    ) -> std::io::Result<()> {
        let mut lzw_struct = Compress::new(max_bits_count as u8);
        let reader = File::open(source_file)?;
        let mut writer = File::create(result_file)?;
        // Сжимаем
        lzw_struct.compress(reader, &mut writer)?;
        // Обязательно вызываем last_bytes, переносим внутренний буфер в поток
        lzw_struct.last_bytes(&mut writer)?;
        Ok(())
    }
    /// Запускает декомпрессию файла
    pub fn decompress(
        source_file: &str,
        result_file: &str,
        max_bits_count: usize,
    ) -> std::io::Result<()> {
        let mut lzw_struct = Decompress::new(max_bits_count as u8);
        let reader = File::open(source_file)?;
        let mut writer = File::create(result_file)?;
        lzw_struct.decompress(reader, &mut writer)?;
        Ok(())
    }
    /* Компрессия и декомпрессия с AES шифрованием */
    /// Компрессия с применением AES шифрования
    pub fn compress_aes(
        source_file: &str,
        result_file: &str,
        max_bits_count: usize,
        secret: &str,
    ) -> std::io::Result<()> {
        // Инициализируем объекты
        let mut lzw_struct = Compress::new(max_bits_count as u8);
        let mut reader = BufReader::new(File::open(source_file)?);
        let mut writer = File::create(result_file)?;
        // Промежуточный буфер для чтения
        let mut buf_read: Vec<u8> = vec![0u8; AES_BLOCKLEN];
        // Промежуточный буфер для записи
        // По наблюдением, не нужен более чем 50 байт
        let mut buf_write: Vec<u8> = Vec::with_capacity(50);
        // Объекты шифрования
        let key = derive::derive_key(secret);
        let iv: Block = rand::random();
        // Инициализируем AES ключом и IV
        let mut aes = AesCtx::with_iv(key, iv);
        // Пишем вектор инициализации в файл
        writer.write_all(&iv)?;
        // Цикл компрессии с шифрованием
        loop {
            let bytes_read = reader.read(&mut buf_read)?;
            if bytes_read == 0 {
                break;
            }
            buf_read.truncate(bytes_read);
            lzw_struct.compress(buf_read.as_slice(), &mut buf_write)?;
            // Если в буфере набралось 128 бит (16 байт) для шифрования
            while buf_write.len() >= AES_BLOCKLEN {
                let buf_aes: Vec<u8> = buf_write.drain(0..AES_BLOCKLEN).collect();
                aes.aes_cbc_encrypt_buffer(buf_aes.as_slice(), &mut writer)?;
            }
        }
        // Получаем/шифруем остаток байт
        lzw_struct.last_bytes(&mut buf_write)?;
        aes.aes_cbc_encrypt_buffer(buf_write.as_slice(), &mut writer)?;
        Ok(())
    }
    /// Декомпрессия с применением AES шифрования
    pub fn decompress_aes(
        source_file: &str,
        result_file: &str,
        max_bits_count: usize,
        secret: &str,
    ) -> std::io::Result<()> {
        // Инициализируем объекты
        let mut lzw_struct = Decompress::new(max_bits_count as u8);
        let mut reader = BufReader::new(File::open(source_file)?);
        let mut writer = File::create(result_file)?;
        // Промежуточный буфер для чтения
        let mut buf_read: Vec<u8> = vec![0u8; AES_BLOCKLEN];
        // Промежуточный буфер для записи
        let mut buf_write: Vec<u8> = Vec::with_capacity(AES_BLOCKLEN);
        // Считываем вектор инициализации из файла
        let mut iv: Block = Default::default();
        reader.read_exact(&mut iv)?;

        // Получаем 128-битный ключ
        let key = derive::derive_key(secret);
        // Инициализируем AES ключом и IV
        let mut aes = AesCtx::with_iv(key, iv);
        // Читаем первый блок
        if reader.read(&mut buf_read)? != AES_BLOCKLEN {
            panic!("Файл поврежден!");
        }
        // Цикл декомпрессии с расшифровкой
        while {
            // Отправляем блок на расшифровку
            aes.aes_cbc_decrypt_buffer(buf_read.as_slice(), &mut buf_write)?;
            // Читаем очередной  блок
            let bytes_read = reader.read(&mut buf_read)?;
            // Если достигли конца файла (EOF)
            if bytes_read == 0 {
                // То значит, что это последний блок
                while buf_write.last().unwrap() == &0u8 {
                    // Удаляем замыкающие нули (в соответствии с "Zero padding")
                    buf_write.pop();
                }
            }
            // Распаковываем блок
            lzw_struct.decompress(buf_write.as_slice(), &mut writer)?;
            buf_write.clear();
            // Если что-то считано - продолжаем работу
            bytes_read != 0
        } {}
        Ok(())
    }
}
