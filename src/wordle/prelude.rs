pub const WORD_SIZE: usize = 5;
pub const NUM_TURNS: usize = 6;
pub const ALPHABET_SIZE: usize = (('z' as usize) - ('a' as usize)) + 1;

pub type WordleFloat = f64;

pub const MIN_WORD_PROBABILITY: WordleFloat = 0.0001;

pub use crate::util::*;

pub fn count_letters(word: &str) -> [usize; ALPHABET_SIZE] {
    count_letters_bytes(word.as_bytes())
}

pub fn count_letters_bytes(word: &[u8]) -> [usize; ALPHABET_SIZE] {
    debug_assert!(is_wordle_str_bytes(word));
    let mut out = [0; ALPHABET_SIZE];
    for i in 0..WORD_SIZE {
        out[letter_idx(word[i])] += 1;
    }

    out
}

pub fn letter_idx(letter: u8) -> usize {
    ((letter as isize) - ('a' as isize)) as usize
}

pub fn is_wordle_str(v: &str) -> bool {
    is_wordle_str_bytes(v.as_bytes())
}

pub fn is_wordle_str_bytes(v: &[u8]) -> bool {
    v.len() == WORD_SIZE && v.iter().all(|v| is_normal_wordle_char(v))
}

#[inline]
pub fn sigmoid(v: WordleFloat) -> WordleFloat {
    (1.0 + (-v).exp()).recip()
}

pub fn normalize_wordle_word(str: &str) -> String {
    str.trim().to_lowercase()
}

pub fn is_normal_wordle_char(v: &u8) -> bool {
    v.is_ascii_lowercase()
}