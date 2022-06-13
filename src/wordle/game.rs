use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use super::{prelude::*, color::*, data::*};
use serde::{Serialize, Deserialize};

pub struct Solver<'a> {
    possible_words: HashSet<&'a str>,
    word_probabilities: HashMap<&'a str, WordleFloat>,
    default_state_guesses: Option<Vec<ScoredCandidate<'a>>>,

    guesses: [Option<Guess>; NUM_TURNS],
    remaining_possibilities: HashSet<&'a str>,
    word_weights: HashMap<&'a str, WordleFloat>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Guess {
    pub word: [u8; WORD_SIZE],
    pub coloring: Colorings,
    pub expected_info: WordleFloat,
    pub entropy_delta: WordleFloat,
}

impl Guess {
    pub fn is_correct(&self) -> bool {
        self.coloring.0.iter().all(|v| v == &Coloring::Correct)
    }

    pub fn allows_other_guess(&self, other: &str) -> bool {
        debug_assert!(is_wordle_str(other));

        if self.is_guess_same(other) {
            return false;
        }

        let other_bytes = other.as_bytes();
        let mut unused_letter_counts = [0; ALPHABET_SIZE];
        for idx in 0..WORD_SIZE {
            if self.coloring[idx] != Coloring::Excluded {
                unused_letter_counts[letter_idx(self.word[idx])] += 1;
            }
        }

        let excluded = self.determine_excluded_letters();

        for idx in 0..WORD_SIZE {
            let other_c = other_bytes[idx];
            let self_c = self.word[idx];
            let coloring = self.coloring[idx];
            let matches = other_c == self_c;

            // if we have something marked correct, but the "other" uses a different letter, then
            // obviously this other answer is impossible
            if coloring == Coloring::Correct && !matches {
                return false;
            }

            // if we have a letter marked misplaced, but the "other" uses the same letter in the same
            // position, we know the other answer is impossible
            if coloring == Coloring::Misplaced && matches {
                return false;
            }

            // now check if the letter is excluded explicitly
            //
            // We keep a count of the "budget" for all letters either misplaced/correct.
            //
            // When we encounter a letter that is not obviously incorrect (based on the above rules)
            // we will check if we have "budget" for this letter before checking if it's expliclty
            // excluded.
            //
            // This is because letters that are repeated, where at least one instance is marked as
            // excluded, will be marked as completely excluded.
            //
            // Checking the budget before checking whether it's excluded allows us to correctly handle
            // repeated letters, such as this case:
            //    word=abbey, coloring=[CORRECT, CORRECT, EXCLUDED, EXCLUDED, EXCLUDED]
            // then the letter 'b' would be marked as excluded. If the guess we're testing was "abhor"
            // then "b" would be marked as excluded despite being CORRECT. Because we know the "budget"
            // of "b" is 1, we check that first, decrement it, and then because no further "b" is in
            // "abhor" the entire function should (correctly) return true
            let other_letter_idx = letter_idx(other_c);
            let counter = &mut unused_letter_counts[other_letter_idx];
            if *counter > 0 {
                *counter -= 1;
            } else if excluded[other_letter_idx] {
                return false;
            }
        }

        // now verify that we consumed the entire "budget" of known CORRECT/MISPLACED letters
        //
        // this is the last check... if we get to this line all other tests passed, so we can
        // simply return the result of this final check
        unused_letter_counts.iter().all(|count| count == &0)
    }

    fn determine_excluded_letters(&self) -> [bool; ALPHABET_SIZE] {
        let mut out = [false; ALPHABET_SIZE];
        for idx in 0..WORD_SIZE {
            if self.coloring[idx] == Coloring::Excluded {
                out[letter_idx(self.word[idx])] = true;
            }
        }

        out
    }

    pub fn is_guess_same(&self, other: &str) -> bool {
        debug_assert!(is_wordle_str(other));
        let ob = other.as_bytes();
        self.word.iter().enumerate().all(|(i, c)| ob[i] == *c)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ScoredCandidate<'a> {
    pub word: &'a str,
    pub score: Score,
}

impl PartialEq<Self> for Score {
    fn eq(&self, other: &Self) -> bool {
        self.abs.eq(&other.abs)
    }
}

impl PartialOrd<Self> for Score {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.abs.partial_cmp(&other.abs)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Score {
    pub abs: WordleFloat,
    pub expected_info: WordleFloat,
    pub weight: WordleFloat,
}

impl Score {
    pub fn new(expected_info: WordleFloat, weight: WordleFloat) -> Self {
        let abs = Self::calculate_abs(expected_info, weight);
        Self {
            abs,
            expected_info,
            weight,
        }
    }

    pub fn calculate_abs(expected_info: WordleFloat, weight: WordleFloat) -> WordleFloat {
        expected_info + weight
    }
}

impl Default for Solver<'static> {
    fn default() -> Self {
        let possible_words = DATA.allowed_words.iter().map(|v| v.as_str()).collect();
        let frequency_data = &DATA.frequency_data;
        let word_probabilities = compute_word_probabilities(&possible_words, &frequency_data).collect();
        let word_weights = compute_word_weights(&possible_words, &word_probabilities).collect();
        let remaining_possibilities = possible_words.clone();
        let default_state_guesses = DATA.default_state_data.as_ref().map(|dsd| compute_default_state_guesses(&possible_words, dsd).collect());
        Self {
            possible_words,
            word_probabilities,
            default_state_guesses,

            guesses: [None; NUM_TURNS],
            remaining_possibilities,
            word_weights,
        }
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolverErr {
    #[error("no possible words remain")]
    NoCandidates,
    #[error("no turns remaining")]
    TurnsExhausted,
    #[error("the wordle puzzle is already solved")]
    AlreadySolved,
    #[error("provided guess is not valid")]
    InvalidGuess(String),
}

impl<'a> Solver<'a> {
    pub fn make_guess(&mut self, guess: &str, coloring: Colorings) -> Result<(), SolverErr> {
        if self.is_solved() {
            return Err(SolverErr::AlreadySolved);
        }

        if !self.has_possible_guesses() {
            return Err(SolverErr::NoCandidates);
        }

        let next_guess_idx = self.next_guess_idx().ok_or(SolverErr::TurnsExhausted)?;
        let guess = normalize_wordle_word(guess);

        if !is_wordle_str(&guess) {
            return Err(SolverErr::InvalidGuess(guess));
        }

        let mut word = [0u8; WORD_SIZE];
        word.copy_from_slice(guess.as_bytes());

        let start_entropy = self.remaining_entropy();
        self.guesses[next_guess_idx] = Some(Guess {
            coloring,
            word,
            expected_info: self.expected_guess_info(&guess),
            entropy_delta: 0.0,
        });

        self.recompute_after_guess();

        self.guesses[next_guess_idx].as_mut().unwrap().entropy_delta = start_entropy - self.remaining_entropy();

        Ok(())
    }

    fn recompute_after_guess(&mut self) {
        self.recompute_possibilities();
        self.recompute_word_weights();
    }

    fn recompute_possibilities(&mut self) {
        self.remaining_possibilities.retain(|word|
            self.guesses
                .iter()
                .filter_map(|g| g.as_ref())
                .fuse()
                .all(|g| g.allows_other_guess(*word)))
    }

    fn recompute_word_weights(&mut self) {
        self.word_weights.clear();
        self.word_weights.extend(compute_word_weights(&self.remaining_possibilities, &self.word_probabilities));

        debug_assert!(
            self.word_weights.is_empty() ||
                (self.word_weights.values().copied().sum::<WordleFloat>() - 1.0).abs() < 0.000001,
            "weights must add up to exactly 1.0",
        );
    }

    pub fn can_guess(&self) -> bool {
        self.num_guesses() < NUM_TURNS && !self.is_solved() && self.has_possible_guesses()
    }

    pub fn can_use_guess(&self, guess: &str) -> bool {
        self.possible_words.contains(guess)
    }

    pub fn is_solved(&self) -> bool {
        let n_guesses = self.num_guesses();
        n_guesses > 0 && self.guesses[n_guesses - 1].map(|v| v.is_correct()).unwrap_or(false)
    }

    pub fn num_guesses(&self) -> usize {
        self.next_guess_idx().unwrap_or(NUM_TURNS)
    }

    pub fn has_possible_guesses(&self) -> bool {
        !self.remaining_possibilities.is_empty()
    }

    pub fn num_remaining_possibilities(&self) -> usize {
        self.remaining_possibilities.len()
    }

    pub fn num_total_possibilities(&self) -> usize {
        self.possible_words.len()
    }

    pub fn remaining_entropy(&self) -> WordleFloat {
        self.remaining_possibilities
            .iter()
            .copied()
            .map(|word| self.freq_weight_for(word))
            .map(|item| item * -(item.log2()))
            .sum()
    }

    fn next_guess_idx(&self) -> Option<usize> {
        for (idx, v) in self.guesses.iter().enumerate() {
            if v.is_none() {
                return Some(idx);
            }
        }

        None
    }

    fn is_default_state(&self) -> bool {
        self.num_guesses() == 0
    }

    pub fn top_k_guesses<'b, const K: usize>(&'b self) -> TopK<ScoredCandidate<'a>, K>
        where
            'a: 'b,
            [Option<ScoredCandidate<'a>>; K]: Default,
            [Option<Score>; K]: Default,
    {
        if self.is_default_state() {
            if let Some(dsd) = &self.default_state_guesses {
                if dsd.len() >= K {
                    return dsd.iter().copied().top_k(|item| item.score);
                }
            }
        }

        self.top_k_guesses_real()
    }

    pub fn top_k_guesses_real<'b, const K: usize>(&'b self) -> TopK<ScoredCandidate<'a>, K>
        where
            'a: 'b,
            [Option<ScoredCandidate<'a>>; K]: Default,
            [Option<Score>; K]: Default
    {
        self.remaining_possibilities
            .iter()
            .copied()
            .map(self.map_to_scored_guess())
            .top_k(|item| item.score)
    }

    fn map_to_scored_guess<'b>(&'b self) -> impl Fn(&'a str) -> ScoredCandidate<'a> + 'b {
        move |word| {
            let score = self.score_guess(word);
            ScoredCandidate {
                word,
                score,
            }
        }
    }

    fn score_guess(&self, guess: &'a str) -> Score {
        let expected_info = self.expected_guess_info(guess);
        let weight = self.word_probabilities
            .get(guess)
            .copied()
            .unwrap_or(MIN_WORD_PROBABILITY);

        Score::new(expected_info, weight)
    }

    fn expected_guess_info(&self, guess: &'a str) -> WordleFloat {
        let mut probabilities: [WordleFloat; Colorings::NUM_STATES] = [0.0 as WordleFloat; Colorings::NUM_STATES];
        self.remaining_possibilities
            .iter()
            .for_each(|possible_answer| {
                let weight = self.freq_weight_for(possible_answer);
                let coloring = Colorings::with_guess_answer(guess, possible_answer);
                let bucket_idx = coloring.to_code() as usize;
                probabilities[bucket_idx] += weight;
            });

        debug_assert!((probabilities.iter().sum::<WordleFloat>() - 1.0).abs() < 0.0001);

        probabilities.iter()
            .filter(|v| *v > &(0.0 as WordleFloat))
            .map(|v| v * -(v.log2()))
            .sum()
    }

    fn freq_weight_for(&self, guess: &'a str) -> WordleFloat {
        self.word_weights[guess]
    }

    pub fn uncertainty(&self) -> WordleFloat {
        self.remaining_possibilities.iter().map(|guess| {
            let p = self.freq_weight_for(guess);
            p * -p.log2()
        }).sum()
    }

    pub fn iter_guesses<'b>(&'b self) -> impl Iterator<Item=Guess> + 'b where 'a: 'b {
        self.guesses.iter().filter_map(|v| v.as_ref()).copied()
    }

    pub fn reset(&mut self) {
        self.guesses = [None; NUM_TURNS];
        self.remaining_possibilities.clear();
        self.remaining_possibilities.extend(&self.possible_words);
        self.recompute_word_weights();
    }
}

fn compute_word_probabilities<'a: 'b, 'b>(
    words: &'b HashSet<&'a str>,
    frequency_data: &'b FrequencyData,
) -> impl Iterator<Item=(&'a str, WordleFloat)> + 'b
{
    #[inline]
    fn raw_compute_word_probability(guess: &str, frequency_data: &FrequencyData) -> Option<WordleFloat> {
        const N_COMMON: WordleFloat = 3200.0;
        const WIDTH: WordleFloat = 5.7;

        let n_words = frequency_data.by_word.len() as WordleFloat;
        let rank = frequency_data.by_word.get(guess)?.position as WordleFloat;
        let x = ((N_COMMON - rank) / n_words) * WIDTH;
        let out = sigmoid(x);

        Some(if out < MIN_WORD_PROBABILITY {
            MIN_WORD_PROBABILITY
        } else {
            out
        })
    }

    #[inline]
    fn compute_word_probability(guess: &str, frequency_data: &FrequencyData) -> WordleFloat {
        raw_compute_word_probability(guess, frequency_data).unwrap_or(MIN_WORD_PROBABILITY)
    }

    words.iter()
        .map(|w| (*w, compute_word_probability(w, frequency_data)))
}

fn compute_word_weights<'a: 'b, 'b>(
    words: &'b HashSet<&'a str>,
    probabilities: &'b HashMap<&'a str, WordleFloat>,
) -> impl Iterator<Item=(&'a str, WordleFloat)> + 'b
{
    let total: WordleFloat = words.iter().map(|w| probabilities[w]).sum();
    words.iter().map(move |w| (*w, probabilities[w] / total))
}

fn compute_default_state_guesses<'a: 'b, 'b>(
    words: &'b HashSet<&'a str>,
    supplied_data: &'b Vec<DefaultStateEntry>,
) -> impl Iterator<Item=ScoredCandidate<'a>> + 'b {
    supplied_data.iter().map(|entry| {
        let word = *words.iter()
            .filter(|item| *item == &entry.word)
            .next()
            .expect("default state data should contain possible words only");

        let score = Score {
            abs: entry.score,
            expected_info: entry.expected_info,
            weight: entry.weight,
        };

        ScoredCandidate {
            word,
            score,
        }
    })
}