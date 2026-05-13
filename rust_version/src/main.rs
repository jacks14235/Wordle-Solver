use std::env;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use wordle_stuff::{generate_trials_native, hard_mode_match_exists_for_trials, loaded_word_count};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let positional_args = positional_args(&args);
    let answer = positional_args
        .first()
        .map(String::as_str)
        .unwrap_or("fjord");
    let n_trials = positional_args
        .get(1)
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(10);
    let hard_mode = !args.iter().any(|arg| arg == "--easy-mode");
    let verbose = args.iter().any(|arg| arg == "--verbose" || arg == "-v");
    let seed = seed_from_args(&args)?;

    println!("Loaded {} words", loaded_word_count());
    println!("({},{})", loaded_word_count(), 5);

    let trials = generate_trials_native(answer, n_trials, seed, hard_mode)?;
    for trial in &trials {
        print_trial(trial);
        println!("====================\n");
    }

    let possible_words = hard_mode_match_exists_for_trials(&trials, verbose)?;
    println!("hard mode down to {}", possible_words.len());

    let output_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("possible-words.txt");
    fs::write(&output_path, format!("{}\n", possible_words.join("\n")))?;
    println!("wrote {}", output_path.display());

    Ok(())
}

fn positional_args(args: &[String]) -> Vec<String> {
    let mut positionals = Vec::new();
    let mut skip_next = false;

    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }

        match arg.as_str() {
            "--seed" => skip_next = true,
            "--easy-mode" | "--verbose" | "-v" => {}
            _ => positionals.push(arg.clone()),
        }
    }

    positionals
}

fn seed_from_args(args: &[String]) -> Result<u64, Box<dyn std::error::Error>> {
    if let Some(seed_flag_idx) = args.iter().position(|arg| arg == "--seed") {
        let seed = args
            .get(seed_flag_idx + 1)
            .ok_or("--seed requires a numeric value")?
            .parse::<u64>()?;
        return Ok(seed);
    }

    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64)
}

fn print_trial(trial: &wordle_stuff::TrialGame) {
    for guess in &trial.guesses {
        println!(
            "{} {} remaining={}",
            guess.word, guess.match_text, guess.remaining
        );
    }
}
