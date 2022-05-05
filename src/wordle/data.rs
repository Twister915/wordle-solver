use std::collections::HashMap;
use std::num::ParseIntError;
use std::str::Utf8Error;
use rust_embed::RustEmbed;
use thiserror::Error;
use lazy_static::lazy_static;
use crate::wordle::data::LoadDataErr::EncodingError;
use crate::wordle::prelude::is_wordle_str;

const FREQUENCY_FILE_NAME: &'static str = "5word_frequencies.txt";
const ALLOWED_WORDS_FILE_NAME: &'static str = "allowed_words.txt";

lazy_static! {
    pub static ref DATA: Data = Data::read().expect("should have no failures reading data...");
}

#[derive(RustEmbed)]
#[folder = "data/"]
struct RawData;

#[derive(Clone, Debug)]
pub struct Data {
    pub frequency_data: FrequencyData,
    pub allowed_words: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct FrequencyData {
    pub by_word: HashMap<String, FrequencyDetail>,
    pub lines: Vec<FrequencyDataLine>,
}

#[derive(Clone, Debug)]
pub struct FrequencyDataLine {
    pub word: String,
    pub detail: FrequencyDetail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrequencyDetail {
    pub frequency: i64,
    pub position: usize,
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
    EncodingError(#[from] Utf8Error)
}

impl Data {
    pub fn read() -> Result<Self, LoadDataErr> {
        let out = Self {
            frequency_data: try_read_frequency_data()?,
            allowed_words: try_read_allowed_words()?,
        };
        log::debug!("got frequency data for {} words", out.frequency_data.by_word.len());
        log::debug!("got {} allowed words from data file", out.allowed_words.len());
        Ok(out)
    }
}

fn try_read_frequency_data() -> Result<FrequencyData, LoadDataErr> {
    let file_data = retrieve_file_as_str(FREQUENCY_FILE_NAME)?
        .ok_or(LoadDataErr::MissingFrequencyDataFile)?;

    const CAPACITY: usize = 100_000;
    let mut lines = Vec::with_capacity(CAPACITY);
    let mut by_word = HashMap::with_capacity(CAPACITY);
    let mut pos = 0;

    for line in file_data.lines() {
        if let Some((l, r)) = line.split_once(" ") {
            let word = l.trim().to_lowercase();
            if is_wordle_str(&word) {
                let frequency = r.trim()
                    .parse::<i64>()
                    .map_err(|err|
                        LoadDataErr::BadFrequencyNumber(r.to_string(), err))?;

                let detail = FrequencyDetail {
                    frequency,
                    position: pos,
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

fn try_read_allowed_words() -> Result<Vec<String>, LoadDataErr> {
    Ok(retrieve_file_as_str(ALLOWED_WORDS_FILE_NAME)?
        .ok_or(LoadDataErr::MissingAllowedWordsFile)?
        .lines()
        .map(|line| line.trim().to_lowercase())
        .filter(|line| is_wordle_str(line))
        .collect())
}

fn retrieve_file_as_str(name: &str) -> Result<Option<String>, LoadDataErr> {
    let f: rust_embed::EmbeddedFile = if let Some(data) = RawData::get(name) {
        data
    } else {
        return Ok(None);
    };

    Ok(Some(std::str::from_utf8(&f.data).map_err(|err| EncodingError(err))?.to_string()))
}