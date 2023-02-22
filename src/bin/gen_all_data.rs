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
use std::{
    collections::HashSet,
    fs,
    io::{self, BufRead, Write},
    time::{Duration, Instant},
};
use wordle_site::wordle::*;

fn main() {
    do_all().expect("should work");
}

fn do_all() -> io::Result<()> {
    write_ordered_allowed()?;
    write_default_state_data()?;
    Ok(())
}

fn write_default_state_data() -> io::Result<()> {
    let at = format!("{}{}", EMBED_DATA_DIRECTORY, DEFAULT_STATE_DATA_FILE_NAME);
    // open the file to contain the cached default state data
    let mut f = fs::File::create(&at)?;

    let (dur, out): (Duration, io::Result<()>) = timed(move || {
        // compute the data we should put into the file, and write it...
        for item in Solver::default().compute_top_k_guesses::<{ N_RECOMMENDATIONS }>() {
            writeln!(
                f,
                "{} {} {} {}",
                item.word, item.score.abs, item.score.expected_info, item.score.weight,
            )?;
        }
        Ok(())
    });
    out?;
    eprintln!(
        "done! wrote {} recommendations to {} in {:.2}s",
        N_RECOMMENDATIONS,
        at,
        dur.as_secs_f64()
    );
    Ok(())
}

fn write_ordered_allowed() -> io::Result<()> {
    let (dur, out) = timed(write_ordered_allowed_inner);
    let (name, lines) = out?;
    eprintln!(
        "done! wrote {} words to {} in {:.2}s",
        lines,
        name,
        dur.as_secs_f64()
    );
    Ok(())
}

fn write_ordered_allowed_inner() -> io::Result<(String, usize)> {
    let unordered = read_unordered_allowed_words()?;
    let ordered = read_ordered_frequency_data_words()?;
    let to_write = ordered_words(&unordered, &ordered).filter(|s| is_wordle_str(s));

    let at = format!(
        "{}{}",
        EMBED_DATA_DIRECTORY, ORDERED_ALLOWED_WORDS_FILE_NAME
    );
    let mut out = io::BufWriter::new(
        fs::File::options()
            .truncate(true)
            .create(true)
            .write(true)
            .open(&at)?,
    );

    let mut count = 0;
    for item in to_write {
        let compressed = CompressedWord::new(item);
        assert_eq!(compressed.to_string(), item);
        out.write_all(&compressed.as_bytes())?;
        count += 1;
    }

    Ok((at, count))
}

fn ordered_words<'a>(
    unordered: &'a [String],
    ordered: &'a [String],
) -> impl Iterator<Item = &'a str> + 'a {
    // we basically have to output the "unordered" list in the following order:
    // * a given word must be in the same position as it is in "ordered" (if it is contained in "ordered")
    // * all words in unordered not in ordered are emitted at the end in original order
    let unordered_s: HashSet<&str> = unordered.iter().map(|s| s.as_str()).collect();

    let ordered_s: HashSet<&str> = ordered.iter().map(|s| s.as_str()).collect();

    ordered
        .iter()
        .map(|s| s.as_str())
        .filter(move |item| unordered_s.contains(*item))
        .chain(
            unordered
                .iter()
                .filter(move |item| !ordered_s.contains(item.as_str()))
                .map(|v| v.as_str()),
        )
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
        .map(|l| l.map(|l| l.split(' ').next().map(normalize_wordle_word)))
        .filter_map(|l| match l {
            Ok(Some(v)) => Some(Ok(v)),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        })
        .collect()
}

fn timed<R, F>(f: F) -> (Duration, R)
where
    F: FnOnce() -> R,
{
    let start_at = Instant::now();
    let out = f();
    let dur = start_at.elapsed();

    (dur, out)
}
