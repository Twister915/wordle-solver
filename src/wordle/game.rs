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

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use super::{prelude::*, color::*, data::*};

///
/// The default kind of Solver is a Solver<'static> because the strings being referenced are from
/// constants embedded in the binary.
///
/// You will almost never need to use a different lifetime on a Solver.
///
/// It remains possible to use a lifetime argument for supporting future use-cases (testing?).
///
pub type StaticSolver = Solver<'static>;

///
/// Performs the core task of this app- solving wordle!
///
/// You can initialize this using the Default implementation.
///
/// This struct likes to hold &str items, because it reuses the same strings over and over in
/// multiple fields. The struct itself cannot really own the String objects without lots of
/// duplicated allocations. Therefore, it expects whoever owns Solver to handle allocating the
/// Strings to acquire &str refs. Often, these &str refs are &'static str, but we also don't want
/// to constrain users of Solver to only using &'static str, so we make this lifetime argument 'a.
///
pub struct Solver<'a> {
    /// an unchanging set of all words which you're allowed to guess
    possible_words: HashSet<&'a str>,

    /// "weight" of seeing a given word. The values in this map do not sum to 1.0 and aren't
    /// probabilities, but instead indicate the relative frequency of various possible_words
    word_weights: HashMap<&'a str, WordleFloat>,

    /// it is extremely expensive to compute the scores in the "default state" (when no guesses have
    /// been made) because the algorithm scales with the square of the possibilities remaining,
    /// making the very first computation the most expensive.
    ///
    /// Therefore we support a "cached" version of this calculation
    ///
    /// At compile time (thanks to the trunk pre-build hook & the code in gen_default_state_data)
    /// we generate a text file which contains some top N scores and put that data into this field
    /// at runtime.
    ///
    /// It is an Option because we need to not load the data from a text file during the generation
    /// of the text file.
    default_state_guesses: Option<Vec<ScoredCandidate<'a>>>,

    /// The guesses that the user has made thus far. It is Option because we start off with None,
    /// and change to Some when a guess is made.
    guesses: [Option<Guess>; NUM_TURNS],

    /// The subset of possible_words which remain. Possibilities are eliminated as guesses are made,
    /// so this subset is updated upon each guess & gets smaller as the game goes on.
    remaining_possibilities: HashSet<&'a str>,

    /// word_weights, but the keys are the values in "remaining_possibilities" and the values
    /// are normalized such that they sum to 1.0.
    word_probabilities: HashMap<&'a str, WordleFloat>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Guess {
    pub word: [u8; WORD_SIZE],
    pub coloring: Colorings,
    pub expected_info: WordleFloat,
    pub entropy_delta: WordleFloat,
}

impl Guess {
    /// Whether or not the coloring indicates that this guess is the correct answer
    pub fn is_correct(&self) -> bool {
        self.coloring.0.iter().all(|v| v == &Coloring::Correct)
    }

    /// Tests if this guess "allows" a different guess. For example: if this guess has a Correct
    /// coloring at position 0 for the letter 'q' but the other uses 'a' in position 0 then that
    /// guess is not allowed.
    pub fn allows_other_guess(&self, other: &str) -> bool {
        debug_assert!(is_wordle_str(other));

        // You shouldn't make the same guess twice! Technically you could but then this bot wouldn't
        // seem very smart lol
        if self.is_guess_same(other) {
            return false;
        }

        // count up how many of each letter we expect to see based on our own coloring
        let mut unused_letter_counts = [0; ALPHABET_SIZE];
        for idx in 0..WORD_SIZE {
            if self.coloring[idx] != Coloring::Excluded {
                unused_letter_counts[letter_idx(self.word[idx])] += 1;
            }
        }

        // determine the letters which we know are excluded
        let excluded = self.determine_excluded_letters();
        let other_bytes = other.as_bytes();

        // we use idx to index other_bytes, self.word, and self.coloring simultaneously
        // we know for certain (because this is wordle) that the arrays are all sized according to
        // WORD_SIZE constant.
        //
        // Code generated from this implementation is actually an unrolled loop because the bounds
        // are based on a constant, but using iterators with zip results in worse performance
        #[allow(clippy::needless_range_loop)]
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

            // if we have a letter marked misplaced, but the "other" uses the same letter in the
            // same position, we know the other answer is impossible
            if coloring == Coloring::Misplaced && matches {
                return false;
            }

            // now check if the letter is excluded explicitly
            //
            // We keep a count of the "budget" for all letters either misplaced/correct.
            //
            // When we encounter a letter that is not obviously incorrect (based on the above rules)
            // we will check if we have "budget" for this letter before checking if it's excluded.
            //
            // This is because letters that are repeated, where at least one instance is marked as
            // excluded, will be marked as completely excluded.
            //
            // Checking the budget before checking whether it's excluded allows us to correctly
            // handle repeated letters, such as this case:
            //    word=abbey, coloring=[CORRECT, CORRECT, EXCLUDED, EXCLUDED, EXCLUDED]
            // then the letter 'b' would be marked as excluded. If the guess we're testing was
            // "abhor" then 'b' would be marked as excluded despite being CORRECT. Because we know
            // the "budget" of "b" is 1, we check that first, decrement it, and then because no
            // further "b" is in "abhor" the entire function should (correctly) return true
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

    ///
    /// Outputs true/false flags for each letter in the alphabet, where true indicates that the
    /// letter has been flagged as "excluded" at least once.
    ///
    /// "excluded" might be true for a given letter & it still appears in the word, due to
    /// repetition of letters.
    ///
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

/// Implementation of Default uses the embedded data to construct a solver
impl Default for Solver<'static> {
    fn default() -> Self {
        let possible_words = DATA.allowed_words
            .iter()
            .map(|v| v.as_str())
            .collect();

        let word_weights =
            compute_word_weights(&DATA.allowed_words)
                .collect();
        let word_probabilities =
            compute_word_probabilities(&possible_words, &word_weights)
                .collect();
        let remaining_possibilities = possible_words.clone();
        let default_state_guesses = DATA.default_state_data
            .as_ref()
            .map(|dsd|
                compute_default_state_guesses(&possible_words, dsd)
                    .collect());

        Self {
            possible_words,
            word_weights,
            default_state_guesses,

            guesses: [None; NUM_TURNS],
            remaining_possibilities,
            word_probabilities,
        }
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
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
    ///
    /// Add a guess to the solver so that it makes new recommendations based on the new information
    /// provided by the user.
    ///
    pub fn make_guess(&mut self, guess: &str, coloring: Colorings) -> Result<(), SolverErr> {
        // if we're solved, we cannot make a guess
        if self.is_solved() {
            return Err(SolverErr::AlreadySolved);
        }

        // if there's no possibilities remaining, we cannot make a guess
        if !self.has_possible_guesses() {
            return Err(SolverErr::NoCandidates);
        }

        // try to find out which index to put this guess into, and if we can't find one it's because
        // we've already made the maximum number of guesses (so we return an error with ?)
        let next_guess_idx = self.next_guess_idx().ok_or(SolverErr::TurnsExhausted)?;

        // ensure the provided guess is a 5 letter word in lowercase ascii
        let guess = normalize_wordle_word(guess);
        if !is_wordle_str(&guess) {
            return Err(SolverErr::InvalidGuess(guess));
        }

        // copy guess characters to a fixed size byte array (we cannot use .as_bytes() because it's
        // a fixed size array [u8; WORD_SIZE(5)], not a &[u8])
        let mut word = [0u8; WORD_SIZE];
        word.copy_from_slice(guess.as_bytes());

        // track entropy in the puzzle, so we can calculate the delta after making the guess
        let start_entropy = self.remaining_entropy();

        // store the guess in the guesses array
        self.guesses[next_guess_idx] = Some(Guess {
            coloring,
            word,
            expected_info: self.expected_guess_info(&guess),
            entropy_delta: 0.0,
        });

        // update internal state (such as remaining guesses, probabilities, etc) for the solver
        self.recompute_after_guess();

        // re-calculate the puzzle entropy
        let new_entropy = self.remaining_entropy();
        // update the entropy_delta of the guess we just made, now that we can compute
        self.guesses[next_guess_idx].as_mut().unwrap().entropy_delta = start_entropy - new_entropy;

        Ok(())
    }

    fn recompute_after_guess(&mut self) {
        self.recompute_possibilities();
        self.recompute_word_probabilities();
    }

    ///
    /// This updates self.remaining_possibilities such that it only contains possible guesses given
    /// the "rules" specified by the colorings.
    ///
    /// For example, if we have a single guess like "quack" with coloring [🟩,🟩,X,🟩,🟩], then we
    /// can clearly eliminate a possible answer such as "tares" because "q" must be in the first
    /// position.
    ///
    fn recompute_possibilities(&mut self) {
        // retain removes items from the set when the closure returns false
        self.remaining_possibilities.retain(|word|
            is_guess_allowed_by_existing_guesses(&self.guesses, *word))
    }

    ///
    /// This recalculates the data in self.word_probabilities to meet word_probabilities' definition.
    /// Should be called whenever remaining_possibilities is updated...
    ///
    fn recompute_word_probabilities(&mut self) {
        self.word_probabilities.clear();
        self.word_probabilities.extend(
            compute_word_probabilities(&self.remaining_possibilities, &self.word_weights));

        debug_assert!(
            self.word_probabilities.is_empty() ||
                (self.word_probabilities
                    .values()
                    .copied()
                    .sum::<WordleFloat>() - 1.0).abs() < 0.000001,
            "weights must add up to exactly 1.0",
        );
    }

    ///
    /// This function is true when a guess is allowed, false otherwise in these cases:
    ///   * The puzzle is solved
    ///   * All possible guesses have been eliminated
    ///   * Turns are exhausted
    ///
    pub fn can_guess(&self) -> bool {
        self.num_guesses() < NUM_TURNS && !self.is_solved() && self.has_possible_guesses()
    }

    ///
    /// This function simply tests if a guess is in the possible_words set. This does not indicate
    /// whether a guess is allowed given guesses that have already been made.
    ///
    pub fn is_guess_permitted(&self, guess: &str) -> bool {
        self.possible_words.contains(guess)
    }

    ///
    /// Indicates whether or not the puzzle is solved (the final guess is all green)
    ///
    pub fn is_solved(&self) -> bool {
        self.iter_guesses()
            .last()
            .map(|g| g.is_correct())
            .unwrap_or(false)
    }

    ///
    /// Returns the number of guesses already made
    ///
    pub fn num_guesses(&self) -> usize {
        self.next_guess_idx().unwrap_or(NUM_TURNS)
    }

    ///
    /// Indicates whether or not any words can be guessed. This returns false in cases where the
    /// guesses already made have eliminated all possibilities.
    ///
    pub fn has_possible_guesses(&self) -> bool {
        !self.remaining_possibilities.is_empty()
    }

    ///
    /// Returns the number of possible guesses which remain
    ///
    pub fn num_remaining_possibilities(&self) -> usize {
        self.remaining_possibilities.len()
    }

    ///
    /// Returns the number of possible guesses, without considering any guesses that have been made
    ///
    pub fn num_total_possibilities(&self) -> usize {
        self.possible_words.len()
    }

    ///
    /// Determines the "entropy" of the puzzle given the guesses that remain.
    ///
    /// todo document this
    ///
    pub fn remaining_entropy(&self) -> WordleFloat {
        self.word_probabilities
            .values()
            .map(|v| v * -(v.log2()))
            .sum()
    }

    ///
    /// Returns the first empty index in the self.guesses array. Indicates many things:
    ///   * How many guesses have been made = the output of this function
    ///   * None = turns exhausted
    ///
    /// This function's primary purpose, however, is to provide the index to store a new guess at
    ///
    fn next_guess_idx(&self) -> Option<usize> {
        // find the first None item in self.guesses
        for (idx, v) in self.guesses.iter().enumerate() {
            if v.is_none() {
                // return it's index
                return Some(idx);
            }
        }

        // there are no None items in self.guesses, therefore we cannot store a new guess, and
        // turns have been exhausted
        None
    }

    ///
    /// "default state" is defined as "no guesses have been made" and we can safely load the cached
    /// default scores to save on that super expensive calculation
    ///
    fn is_default_state(&self) -> bool {
        self.num_guesses() == 0
    }

    ///
    /// Returns the highest scored guesses which remain. A maximum of K items are returned.
    ///
    /// This uses const generics because TopK does, and ultimately this is to avoid allocating a Vec
    /// and sorting it based on score.
    ///
    /// The returned iterator may not return K items, but fewer, if the number of possibilities is
    /// less than K.
    ///
    pub fn top_k_guesses<'b, const K: usize>(&'b self) -> TopK<ScoredCandidate<'a>, K>
        where
            'a: 'b,
            [Option<ScoredCandidate<'a>>; K]: Default,
            [Option<Score>; K]: Default,
    {
        // an efficiency hack, mentioned a few times above... if we are in default state and we have
        // cached data available, then we should return that instead of computing it
        if self.is_default_state() {
            if let Some(dsd) = &self.default_state_guesses {
                if dsd.len() >= K {
                    return dsd.iter().copied().top_k(|item| item.score);
                }
            }
        }

        self.compute_top_k_guesses()
    }

    ///
    /// Returns the highest scored guesses which remain. A maximum of K items are returned.
    ///
    /// You should use the function top_k_guesses instead. This function forces computation of it's
    /// output whereas the top_k_guesses may use cached data when available.
    ///
    /// The reason this function is pub is so that we can call it to generate the cached data for
    /// the default state at compile time (in gen_default_state_data.rs).
    ///
    pub fn compute_top_k_guesses<'b, const K: usize>(&'b self) -> TopK<ScoredCandidate<'a>, K>
        where
            'a: 'b,
            [Option<ScoredCandidate<'a>>; K]: Default,
            [Option<Score>; K]: Default
    {
        self.remaining_possibilities
            .iter()
            .copied()
            .map(|word| ScoredCandidate {
                word,
                score: self.score_guess(word),
            })
            .top_k(|item| item.score)
    }

    ///
    /// Computes a score for a given possible guess
    ///
    fn score_guess(&self, guess: &'a str) -> Score {
        // expected info in bits... explanation & definition below
        let expected_info = self.expected_guess_info(guess);

        // weight (not probability!) of the word
        let weight = self.word_weights
            .get(guess)
            .copied()
            .unwrap_or(MIN_WORD_WEIGHT);

        Score::new(expected_info, weight)
    }

    ///
    /// Computes the "expected info" of a given guess.
    ///
    /// I like to think of "info" as a measurement of how much of the search space is eliminated
    /// with a certain guess.
    ///
    /// This "info" can only be calculated if we know the guess and the coloring which the guess
    /// produces. This is due to the definition of "info..." we must know how much of the search
    /// space is eliminated.
    ///
    /// For example, if we guess "quack" and the coloring is [🟩,🟩,X,🟩,🟩] then we can eliminate
    /// a huge number of other possible answers such as "lifts" or "rates" because the coloring
    /// excludes them (qu_ck are green and therefore the answer must contain them at that position).
    ///
    /// Because we do not know the answer, we cannot know what coloring we will get. Therefore, we
    /// cannot calculate how much info a guess will get. However, we can calculate how much info we
    /// get on average.
    ///
    /// In the above example of "quack" it's actually extremely unlikely to get the coloring
    /// [🟩,🟩,X,🟩,🟩] because very few answers produce this coloring (only example I can think
    /// of is "quick"). Although this coloring is high information, it is low probability.
    ///
    /// The implementation below tries to calculate the probability of each possible coloring.
    /// Consider if a coloring only occurs once, this means two things- it's unlikely to happen, but
    /// if it does then we gain a tremendous amount of information. The information gained and the
    /// probability of seeing a coloring are "two sides of the same coin."
    ///
    /// The probability of seeing a coloring is calculated first, then the info gained by that
    /// coloring is log2 of that probability. For example, if the probability of a coloring is 0.5,
    /// that means half the search space produces that coloring (and the other half is eliminated)
    /// giving us -log2(0.5) = 2 bits of information with a 0.5 probability.
    ///
    /// The "expected info" is therefore the sum of p * -p.log2() for all colorings.
    ///
    fn expected_guess_info(&self, guess: &'a str) -> WordleFloat {
        // This array holds a float  for each coloring which tracks the probability of it occurring.
        #[allow(clippy::unnecessary_cast)]
        let mut probabilities = [0.0 as WordleFloat; Colorings::NUM_STATES];

        // go through all possible answers that remain
        for possible_answer in &self.remaining_possibilities {
            // Figure out how probable this answer actually is...
            // This is based on english word frequency data, and the sum of freq_weight_for applied
            // across all possibilities should be 1.0
            let weight = self.word_probability_for(possible_answer);

            // determine what coloring we'd get if we used guess & assumed answer=possible_answer
            let coloring = Colorings::with_guess_answer(guess, possible_answer);

            // this converts the coloring to a unique index (it's like a base 3 number, see to_code)
            let bucket_idx = coloring.to_code() as usize;

            // we add the weight to the bucket because OR probabilities add (the chance of seeing
            // a given coloring = chance of word A || chance of word B || ... when A, B, ...
            // give that coloring)
            probabilities[bucket_idx] += weight;
        }

        // ensure (in debug builds only) that the sum of all probabilities is (approximately) 1.0
        debug_assert!((probabilities.iter().sum::<WordleFloat>() - 1.0).abs() < 0.0001);

        // determine the average information gained
        #[allow(clippy::unnecessary_cast)]
        probabilities.iter()
            // filter non-positive data (aka the 0s) because log2(0) is undefined
            .filter(|v| *v > &(0.0 as WordleFloat))
            .map(|v| v * -(v.log2()))
            .sum()
    }

    ///
    /// Look up the probability of a given guess (not weight!).
    ///
    /// The word must be in remaining_possibilities (and therefore word_probabilities) or no
    /// probability will be found.
    ///
    /// In a perfect world this would return an Option... but panics if no probability is found
    ///
    fn word_probability_for(&self, guess: &'a str) -> WordleFloat {
        self.word_probabilities[guess]
    }

    ///
    /// Returns all the guesses we've made so far.
    ///
    pub fn iter_guesses<'b>(&'b self) -> impl Iterator<Item=&'b Guess> + 'b where 'a: 'b {
        iter_guesses(&self.guesses)
    }

    ///
    /// Clears all guesses we've made and resets all state to original state. This avoids
    /// recalculating some data (such as word_weights) when we play another game
    ///
    pub fn reset(&mut self) {
        self.guesses = [None; NUM_TURNS];
        self.remaining_possibilities.clear();
        self.remaining_possibilities.extend(&self.possible_words);
        self.recompute_word_probabilities();
    }
}

///
/// Returns whether or not the provided guesses allow the provided guess
///
/// This is external to the solver because it is used in only one place- recompute_possibilities
/// which borrows solver '&mut self'
///
/// This function would be defined &self meaning recompute_possibilities would borrow self immutably
/// & mutably, which is an error.
///
/// Therefore, we allow borrowing of the field &self.guesses (and passing that to this function)
/// instead, which is not an error.
///
/// Think of it as constraining the scope of the immutable borrow to a single field, instead of
/// borrowing the entire Solver struct to determine if the guess is allowed.
///
fn is_guess_allowed_by_existing_guesses(guesses: &[Option<Guess>], guess: &str) -> bool {
    iter_guesses(guesses).all(|g| g.allows_other_guess(guess))
}

///
/// Helper which takes any slice of Option<Guess> and iterates through references to the Guesses
/// that have been made.
///
pub fn iter_guesses(guesses: &[Option<Guess>]) -> impl Iterator<Item=&Guess> {
    OptionIter::new(guesses.iter().map(|v| v.as_ref()))
}

///
/// This function computes "weights" (not probabilities) for the possible_guesses.
///
/// Based on the 3blue1brown implementation, we base the weight on the word's rank.
///
/// An arbitrary line called N_COMMON(=2700) is defined. Words with lower ranks (ie; more common
/// words with rank 0, 1, 2, etc) are considered common, whereas words with ranks higher than
/// N_COMMON are considered uncommon.
///
/// A WIDTH is defined, and this is a unitless scaling factor.
///
/// A value called "x" is calculated for each word. Imagine this as a position along a sigmoid curve.
/// The most common word (rank=0) is given an "x" value = WIDTH, and words with lower ranks are
/// linearly spaced such that the word with rank N_COMMON has an "x" value of 0. Words with ranks
/// lower than N_COMMON continue the same linear spacing into negative numbers off to -inf.
///
/// The "x" value is then passed into sigmoid so that it exists between (0.0, 1.0) for all words,
/// and this is the "weight"
///
/// Finally, we use MIN_WORD_WEIGHT when no frequency data exists for a given word, or when the
/// computed weight is below MIN_WORD_WEIGHT. When a word does not have frequency data, it is a
/// fair assumption that it is extremely uncommon.
///
/// The constants N_COMMON and WIDTH can be tuned to possibly yield better results. Their values
/// depend on the size of the allowed_words and frequency data file. If you use a different dataset
/// for word frequency it is recommended to experiment and tune these constants to this new dataset.
///
fn compute_word_weights(ordered_words: &Vec<String>) -> impl Iterator<Item=(&str, WordleFloat)> {

    // Implementation defines a few helper functions...
    //
    // * raw_compute_word_wight = actually do the computation, sometimes returning None when no
    //                            data exists about a word
    // * compute_word_weight = do the computation, but default to MIN_WORD_WEIGHT
    //
    #[inline]
    fn raw_compute_word_weight(n_words: WordleFloat, rank: WordleFloat) -> Option<WordleFloat> {
        const N_COMMON: WordleFloat = 2700.0;
        const WIDTH: WordleFloat = 5.7;

        let x = ((N_COMMON - rank) / n_words) * WIDTH;
        let weight = sigmoid(x);

        Some(if weight < MIN_WORD_WEIGHT {
            MIN_WORD_WEIGHT
        } else {
            weight
        })
    }

    #[inline]
    fn compute_word_weight(n_words: WordleFloat, rank: usize) -> WordleFloat {
        raw_compute_word_weight(n_words, rank as WordleFloat).unwrap_or(MIN_WORD_WEIGHT)
    }

    let n_words = ordered_words.len() as WordleFloat;

    ordered_words
        .iter()
        .map(|w| w.as_str())
        .enumerate()
        .map(move |(idx, w)| (w, compute_word_weight(n_words, idx)))
}

///
/// "weights" is a mapping from possible guesses -> weight of seeing that guess. These values do not
/// sum to 1.0
///
/// "words" is a subset of the keys from "weights"
///
/// The output is (word, probability) pairs such that:
///   * only words in the "words" HashSet are emitted
///   * all probability values sum to (approximately) 1.0
///
fn compute_word_probabilities<'a: 'b, 'b>(
    words: &'b HashSet<&'a str>,
    weights: &'b HashMap<&'a str, WordleFloat>,
) -> impl Iterator<Item=(&'a str, WordleFloat)> + 'b
{
    // get weights for each of the words provided, and sum that up, so we can perform normalization
    let total: WordleFloat = words.iter().map(|w| weights[w]).sum();
    // go through all the words (again) and divide each weight by the sum, producing a probability
    words.iter().map(move |w| (*w, weights[w] / total))
}

///
/// The cached default state data is stored as &[DefaultStateEntry] and this function helps convert
/// that to Vec<ScoredCandidate<'a>>.
///
fn compute_default_state_guesses<'a: 'b, 'b>(
    words: &'b HashSet<&'a str>,
    supplied_data: &'b [DefaultStateEntry],
) -> impl Iterator<Item=ScoredCandidate<'a>> + 'b
{
    // go through the linear data from the text-file
    supplied_data.iter().map(|entry| {
        // find the correct &str from the 'words' HashSet
        let word = *words.iter()
            .find(|item| *item == &entry.word)
            .expect("default state data should contain possible words only");

        // create the score
        let score = Score {
            abs: entry.score,
            expected_info: entry.expected_info,
            weight: entry.weight,
        };

        // combine
        ScoredCandidate {
            word,
            score,
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::wordle::{Guess, iter_guesses};

    #[test]
    fn test_guess_iterator() {
        use crate::wordle::Coloring::*;
        let example_guess = Guess {
            word: [b'a', b'a', b'a', b'a', b'a'],
            coloring: [Excluded, Excluded, Excluded, Excluded, Excluded].into(),
            expected_info: 0.0,
            entropy_delta: 0.0
        };

        {
            let guesses = [Some(example_guess.clone()), Some(example_guess.clone()), None, None, None, None];
            let data: Vec<Guess> = iter_guesses(&guesses).cloned().collect();
            let expected = [example_guess.clone(), example_guess.clone()];
            assert_eq!(&data, &expected, "should have exactly two guesses");
        }

        {
            let guesses = [None, Some(example_guess.clone()), Some(example_guess.clone()), None, None, None];
            let count = iter_guesses(&guesses).count();
            assert_eq!(count, 0, "even though there are some guesses, they must be in order, and the first is None therefore there are no guesses, so the count should be 0... got {}", count);
        }
    }
}