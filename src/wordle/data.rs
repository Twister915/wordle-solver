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
use lazy_static::lazy_static;
use rust_embed::RustEmbed;
use std::num::ParseFloatError;
use std::str::Utf8Error;
use thiserror::Error;

// Stores "input data" which is manually updated/configured
pub const DATA_DIRECTORY: &str = "data/";
pub const FREQUENCY_FILE_NAME: &str = "5word_frequencies.txt";
pub const ALLOWED_WORDS_FILE_NAME: &str = "allowed_words.txt";

// Stores "derived data" which is generated at build time using the data from the text-files above
pub const EMBED_DATA_DIRECTORY: &str = "txt_data/";
pub const DEFAULT_STATE_DATA_FILE_NAME: &str = "default_state_data.txt";
pub const ORDERED_ALLOWED_WORDS_FILE_NAME: &str = "allowed_words_ord.txt";

lazy_static! {
    pub static ref DATA: Data = Data::read().expect("should have no failures reading data...");
}

#[derive(RustEmbed)]
#[folder = "txt_data/"]
struct RawData;

/// Holds all of the data represented by the static/embedded text files
#[derive(Clone, Debug)]
pub struct Data {
    /// The list of words which can be guessed, in rank order from most common to least common
    pub allowed_words: Vec<String>,
    /// Cached calculation of scored guesses in the "default state" (see game.rs for more details)
    pub default_state_data: Option<Vec<DefaultStateEntry>>,
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
    #[error("missing allowed words file")]
    MissingAllowedWordsFile,
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
            allowed_words: try_read_allowed_words()?,
            default_state_data: try_read_default_state_data()?,
        };
        log::debug!(
            "got {} allowed words from data file",
            out.allowed_words.len()
        );
        if let Some(default_state) = &out.default_state_data {
            log::debug!("got {} default items", default_state.len());
        }
        Ok(out)
    }
}

/// Reads the allowed words text file. This is pretty simple: one allowed word per line.
fn try_read_allowed_words() -> Result<Vec<String>, LoadDataErr> {
    Ok(retrieve_file_as_str(ORDERED_ALLOWED_WORDS_FILE_NAME)?
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
    for line in raw_data.lines() {
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
            let raw = parts
                .next()
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
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(mut f) = std::fs::File::open(format!("{}{}", EMBED_DATA_DIRECTORY, name)) {
            let mut out = String::default();
            if std::io::Read::read_to_string(&mut f, &mut out).is_ok() {
                return Ok(Some(out));
            }
        }

        return Ok(None);
    };

    Ok(Some(
        std::str::from_utf8(&f.data)
            .map_err(LoadDataErr::EncodingError)?
            .to_string(),
    ))
}
