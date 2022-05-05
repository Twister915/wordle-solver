use std::collections::HashMap;
use std::num::{ParseFloatError, ParseIntError};
use std::str::Utf8Error;
use rust_embed::RustEmbed;
use thiserror::Error;
use lazy_static::lazy_static;
use crate::web::solver_agent::N_RECOMMENDATIONS;
use crate::wordle::data::LoadDataErr::EncodingError;
use crate::wordle::prelude::is_wordle_str;
use crate::wordle::WordleFloat;

pub const DATA_DIRECTORY: &'static str = "data/";
pub const FREQUENCY_FILE_NAME: &'static str = "5word_frequencies.txt";
pub const ALLOWED_WORDS_FILE_NAME: &'static str = "allowed_words.txt";
pub const DEFAULT_STATE_DATA_FILE_NAME: &'static str = "default_state_data.txt";

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
    pub default_state_data: Vec<DefaultStateEntry>,
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

#[derive(Clone, Debug)]
pub struct DefaultStateEntry {
    pub word: String,
    pub score: WordleFloat,
    pub expected_info: WordleFloat,
    pub weight: WordleFloat,
}

#[derive(Error, Debug)]
pub enum LoadDataErr {
    #[error("missing frequency data file")]
    MissingFrequencyDataFile,
    #[error("missing allowed words file")]
    MissingAllowedWordsFile,
    #[error("missing default state data file")]
    MissingDefaultStateDataFile,
    #[error("failed to parse number '{0}'")]
    BadFrequencyNumber(String, #[source] ParseIntError),
    #[error(transparent)]
    EncodingError(#[from] Utf8Error),
    #[error("malformed default data line '{0}'")]
    BadDefaultDataLine(String),
    #[error("malformed floating point text '{0}'")]
    BadFloatStr(String, #[source] ParseFloatError),
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

fn try_read_default_state_data() -> Result<Vec<DefaultStateEntry>, LoadDataErr> {
    let mut out = Vec::with_capacity(N_RECOMMENDATIONS);
    for line in retrieve_file_as_str(DEFAULT_STATE_DATA_FILE_NAME)?
        .ok_or(LoadDataErr::MissingDefaultStateDataFile)?
        .lines()
    {
        let mut parts = line.splitn(4, " ");
        let word = if let Some(w) = parts.next() {
            w.to_string()
        } else {
            continue;
        };

        let mut consume_float = || {
            let raw = parts.next()
                .ok_or_else(|| LoadDataErr::BadDefaultDataLine(line.to_string()))?;
            raw.trim()
                .parse::<WordleFloat>()
                .map_err(|err| LoadDataErr::BadFloatStr(raw.to_string(), err))
        };

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

    Ok(out)
}

fn retrieve_file_as_str(name: &str) -> Result<Option<String>, LoadDataErr> {
    let f: rust_embed::EmbeddedFile = if let Some(data) = RawData::get(name) {
        data
    } else {
        return Ok(None);
    };

    Ok(Some(std::str::from_utf8(&f.data).map_err(|err| EncodingError(err))?.to_string()))
}
