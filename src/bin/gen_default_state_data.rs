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
