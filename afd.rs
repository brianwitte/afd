#![no_std]
#![no_main]

use core::panic::PanicInfo;

// Provide missing C functions that Rust might generate calls to
#[no_mangle]
pub unsafe extern "C" fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = c as u8;
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if (dest as usize) < (src as usize) {
        let mut i = 0;
        while i < n {
            *dest.add(i) = *src.add(i);
            i += 1;
        }
    } else {
        let mut i = n;
        while i > 0 {
            i -= 1;
            *dest.add(i) = *src.add(i);
        }
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return a as i32 - b as i32;
        }
        i += 1;
    }
    0
}

// Basic syscall interface for Linux x86_64
#[cfg(target_arch = "x86_64")]
mod syscalls {
    pub const SYS_READ: usize = 0;
    pub const SYS_WRITE: usize = 1;
    pub const SYS_EXIT: usize = 60;
    pub const STDIN_FILENO: usize = 0;
    pub const STDOUT_FILENO: usize = 1;

    #[inline]
    pub unsafe fn syscall1(n: usize, a1: usize) -> isize {
        let ret: isize;
        core::arch::asm!(
            "syscall",
            in("rax") n,
            in("rdi") a1,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }

    #[inline]
    pub unsafe fn syscall3(n: usize, a1: usize, a2: usize, a3: usize) -> isize {
        let ret: isize;
        core::arch::asm!(
            "syscall",
            in("rax") n,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }
}

// Simple print functions
fn print_str(s: &str) {
    unsafe {
        syscalls::syscall3(syscalls::SYS_WRITE, syscalls::STDOUT_FILENO, s.as_ptr() as usize, s.len());
    }
}

fn print_num(n: i32) {
    let mut buf = [0u8; 12];
    let mut i = buf.len();
    let mut num = n;
    let negative = num < 0;
    
    if negative {
        num = -num;
    }
    
    if num == 0 {
        print_str("0");
        return;
    }
  
    while num > 0 {
        i -= 1;
        buf[i] = (num % 10) as u8 + b'0';
        num /= 10;
    }
    
    if negative {
        i -= 1;
        buf[i] = b'-';
    }
    
    let s = unsafe { core::str::from_utf8_unchecked(&buf[i..]) };
    print_str(s);
}

fn read_char() -> Option<u8> {
    let mut buf = [0u8; 1];
    unsafe {
        let result = syscalls::syscall3(syscalls::SYS_READ, syscalls::STDIN_FILENO, buf.as_mut_ptr() as usize, 1);
        if result == 1 {
            Some(buf[0])
        } else {
            None
        }
    }
}

// Forth interpreter structures - reduced sizes
const STACK_SIZE: usize = 64;
const INPUT_BUFFER_SIZE: usize = 256;
const WORD_BUFFER_SIZE: usize = 32;
const DICTIONARY_SIZE: usize = 32;
const MAX_WORD_NAME_LEN: usize = 16;
const USER_WORDS_SIZE: usize = 1024;

struct ForthStack {
    data: [i32; STACK_SIZE],
    top: usize,
}

impl ForthStack {
    fn new() -> Self {
        Self {
            data: [0; STACK_SIZE],
            top: 0,
        }
    }
    
    fn push(&mut self, value: i32) -> Result<(), &'static str> {
        if self.top >= STACK_SIZE {
            return Err("Stack overflow");
        }
        self.data[self.top] = value;
        self.top += 1;
        Ok(())
    }
    
    fn pop(&mut self) -> Result<i32, &'static str> {
        if self.top == 0 {
            return Err("Stack underflow");
        }
        self.top -= 1;
        Ok(self.data[self.top])
    }
    
    fn peek(&self) -> Result<i32, &'static str> {
        if self.top == 0 {
            return Err("Stack empty");
        }
        Ok(self.data[self.top - 1])
    }
    
    fn size(&self) -> usize {
        self.top
    }
}

#[derive(Clone, Copy)]
struct DictionaryEntry {
    name: [u8; MAX_WORD_NAME_LEN],
    name_len: usize,
    is_builtin: bool,
    builtin_id: u8,
    user_word_start: usize,
    user_word_len: usize,
}

impl DictionaryEntry {
    fn new() -> Self {
        Self {
            name: [0; MAX_WORD_NAME_LEN],
            name_len: 0,
            is_builtin: true,
            builtin_id: 0,
            user_word_start: 0,
            user_word_len: 0,
        }
    }
}

// Built-in word IDs
const BUILTIN_ADD: u8 = 1;
const BUILTIN_SUB: u8 = 2;
const BUILTIN_MUL: u8 = 3;
const BUILTIN_DIV: u8 = 4;
const BUILTIN_MOD: u8 = 5;
const BUILTIN_DUP: u8 = 6;
const BUILTIN_DROP: u8 = 7;
const BUILTIN_SWAP: u8 = 8;
const BUILTIN_OVER: u8 = 9;
const BUILTIN_ROT: u8 = 10;
const BUILTIN_DOT: u8 = 11;
const BUILTIN_DOTS: u8 = 12;
const BUILTIN_CR: u8 = 13;
const BUILTIN_BYE: u8 = 14;
const BUILTIN_COLON: u8 = 15;
const BUILTIN_SEMICOLON: u8 = 16;
const BUILTIN_EQUAL: u8 = 17;
const BUILTIN_LESS: u8 = 18;
const BUILTIN_GREATER: u8 = 19;
const BUILTIN_WORDS: u8 = 20;

struct ForthInterpreter {
    stack: ForthStack,
    input_buffer: [u8; INPUT_BUFFER_SIZE],
    word_buffer: [u8; WORD_BUFFER_SIZE],
    dictionary: [DictionaryEntry; DICTIONARY_SIZE],
    dict_top: usize,
    user_words: [u8; USER_WORDS_SIZE],
    user_words_top: usize,
    compiling: bool,
    current_def_start: usize,
}

impl ForthInterpreter {
    fn new() -> Self {
        Self {
            stack: ForthStack::new(),
            input_buffer: [0; INPUT_BUFFER_SIZE],
            word_buffer: [0; WORD_BUFFER_SIZE],
            dictionary: [DictionaryEntry::new(); DICTIONARY_SIZE],
            dict_top: 0,
            user_words: [0; USER_WORDS_SIZE],
            user_words_top: 0,
            compiling: false,
            current_def_start: 0,
        }
    }
    
    fn init_builtins(&mut self) {
        self.add_builtin(b"+", BUILTIN_ADD);
        self.add_builtin(b"-", BUILTIN_SUB);
        self.add_builtin(b"*", BUILTIN_MUL);
        self.add_builtin(b"/", BUILTIN_DIV);
        self.add_builtin(b"mod", BUILTIN_MOD);
        self.add_builtin(b"dup", BUILTIN_DUP);
        self.add_builtin(b"drop", BUILTIN_DROP);
        self.add_builtin(b"swap", BUILTIN_SWAP);
        self.add_builtin(b"over", BUILTIN_OVER);
        self.add_builtin(b"rot", BUILTIN_ROT);
        self.add_builtin(b".", BUILTIN_DOT);
        self.add_builtin(b".s", BUILTIN_DOTS);
        self.add_builtin(b"cr", BUILTIN_CR);
        self.add_builtin(b"bye", BUILTIN_BYE);
        self.add_builtin(b":", BUILTIN_COLON);
        self.add_builtin(b";", BUILTIN_SEMICOLON);
        self.add_builtin(b"=", BUILTIN_EQUAL);
        self.add_builtin(b"<", BUILTIN_LESS);
        self.add_builtin(b">", BUILTIN_GREATER);
        self.add_builtin(b"words", BUILTIN_WORDS);
    }
    
    fn add_builtin(&mut self, name: &[u8], id: u8) {
        if self.dict_top >= DICTIONARY_SIZE {
            return;
        }
        
        let entry = &mut self.dictionary[self.dict_top];
        let len = core::cmp::min(name.len(), MAX_WORD_NAME_LEN);
        
        for i in 0..len {
            entry.name[i] = name[i];
        }
        entry.name_len = len;
        entry.is_builtin = true;
        entry.builtin_id = id;
        
        self.dict_top += 1;
    }
    
    fn execute_builtin(&mut self, id: u8) -> Result<bool, &'static str> {
        match id {
            BUILTIN_ADD => {
                let b = self.stack.pop()?;
                let a = self.stack.pop()?;
                self.stack.push(a + b)?;
            }
            BUILTIN_SUB => {
                let b = self.stack.pop()?;
                let a = self.stack.pop()?;
                self.stack.push(a - b)?;
            }
            BUILTIN_MUL => {
                let b = self.stack.pop()?;
                let a = self.stack.pop()?;
                self.stack.push(a * b)?;
            }
            BUILTIN_DIV => {
                let b = self.stack.pop()?;
                let a = self.stack.pop()?;
                if b == 0 {
                    return Err("Division by zero");
                }
                self.stack.push(a / b)?;
            }
            BUILTIN_MOD => {
                let b = self.stack.pop()?;
                let a = self.stack.pop()?;
                if b == 0 {
                    return Err("Division by zero");
                }
                self.stack.push(a % b)?;
            }
            BUILTIN_DUP => {
                let a = self.stack.peek()?;
                self.stack.push(a)?;
            }
            BUILTIN_DROP => {
                self.stack.pop()?;
            }
            BUILTIN_SWAP => {
                let b = self.stack.pop()?;
                let a = self.stack.pop()?;
                self.stack.push(b)?;
                self.stack.push(a)?;
            }
            BUILTIN_OVER => {
                let b = self.stack.pop()?;
                let a = self.stack.pop()?;
                self.stack.push(a)?;
                self.stack.push(b)?;
                self.stack.push(a)?;
            }
            BUILTIN_ROT => {
                let c = self.stack.pop()?;
                let b = self.stack.pop()?;
                let a = self.stack.pop()?;
                self.stack.push(b)?;
                self.stack.push(c)?;
                self.stack.push(a)?;
            }
            BUILTIN_DOT => {
                let value = self.stack.pop()?;
                print_num(value);
                print_str(" ");
            }
            BUILTIN_DOTS => {
                print_str("<");
                print_num(self.stack.size() as i32);
                print_str("> ");
                for i in 0..self.stack.size() {
                    print_num(self.stack.data[i]);
                    print_str(" ");
                }
            }
            BUILTIN_CR => {
                print_str("\n");
            }
            BUILTIN_BYE => {
                print_str("Goodbye!\n");
                return Ok(true);
            }
            BUILTIN_COLON => {
                if self.compiling {
                    return Err("Already compiling");
                }
                self.compiling = true;
                self.current_def_start = self.user_words_top;
            }
            BUILTIN_SEMICOLON => {
                if !self.compiling {
                    return Err("Not compiling");
                }
                self.compiling = false;
            }
            BUILTIN_EQUAL => {
                let b = self.stack.pop()?;
                let a = self.stack.pop()?;
                self.stack.push(if a == b { -1 } else { 0 })?;
            }
            BUILTIN_LESS => {
                let b = self.stack.pop()?;
                let a = self.stack.pop()?;
                self.stack.push(if a < b { -1 } else { 0 })?;
            }
            BUILTIN_GREATER => {
                let b = self.stack.pop()?;
                let a = self.stack.pop()?;
                self.stack.push(if a > b { -1 } else { 0 })?;
            }
            BUILTIN_WORDS => {
                print_str("Words: ");
                for i in 0..self.dict_top {
                    let entry = &self.dictionary[i];
                    let name_slice = &entry.name[..entry.name_len];
                    let name_str = unsafe { core::str::from_utf8_unchecked(name_slice) };
                    print_str(name_str);
                    print_str(" ");
                }
                print_str("\n");
            }
            _ => return Err("Unknown builtin"),
        }
        Ok(false)
    }
    
    fn read_line(&mut self) -> bool {
        let mut pos = 0;
        
        while pos < INPUT_BUFFER_SIZE - 1 {
            if let Some(ch) = read_char() {
                if ch == b'\n' || ch == b'\r' {
                    break;
                }
                self.input_buffer[pos] = ch;
                pos += 1;
            } else {
                return false;
            }
        }
        
        self.input_buffer[pos] = 0;
        
        // Null terminate the rest of the buffer
        for i in (pos + 1)..INPUT_BUFFER_SIZE {
            self.input_buffer[i] = 0;
        }
        
        true
    }
    
    fn parse_number(word: &[u8]) -> Option<i32> {
        if word.is_empty() {
            return None;
        }
        
        let mut result = 0i32;
        let mut negative = false;
        let mut start = 0;
        
        if word[0] == b'-' {
            negative = true;
            start = 1;
            if word.len() == 1 {
                return None;
            }
        }
        
        for &byte in &word[start..] {
            if byte < b'0' || byte > b'9' {
                return None;
            }
            result = result * 10 + (byte - b'0') as i32;
        }
        
        if negative {
            result = -result;
        }
        
        Some(result)
    }
    
    fn word_matches(word: &[u8], target: &[u8]) -> bool {
        if word.len() != target.len() {
            return false;
        }
        
        for i in 0..word.len() {
            if word[i] != target[i] {
                return false;
            }
        }
        
        true
    }
    
    fn find_word(&self, word: &[u8]) -> Option<usize> {
        for i in (0..self.dict_top).rev() {
            let entry = &self.dictionary[i];
            if Self::word_matches(word, &entry.name[..entry.name_len]) {
                return Some(i);
            }
        }
        None
    }
    
    fn execute_word(&mut self, word: &[u8]) -> Result<bool, &'static str> {
        // Try to parse as number first
        if let Some(num) = Self::parse_number(word) {
            self.stack.push(num)?;
            return Ok(false);
        }
        
        // Look up word in dictionary
        if let Some(index) = self.find_word(word) {
            let entry = self.dictionary[index];
            
            if entry.is_builtin {
                let should_exit = self.execute_builtin(entry.builtin_id)?;
                if should_exit {
                    return Ok(true);
                }
            }
        } else {
            if self.compiling {
                // This must be the name of the word being defined
                if self.dict_top >= DICTIONARY_SIZE {
                    return Err("Dictionary full");
                }
                
                let entry = &mut self.dictionary[self.dict_top];
                let len = core::cmp::min(word.len(), MAX_WORD_NAME_LEN);
                
                for i in 0..len {
                    entry.name[i] = word[i];
                }
                entry.name_len = len;
                entry.is_builtin = false;
                entry.user_word_start = self.current_def_start;
                entry.user_word_len = self.user_words_top - self.current_def_start;
                
                self.dict_top += 1;
            } else {
                print_str("Unknown word: ");
                let word_str = unsafe { core::str::from_utf8_unchecked(word) };
                print_str(word_str);
                print_str("\n");
                return Err("Unknown word");
            }
        }
        
        Ok(false)
    }
    
    fn process_line(&mut self) -> Result<bool, &'static str> {
        let mut pos = 0;
        
        loop {
            // Skip whitespace
            while pos < INPUT_BUFFER_SIZE && 
                  (self.input_buffer[pos] == b' ' || 
                   self.input_buffer[pos] == b'\t') {
                pos += 1;
            }
            
            // Check if we're at the end
            if pos >= INPUT_BUFFER_SIZE || self.input_buffer[pos] == 0 {
                break;
            }
            
            // Extract word into word_buffer
            let start = pos;
            while pos < INPUT_BUFFER_SIZE && 
                  self.input_buffer[pos] != 0 &&
                  self.input_buffer[pos] != b' ' && 
                  self.input_buffer[pos] != b'\t' {
                pos += 1;
            }
            
            if start < pos {
                let word_len = pos - start;
                if word_len >= WORD_BUFFER_SIZE {
                    return Err("Word too long");
                }
                
                // Copy word to word_buffer
                for i in 0..word_len {
                    self.word_buffer[i] = self.input_buffer[start + i];
                }
                
                // Create a local copy to avoid borrowing conflicts
                let mut word_copy = [0u8; WORD_BUFFER_SIZE];
                for i in 0..word_len {
                    word_copy[i] = self.word_buffer[i];
                }
                
                let should_exit = self.execute_word(&word_copy[..word_len])?;
                if should_exit {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    fn run(&mut self) {
        print_str("afd: Alien Forth Dialect v0.4\n");
        print_str("Type 'bye' to exit, '.s' to show stack, 'words' to list words\n");
        print_str("Available: + - * / mod dup drop swap over rot . .s cr bye\n");
        print_str("           = < > words : ;\n\n");
        
        loop {
            if self.compiling {
                print_str(": ");
            } else {
                print_str("ok> ");
            }
            
            if !self.read_line() {
                break;
            }
            
            match self.process_line() {
                Ok(should_exit) => {
                    if should_exit {
                        break;
                    }
                    if !self.compiling {
                        print_str("ok\n");
                    }
                },
                Err(err) => {
                    print_str("Error: ");
                    print_str(err);
                    print_str("\n");
                    if self.compiling {
                        print_str("Compilation aborted\n");
                        self.compiling = false;
                        self.user_words_top = self.current_def_start;
                    }
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut interpreter = ForthInterpreter::new();
    interpreter.init_builtins();
    interpreter.run();
    
    unsafe {
        syscalls::syscall1(syscalls::SYS_EXIT, 0);
    }
    
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    print_str("\nPanic occurred!\n");
    unsafe {
        syscalls::syscall1(syscalls::SYS_EXIT, 1);
    }
    loop {}
}
