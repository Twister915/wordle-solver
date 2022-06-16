// This file allows you to configure some of the constants that define the game of wordle.
//
// It is unlikely you will ever change the WORD_SIZE but if you want to, it should be supported by
// this implementation.


// how many characters are in a wordle answer?
pub const WORD_SIZE: usize = 5;
// how many turns are you allowed to play?
pub const NUM_TURNS: usize = 6;
// how many letters are in the english alphabet? (don't change this lol)
pub const ALPHABET_SIZE: usize = (('z' as usize) - ('a' as usize)) + 1;
// how many recommendations to store in the default state data. This is also used in the web app
// code to determine how many recommendations to display on the sidebar.
pub const N_RECOMMENDATIONS: usize = 32;

// This type allows you to switch between using f64 for all calculations and f32 if you so desire.
pub type WordleFloat = f64;

pub const MIN_WORD_WEIGHT: WordleFloat = 0.0001;

pub use crate::util::*;

/// Returns the number of times each letter of the alphabet occurs in the argument. The &str should
/// be in lowercase. The output is indexed by the position of the letter in the alphabet.
pub fn count_letters(word: &str) -> [usize; ALPHABET_SIZE] {
    count_letters_bytes(word.as_bytes())
}

/// Returns the number of times each letter of the alphabet occurs in the argument. The bytes should
/// represent only ASCII data (UTF-8 data is equiv for wordle words... all lowercase 5 letters)
/// The output is indexed by the position of the letter in the alphabet... like 'a' = 0,
/// 'b' = 1, etc...
pub fn count_letters_bytes(word: &[u8]) -> [usize; ALPHABET_SIZE] {
    debug_assert!(is_wordle_str_bytes(word));
    let mut out = [0; ALPHABET_SIZE];
    for i in 0..WORD_SIZE {
        out[letter_idx(word[i])] += 1;
    }

    out
}

/// Returns the index of the given letter within the alphabet (like 'a' = 0, 'b' = 1, etc...)
pub fn letter_idx(letter: u8) -> usize {
    ((letter as isize) - ('a' as isize)) as usize
}

/// Checks whether or not the passed string meets the constraints of a "wordle_str"
/// must be (5 letters, all lowercase)
pub fn is_wordle_str(v: &str) -> bool {
    is_wordle_str_bytes(v.as_bytes())
}

/// Checks whether or not the passed bytes represent an ASCII sequence which is also a "wordle_str"
pub fn is_wordle_str_bytes(v: &[u8]) -> bool {
    v.len() == WORD_SIZE && v.iter().all(is_normal_wordle_char)
}

#[inline]
pub fn sigmoid(v: WordleFloat) -> WordleFloat {
    (1.0 + (-v).exp()).recip()
}

/// Given some input &str, try to clean it up such that it might be a wordle_str.
///
/// This function does not trim the length of a word or remove non alpha characters. It simply
/// cleans up words that are already valid wordle words by removing any spacing and converting
/// to all lowercase.
///
/// You should always verify that the output of this function passes is_wordle_str.
pub fn normalize_wordle_word(str: &str) -> String {
    str.trim().to_lowercase()
}

/// Verifies that a byte represents a lowercase alphabetic character (a valid wordle_str char)
pub fn is_normal_wordle_char(v: &u8) -> bool {
    v.is_ascii_lowercase()
}