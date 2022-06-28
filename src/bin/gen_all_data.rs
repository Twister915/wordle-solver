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
use std::{io::{self, Write, BufRead}, fs, time::{Duration, Instant}, collections::HashSet};
use wordle_site::wordle::*;

fn main() {
    do_all().expect("should work");
}

fn do_all() -> io::Result<()> {
    write_default_state_data()?;
    write_ordered_allowed()?;
    Ok(())
}

fn write_default_state_data() -> io::Result<()> {
    let at = format!("{}{}", EMBED_DATA_DIRECTORY, DEFAULT_STATE_DATA_FILE_NAME);
    // open the file to contain the cached default state data
    let mut f = fs::File::options()
        .truncate(true)
        .create(true)
        .write(true)
        .open(&at)
        .expect("should open");

    let (dur, out): (Duration, io::Result<()>) = timed(move || {
        // compute the data we should put into the file, and write it...
        for item in Solver::default().compute_top_k_guesses::<{N_RECOMMENDATIONS}>() {
            write!(f, "{} {} {} {}\n", item.word, item.score.abs, item.score.expected_info, item.score.weight)?;
        }
        Ok(())
    });
    out?;
    eprintln!("done! wrote {} recommendations to {} in {:.2}s", N_RECOMMENDATIONS, at, dur.as_secs_f64());
    Ok(())
}

fn write_ordered_allowed() -> io::Result<()> {
    let unordered = read_unordered_allowed_words()?;
    let ordered = read_ordered_frequency_data_words()?;
    let to_write = ordered_words(&unordered, &ordered);

    let at = format!("{}{}", EMBED_DATA_DIRECTORY, ORDERED_ALLOWED_WORDS_FILE_NAME);
    let mut out = io::BufWriter::new(fs::File::options()
        .truncate(true)
        .create(true)
        .write(true)
        .open(&at)?);

    for item in to_write {
        write!(out, "{}\n", item)?;
    }

    Ok(())
}

fn ordered_words<'a>(unordered: &'a Vec<String>, ordered: &'a Vec<String>) -> impl Iterator<Item=&'a str> + 'a {
    let mut unordered_s = HashSet::new();
    unordered_s.extend(unordered.iter().map(|i| i.as_str()));

    let mut ordered_s = HashSet::new();
    ordered_s.extend(ordered.iter().map(|i| i.as_str()));

    ordered.iter()
        .map(|s| s.as_str())
        .filter(move |item| unordered_s.contains(*item))
        .chain(unordered.iter().filter(move |item| !ordered_s.contains(item.as_str())).map(|v| v.as_str()))
}

fn read_unordered_allowed_words() -> io::Result<Vec<String>> {
    let path = format!("{}{}", DATA_DIRECTORY, ALLOWED_WORDS_FILE_NAME);
    let f = fs::File::open(path)?;
    io::BufReader::new(f)
        .lines()
        .map(|l| l.map(|l| normalize_wordle_word(&l)))
        .collect()
}

fn read_ordered_frequency_data_words() -> io::Result<Vec<String>> {
    let path = format!("{}{}", DATA_DIRECTORY, FREQUENCY_FILE_NAME);
    let f = fs::File::open(path)?;
    io::BufReader::new(f)
        .lines()
        .map(|l| l.map(|l| l.split(' ').next().map(|w| normalize_wordle_word(w))))
        .filter_map(|l| match l {
            Ok(Some(v)) => Some(Ok(v)),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        })
        .collect()
}

fn timed<R, F>(f: F) -> (Duration, R)
    where F: FnOnce() -> R
{
    let start_at = Instant::now();
    let out = f();
    let dur = start_at.elapsed();

    (dur, out)
}