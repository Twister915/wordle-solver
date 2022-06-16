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

use std::fs::File;
use std::io::Write;
use std::time::Instant;
use wordle_site::wordle::{DATA_DIRECTORY, DEFAULT_STATE_DATA_FILE_NAME, N_RECOMMENDATIONS};

fn main() {
    let at = format!("{}{}", DATA_DIRECTORY, DEFAULT_STATE_DATA_FILE_NAME);
    // open the file to contain the cached default state data
    let mut f = File::options()
        .truncate(true)
        .create(true)
        .write(true)
        .open(&at)
        .expect("should open");

    let start_at = Instant::now();
    // compute the data we should put into the file, and write it...
    wordle_site::wordle::Solver::default()
        .compute_top_k_guesses::<{N_RECOMMENDATIONS}>()
        .for_each(|item| {
            // space seperated, according to the expected format documented in wordle/data.rs
            let line = format!(
                "{} {} {} {}\n",
                item.word,
                item.score.abs,
                item.score.expected_info,
                item.score.weight,
            );
            f.write_all(line.as_bytes()).expect("should write OK");
        });

    let dur = start_at.elapsed();
    eprintln!("done! wrote {} recommendations to {} in {:.2}s", N_RECOMMENDATIONS, at, dur.as_secs_f64());
}
