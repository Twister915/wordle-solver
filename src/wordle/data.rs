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

use crate::wordle::prelude::*;
use std::collections::HashMap;
use std::num::{ParseFloatError, ParseIntError};
use std::str::Utf8Error;
use rust_embed::RustEmbed;
use thiserror::Error;
use lazy_static::lazy_static;

pub const DATA_DIRECTORY: &str = "data/";
pub const FREQUENCY_FILE_NAME: &str = "5word_frequencies.txt";
pub const ALLOWED_WORDS_FILE_NAME: &str = "allowed_words.txt";
pub const DEFAULT_STATE_DATA_FILE_NAME: &str = "default_state_data.txt";

lazy_static! {
    pub static ref DATA: Data = Data::read().expect("should have no failures reading data...");
}

#[derive(RustEmbed)]
#[folder = "data/"]
struct RawData;

/// Holds all of the data represented by the static/embedded text files
#[derive(Clone, Debug)]
pub struct Data {
    /// Word frequencies in english for relative ranking & weight/probability calculations
    pub frequency_data: FrequencyData,
    /// The list of words which can be guessed
    pub allowed_words: Vec<String>,
    /// Cached calculation of scored guesses in the "default state" (see game.rs for more details)
    pub default_state_data: Option<Vec<DefaultStateEntry>>,
}

#[derive(Clone, Debug)]
pub struct FrequencyData {
    /// All parsed lines from the frequency data file, in the order they were read
    pub lines: Vec<FrequencyDataLine>,

    /// Same data as lines but organized in a map so you can look up the data for each word quickly
    pub by_word: HashMap<String, FrequencyDetail>,
}

#[derive(Clone, Debug)]
pub struct FrequencyDataLine {
    /// The word on this line
    pub word: String,
    /// The data associated with that word
    pub detail: FrequencyDetail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrequencyDetail {
    /// How often this dataset claims to have seen this word
    pub frequency: i64,
    /// Rank=0 means most common word in english (according to this dataset), and higher ranks being
    /// less common words.
    pub rank: usize,
}

#[derive(Clone, Debug)]
pub struct DefaultStateEntry {
    /// The word being suggested
    pub word: String,
    /// The total score (score.abs)
    pub score: WordleFloat,
    /// The expected_info for this guess (score.expected_info)
    pub expected_info: WordleFloat,
    /// The weight calculated for this guess (score.weight)
    pub weight: WordleFloat,
}

#[derive(Error, Debug)]
pub enum LoadDataErr {
    #[error("missing frequency data file")]
    MissingFrequencyDataFile,
    #[error("missing allowed words file")]
    MissingAllowedWordsFile,
    #[error("failed to parse number '{0}'")]
    BadFrequencyNumber(String, #[source] ParseIntError),
    #[error(transparent)]
    EncodingError(#[from] Utf8Error),
    #[error("malformed default data line '{0}'")]
    BadDefaultDataLine(String),
    #[error("malformed floating point text '{0}'")]
    BadFloatStr(String, #[source] ParseFloatError),
    #[error("the word '{0}' is not a valid wordle word")]
    NonWordleWord(String),
}

impl Data {
    pub fn read() -> Result<Self, LoadDataErr> {
        let out = Self {
            frequency_data: try_read_frequency_data()?,
            allowed_words: try_read_allowed_words()?,
            default_state_data: try_read_default_state_data()?,
        };
        log::debug!("got frequency data for {} words", out.frequency_data.by_word.len());
        log::debug!("got {} allowed words from data file", out.allowed_words.len());
        if let Some(default_state) = &out.default_state_data {
            log::debug!("got {} default items", default_state.len());
        }
        Ok(out)
    }
}

/// Parses frequency data text into FrequencyData
fn try_read_frequency_data() -> Result<FrequencyData, LoadDataErr> {
    let file_data = retrieve_file_as_str(FREQUENCY_FILE_NAME)?
        .ok_or(LoadDataErr::MissingFrequencyDataFile)?;

    const CAPACITY: usize = 100_000;
    let mut lines = Vec::with_capacity(CAPACITY);
    let mut by_word = HashMap::with_capacity(CAPACITY);
    let mut pos = 0;

    for line in file_data.lines() {
        // frequency data is expected in the following format:
        //
        // word1 123123
        // word2 3213
        // ...
        //
        // We simply need to identify the word & the number following it (the "frequency").
        if let Some((l, r)) = line.split_once(' ') {
            // clean up the word
            let word = normalize_wordle_word(l);
            // verify it's a 5 letter word in the right case
            if is_wordle_str(&word) {
                // parse the number
                let frequency = r.trim()
                    .parse::<i64>()
                    .map_err(|err|
                        LoadDataErr::BadFrequencyNumber(r.to_string(), err))?;

                // store the line!
                let detail = FrequencyDetail {
                    frequency,
                    rank: pos,
                };
                pos += 1;

                let line = FrequencyDataLine {
                    word: word.clone(),
                    detail,
                };

                lines.push(line);
                by_word.insert(word, detail);
            }
        }
    }

    Ok(FrequencyData {
        lines,
        by_word,
    })
}

/// Reads the allowed words text file. This is pretty simple: one allowed word per line.
fn try_read_allowed_words() -> Result<Vec<String>, LoadDataErr> {
    Ok(retrieve_file_as_str(ALLOWED_WORDS_FILE_NAME)?
        .ok_or(LoadDataErr::MissingAllowedWordsFile)?
        .lines()
        .map(normalize_wordle_word)
        .filter(|line| is_wordle_str(line))
        .collect())
}

/// Reads cached default state data, optionally (if it exists)
fn try_read_default_state_data() -> Result<Option<Vec<DefaultStateEntry>>, LoadDataErr> {
    // try to open the default state data (if it doesn't exist, then just return Ok(None))
    let raw_data = match retrieve_file_as_str(DEFAULT_STATE_DATA_FILE_NAME)? {
        Some(data) => data,
        None => return Ok(None),
    };

    let mut out = Vec::with_capacity(N_RECOMMENDATIONS);
    // parse each line in default_state_data
    for line in raw_data.lines()
    {
        // this file is expected to contain 4 pieces of information on each line, split by a space:
        //
        // * word being suggested (5 letter string / wordle word)
        // * it's score (float)
        // * it's expected_info (float)
        // * it's weight (float)
        //
        // The file should also be already sorted from highest -> lowest score
        //
        let mut parts = line.splitn(4, ' ');

        // read the word
        let word = if let Some(w) = parts.next() {
            normalize_wordle_word(w)
        } else {
            continue;
        };

        // validate
        if !is_wordle_str(&word) {
            return Err(LoadDataErr::NonWordleWord(word));
        }

        // helper closure to "consume" a float
        // basically reads whatever parts.next() returns as a float, returning an error if the float
        // isn't valid, or doesn't exist
        let mut consume_float = || {
            // first get the string representation & handle the case when it doesn't exist
            let raw = parts.next()
                .ok_or_else(|| LoadDataErr::BadDefaultDataLine(line.to_string()))?;

            // then try to parse it as a WordleFloat (aka f32/f64), and wrap the error if it can't
            // be parsed
            raw.trim()
                .parse::<WordleFloat>()
                .map_err(|err| LoadDataErr::BadFloatStr(raw.to_string(), err))
        };

        // consume the three floats (score, expected_info, weight)
        let score = consume_float()?;
        let expected_info = consume_float()?;
        let weight = consume_float()?;
        out.push(DefaultStateEntry {
            word,
            score,
            expected_info,
            weight,
        });
    }

    Ok(Some(out))
}

fn retrieve_file_as_str(name: &str) -> Result<Option<String>, LoadDataErr> {
    let f: rust_embed::EmbeddedFile = if let Some(data) = RawData::get(name) {
        data
    } else {
        return Ok(None);
    };

    Ok(Some(std::str::from_utf8(&f.data).map_err(LoadDataErr::EncodingError)?.to_string()))
}
