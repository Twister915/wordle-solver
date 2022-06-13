use std::fs::File;
use std::io::Write;
use std::time::Instant;
use wordle_site::wordle::{DATA_DIRECTORY, DEFAULT_STATE_DATA_FILE_NAME, N_RECOMMENDATIONS};

fn main() {
    let at = format!("{}{}", DATA_DIRECTORY, DEFAULT_STATE_DATA_FILE_NAME);
    let mut f = File::options()
        .truncate(true)
        .create(true)
        .write(true)
        .open(&at)
        .expect("should open");

    let start_at = Instant::now();
    wordle_site::wordle::Solver::default()
        .top_k_guesses_real::<{N_RECOMMENDATIONS}>()
        .for_each(|item| {
            let line = format!("{} {} {} {}\n", item.word, item.score.abs, item.score.expected_info, item.score.weight);
            f.write_all(line.as_bytes()).expect("should write OK");
        });

    let dur = start_at.elapsed();
    eprintln!("done! wrote {} recommendations to {} in {:.2}s", N_RECOMMENDATIONS, at, dur.as_secs_f64());
}
