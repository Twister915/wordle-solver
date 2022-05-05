use std::fmt::{Debug, Display, Formatter};
use serde::{Serialize, Deserialize};
use std::ops::{Index, IndexMut};
use crate::wordle::color::Coloring::{Correct, Excluded, Misplaced};
use super::prelude::*;

pub type ColoringCode = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Coloring {
    Excluded,
    Misplaced,
    Correct,
}

impl Coloring {
    pub const ALL: [Coloring; 3] = [Excluded, Misplaced, Correct];
    pub const NUM: usize = Self::ALL.len();

    pub fn ordinal(&self) -> ColoringCode {
        use Coloring::*;
        match self {
            Excluded => 0,
            Misplaced => 1,
            Correct => 2,
        }
    }

    pub fn from_ordinal(code: ColoringCode) -> Option<Self> {
        use Coloring::*;
        Some(match code {
            0 => Excluded,
            1 => Misplaced,
            2 => Correct,
            _ => return None,
        })
    }

    pub fn emoji(&self) -> &'static str {
        use Coloring::*;
        match self {
            Excluded => "⬛",
            Misplaced => "🟨",
            Correct => "🟩"
        }
    }
}

pub type ColoringsArray = [Coloring; WORD_SIZE];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Colorings(pub ColoringsArray);

impl Into<Colorings> for ColoringsArray {
    fn into(self) -> Colorings {
        Colorings(self)
    }
}

impl Index<usize> for Colorings {
    type Output = Coloring;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for Colorings {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Colorings {
    pub const NUM_STATES: usize = Coloring::NUM.pow(WORD_SIZE as u32);

    pub fn with_guess_answer(guess: &str, answer: &str) -> Self {
        use Coloring::*;
        let mut out = Self::default();

        debug_assert!(is_wordle_str(answer));
        let mut answer_letter_counts = count_letters(answer);

        let answer_bytes = answer.as_bytes();
        debug_assert!(is_wordle_str(guess));
        let guess_bytes = guess.as_bytes();

        // GREEN pass
        for i in 0..WORD_SIZE {
            let gc = &guess_bytes[i];
            let ac = &answer_bytes[i];

            if gc == ac {
                answer_letter_counts[letter_idx(*gc)] -= 1;
                out[i] = Correct;
            }
        }

        // YELLOW pass
        for i in 0..WORD_SIZE {
            if out[i] != Correct {
                let gc = &guess_bytes[i];
                let counter = &mut answer_letter_counts[letter_idx(*gc)];
                if *counter > 0 {
                    *counter -= 1;
                    out[i] = Misplaced;
                }
            }
        }

        out
    }

    pub fn to_code(&self) -> ColoringCode {
        let mut out = 0;
        let mut multiplier = 1;
        for i in 0..WORD_SIZE {
            out += self[i].ordinal() * multiplier;
            multiplier *= Coloring::NUM as u8;
        }
        out
    }

    pub fn from_code(mut code: ColoringCode) -> Option<Self> {
        let mut out = Self::default();
        for i in 0..WORD_SIZE {
            out[i] = Coloring::from_ordinal(code % (Coloring::NUM as u8))?;
            code /= Coloring::NUM as u8;
        }

        Some(out)
    }

    // for testing
    #[cfg(test)]
    fn iter_all_possible() -> IterAllColorings {
        IterAllColorings::default()
    }
}

impl Default for Colorings {
    fn default() -> Self {
        Self([Coloring::Excluded; WORD_SIZE])
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
            for k in (0..WORD_SIZE).rev() {
                match next[k] {
                    Coloring::Excluded => {
                        next[k] = Coloring::Misplaced;
                        self.next = Some(next);
                        break;
                    }
                    Coloring::Misplaced => {
                        next[k] = Coloring::Correct;
                        self.next = Some(next);
                        break;
                    }
                    Coloring::Correct => {
                        if k == 0 {
                            self.next = None;
                            // implicitly this is break; because 0 is the end
                        } else {
                            next[k] = Coloring::Misplaced;
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