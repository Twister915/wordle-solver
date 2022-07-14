/*
 * MIT License
 *
 * Copyright (c) 2022 Joseph Sacchini
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use std::fmt::{Debug, Display, Formatter};
#[cfg(test)]
use std::iter::FusedIterator;
use std::ops::{Index, IndexMut};
use self::Coloring::*;
use super::prelude::*;

///
/// Any set of colorings can be converted to a "code" which uniquely identifies that specific
/// coloring. This type is the number we use to store that code (and we pick u8 because the range is
/// 0 -> 3^5=243 for 3 colorings in a 5 letter puzzle).
///
pub type ColoringCode = u8;

///
/// The three different colors that a wordle square can be...
///   * Excluded = the letter is not in the answer (also indicates no further instances of a letter
///                when another square with the same letter is colored misplaced/correct)
///   * Misplaced = the letter is in the answer, but not in this position
///   * Correct = the letter is in the answer at this position
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Coloring {
    Excluded,
    Misplaced,
    Correct,
}

impl Coloring {
    /// All three colorings (make sure this actually matches the definition above)
    pub const ALL: [Coloring; 3] = [Excluded, Misplaced, Correct];
    /// The number of possible colorings
    pub const NUM: usize = Self::ALL.len();

    /// Converts the coloring to a number (0, 1, or 2)
    pub fn ordinal(&self) -> ColoringCode {
        use Coloring::*;
        match self {
            Excluded => 0,
            Misplaced => 1,
            Correct => 2,
        }
    }

    /// Converts a number (usually from .ordinal()) back to a Coloring
    pub fn from_ordinal(code: ColoringCode) -> Option<Self> {
        use Coloring::*;
        Some(match code {
            0 => Excluded,
            1 => Misplaced,
            2 => Correct,
            _ => return None,
        })
    }

    /// Gives the best emoji to represent the coloring (used for debug printing)
    pub fn emoji(&self) -> &'static str {
        use Coloring::*;
        match self {
            Excluded => "⬛",
            Misplaced => "🟨",
            Correct => "🟩"
        }
    }
}

/// An array of Colorings, one for each square in the puzzle.
pub type ColoringsArray = [Coloring; WORD_SIZE];

/// The array of Colorings, but in a struct, so that we can attach some useful functions to a
/// complete set of Colorings.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Colorings(pub ColoringsArray);

/// Automatic conversion of [Coloring; WORD_SIZE] -> Colorings
impl From<ColoringsArray> for Colorings {
    fn from(arr: ColoringsArray) -> Self {
        Self(arr)
    }
}

/// Delegate indexing of the struct to it's inner value
impl Index<usize> for Colorings {
    type Output = Coloring;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

/// Delegate mutable indexing of the struct to it's inner value
impl IndexMut<usize> for Colorings {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Colorings {
    /// How many different possible colorings are there? In the case of a 5 word puzzle with 3
    /// colorings it's 3^5=243 possible colorings
    pub const NUM_STATES: usize = Coloring::NUM.pow(WORD_SIZE as u32);

    ///
    /// Compute what colors would be shown given some guess & answer. For example if the guess was
    /// "tares" and the answer was "scare" we should compute [Excluded, Misplaced, Misplaced, Misplaced, Misplaced]
    ///
    /// This is implemented by:
    /// * creating an empty [Coloring; WORD_SIZE] where all colors are defaulted to Misplaced
    /// * computing a "budget" for each letter in the alphabet (based on their frequency in the answer)
    /// * performing a "GREEN pass" which marks all correctly positioned letters (and updates the budget)
    /// * performing a "YELLOW pass" to mark all misplaced letters (based on the remaining budget for each letter)
    ///
    pub fn with_guess_answer(guess: &str, answer: &str) -> Self {
        assert!(is_wordle_str(answer));
        assert!(is_wordle_str(guess));

        let mut out = Self::default();
        let mut answer_letter_counts = count_letters(answer);
        let answer_bytes = answer.as_bytes();
        let guess_bytes = guess.as_bytes();

        // GREEN pass
        for i in 0..WORD_SIZE {
            let gc = guess_bytes[i];
            let ac = answer_bytes[i];

            if gc == ac {
                answer_letter_counts[letter_idx(gc)] -= 1;
                out[i] = Correct;
            }
        }

        // YELLOW pass
        for i in 0..WORD_SIZE {
            if out[i] != Correct {
                let gc = guess_bytes[i];
                let counter = &mut answer_letter_counts[letter_idx(gc)];
                if *counter > 0 {
                    *counter -= 1;
                    out[i] = Misplaced;
                }
            }
        }

        out
    }

    ///
    /// Computes a code that uniquely identifies this particular coloring. These codes are numbers in
    /// [0, 243) (in the case of a 5 letter puzzle).
    ///
    /// We essentially treat the colorings as a 5 digit base-3 number. Each Coloring has an ordinal()
    /// which ranges from [0, 3), and the left-most color is digit 0, next digit 1, etc.
    ///
    /// This is useful because in the Solver we want one bucket for each possible coloring, and
    /// using this to_code() we can convert a Coloring to an array index. The alternative (using a
    /// HashMap<Colorings, _>) requires implementing and calculating a Hash, allocating on the heap,
    /// etc. We avoid this and stay on the stack using static sized arrays indexed by Colorings.to_code()
    ///
    pub fn to_code(&self) -> ColoringCode {
        let mut out = 0;
        let mut multiplier = 1;
        for i in 0..WORD_SIZE {
            out += self[i].ordinal() * multiplier;
            multiplier *= Coloring::NUM as u8;
        }
        out
    }

    ///
    /// Converts a ColoringCode back to Colorings.
    ///
    /// This works by treating the code as a base-3 number, and the code is basically identical to
    /// any digit-by-digit processing you've written before.
    ///
    pub fn from_code(mut code: ColoringCode) -> Option<Self> {
        let mut out = Self::default();
        for i in 0..WORD_SIZE {
            out[i] = Coloring::from_ordinal(code % (Coloring::NUM as u8))?;
            code /= Coloring::NUM as u8;
        }

        Some(out)
    }

    #[cfg(test)]
    /// Iterates through all possible [Coloring; 5] configurations
    fn iter_all_possible() -> IterAllColorings {
        IterAllColorings::default()
    }
}

impl Default for Colorings {
    fn default() -> Self {
        Self([Excluded; WORD_SIZE])
    }
}

impl Display for Colorings {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for i in 0..WORD_SIZE {
            f.write_str(self[i].emoji())?;
        }

        Ok(())
    }
}

#[cfg(test)]
struct IterAllColorings {
    next: Option<Colorings>,
}

#[cfg(test)]
impl Default for IterAllColorings {
    fn default() -> Self {
        Self { next: Some(Colorings::default()) }
    }
}

#[cfg(test)]
impl Iterator for IterAllColorings {
    type Item = Colorings;

    fn next(&mut self) -> Option<Self::Item> {
        return if let Some(cur) = self.next {
            let mut next = cur;
            // basically... try to flip the right-most color through these three values:
            //  excluded -> misplaced -> correct
            // but if the right-most color is already "correct" then we reset it back to "excluded"
            // and try to perform the same operation on the next color (to the left).
            //
            // This results in a pattern like:
            // [Excluded, Excluded, Excluded, Excluded, Excluded]
            // [Excluded, Excluded, Excluded, Excluded, Misplaced]
            // [Excluded, Excluded, Excluded, Excluded, Correct]
            // [Excluded, Excluded, Excluded, Misplaced, Excluded]
            // [Excluded, Excluded, Excluded, Misplaced, Misplaced]
            // [Excluded, Excluded, Excluded, Misplaced, Correct]
            // [Excluded, Excluded, Excluded, Correct, Excluded]
            // [Excluded, Excluded, Excluded, Correct, Misplaced]
            // [Excluded, Excluded, Excluded, Correct, Correct]
            // [Excluded, Excluded, Misplaced, Excluded, Excluded]
            // ...
            //
            // which will eventually exhaust all possible colorings
            for k in (0..WORD_SIZE).rev() {
                match next[k] {
                    Excluded => {
                        next[k] = Misplaced;
                        self.next = Some(next);
                        break;
                    }
                    Misplaced => {
                        next[k] = Correct;
                        self.next = Some(next);
                        break;
                    }
                    Correct => {
                        if k == 0 {
                            self.next = None;
                            // implicitly this is break; because 0 is the end
                        } else {
                            next[k] = Misplaced;
                        }
                    }
                }
            }

            Some(cur)
        } else {
            None
        };
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (Colorings::NUM_STATES, Some(Colorings::NUM_STATES))
    }
}

#[cfg(test)]
impl ExactSizeIterator for IterAllColorings {}

#[cfg(test)]
impl FusedIterator for IterAllColorings {}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn test_coloring_ordinal_reversible() {
        for c in Coloring::ALL {
            assert_eq!(Some(c), Coloring::from_ordinal(c.ordinal()))
        }
    }

    #[test]
    fn test_colorings_fit_into_code_type() {
        let num_states = Colorings::NUM_STATES;
        let max_code_rep = ColoringCode::MAX as usize;
        assert!(
            num_states < max_code_rep,
            "{} states need to be represented by {} ({}..{})",
            num_states,
            std::any::type_name::<ColoringCode>(),
            ColoringCode::MIN,
            max_code_rep,
        );
    }

    #[test]
    fn test_unique_coding_of_colorings() {
        let mut seen = [false; ColoringCode::MAX as usize];
        for colorings in Colorings::iter_all_possible() {
            let code = colorings.to_code();
            assert!(!seen[code as usize], "expected no duplicate codes, got duplicate {}", code);
            seen[code as usize] = true;
        }
    }

    #[test]
    fn test_reversible_coding_of_colorings() {
        for colorings in Colorings::iter_all_possible() {
            let code = colorings.to_code();
            assert_eq!(
                Some(colorings),
                Colorings::from_code(code),
                "code {} produced from {:?} should reverse to same colorings",
                code,
                colorings,
            )
        }
    }

    #[test_case("zitis", "zizel", [Correct, Correct, Excluded, Excluded, Excluded])]
    #[test_case("tares", "scare", [Excluded, Misplaced, Misplaced, Misplaced, Misplaced])]
    #[test_case("spare", "scare", [Correct, Excluded, Correct, Correct, Correct])]
    #[test_case("share", "scare", [Correct, Excluded, Correct, Correct, Correct])]
    #[test_case("scare", "scare", [Correct, Correct, Correct, Correct, Correct])]
    #[test_case("tales", "apron", [Excluded, Misplaced, Excluded, Excluded, Excluded])]
    #[test_case("drain", "apron", [Excluded, Misplaced, Misplaced, Excluded, Correct])]
    #[test_case("roman", "apron", [Misplaced, Misplaced, Excluded, Misplaced, Correct])]
    #[test_case("apron", "apron", [Correct, Correct, Correct, Correct, Correct])]
    #[test_case("lanes", "legal", [Correct, Misplaced, Excluded, Misplaced, Excluded])]
    #[test_case("leary", "legal", [Correct, Correct, Misplaced, Excluded, Excluded])]
    #[test_case("lemma", "legal", [Correct, Correct, Excluded, Excluded, Misplaced])]
    #[test_case("legal", "legal", [Correct, Correct, Correct, Correct, Correct])]
    #[test_case("arles", "ledge", [Excluded, Excluded, Misplaced, Misplaced, Excluded])]
    #[test_case("elite", "ledge", [Misplaced, Misplaced, Excluded, Excluded, Correct])]
    #[test_case("ledge", "ledge", [Correct, Correct, Correct, Correct, Correct])]
    fn test_coloring(guess: &str, answer: &str, expected_coloring: ColoringsArray) {
        assert_eq!(
            Colorings::with_guess_answer(guess, answer),
            Colorings(expected_coloring),
            "guess={}, answer={}",
            guess,
            answer
        );
    }
}