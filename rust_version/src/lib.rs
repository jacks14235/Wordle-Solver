use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

const BLACK: u8 = 0;
const YELLOW: u8 = 1;
const GREEN: u8 = 2;
const WORD_LEN: usize = 5;
const ALPHABET: usize = 26;
const WORDS_TXT: &str = include_str!("../../words.txt");

type Word = [u8; WORD_LEN];
type Match = [u8; WORD_LEN];

#[derive(Clone)]
struct Dictionary {
    words: Vec<Word>,
    counts: Vec<[u8; ALPHABET]>,
    lookup: HashMap<u32, usize>,
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct HardModeState {
    greens: [i8; WORD_LEN],
    min_counts: [u8; ALPHABET],
    forbidden: [[bool; WORD_LEN]; ALPHABET],
}

#[derive(Clone)]
struct Game {
    matches: Vec<Match>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum MatchInput {
    Text(String),
    Tiles(Vec<u8>),
}

#[derive(Deserialize)]
struct GuessInput {
    #[serde(rename = "match")]
    match_row: MatchInput,
}

#[derive(Deserialize)]
struct GameInput {
    guesses: Vec<GuessInput>,
}

#[derive(Clone, Serialize)]
pub struct TrialGuess {
    pub word: String,
    #[serde(rename = "match")]
    pub match_text: String,
    pub remaining: usize,
}

#[derive(Clone, Serialize)]
pub struct TrialGame {
    pub answer: String,
    pub guesses: Vec<TrialGuess>,
}

#[derive(Serialize)]
struct ProgressEvent {
    stage: &'static str,
    current: usize,
    total: usize,
    remaining: usize,
    label: String,
}

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9e37_79b9_7f4a_7c15
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn gen_range(&mut self, upper: usize) -> usize {
        debug_assert!(upper > 0);
        (self.next_u64() as usize) % upper
    }
}

#[wasm_bindgen]
pub fn black_tile() -> u8 {
    BLACK
}

#[wasm_bindgen]
pub fn yellow_tile() -> u8 {
    YELLOW
}

#[wasm_bindgen]
pub fn green_tile() -> u8 {
    GREEN
}

#[wasm_bindgen]
pub fn word_count() -> usize {
    dictionary().words.len()
}

pub fn loaded_word_count() -> usize {
    dictionary().words.len()
}

pub fn generate_trials_native(
    answer: &str,
    n_trials: usize,
    seed: u64,
    hard_mode: bool,
) -> Result<Vec<TrialGame>, String> {
    let answer_word = parse_word(answer)?;
    let answer_idx = dictionary()
        .lookup
        .get(&word_key(&answer_word))
        .copied()
        .ok_or_else(|| format!("{answer:?} is not in words.txt"))?;
    let mut rng = XorShift64::new(seed);

    Ok((0..n_trials)
        .map(|_| run_trial(answer_idx, answer, &mut rng, hard_mode))
        .collect())
}

pub fn hard_mode_match_exists_for_trials(
    trials: &[TrialGame],
    verbose: bool,
) -> Result<Vec<String>, String> {
    let games = trials
        .iter()
        .map(|trial| {
            let matches = trial
                .guesses
                .iter()
                .map(|guess| parse_match_text(&guess.match_text))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<Game, String>(Game { matches })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let independent_candidates =
        independent_match_exists_with_options(&flatten_matches(&games), verbose, None)?;
    let possible = hard_mode_candidates(&games, &independent_candidates, None)?;

    Ok(possible
        .iter()
        .map(|idx| word_to_string(&dictionary().words[*idx]))
        .collect())
}

#[wasm_bindgen]
pub fn score_word(guess: &str, solution: &str) -> Result<String, JsValue> {
    let guess = parse_word(guess).map_err(js_error)?;
    let solution = parse_word(solution).map_err(js_error)?;
    let counts = count_letters(&solution);
    Ok(match_to_string(&score_match(&guess, &solution, &counts)))
}

#[wasm_bindgen]
pub fn parse_match(match_text: &str) -> Result<Vec<u8>, JsValue> {
    parse_match_text(match_text)
        .map(|row| row.to_vec())
        .map_err(js_error)
}

#[wasm_bindgen]
pub fn match_exists(observations: JsValue) -> Result<JsValue, JsValue> {
    let games = parse_games(observations)?;
    let rows = flatten_matches(&games);
    let possible = independent_match_exists(&rows);
    words_to_js(possible.iter().map(|idx| dictionary().words[*idx]))
}

#[wasm_bindgen]
pub fn hard_mode_match_exists(observations: JsValue) -> Result<JsValue, JsValue> {
    let games = parse_games(observations)?;
    let independent_candidates = independent_match_exists(&flatten_matches(&games));
    let possible = hard_mode_candidates(&games, &independent_candidates, None).map_err(js_error)?;
    words_to_js(possible.iter().map(|idx| dictionary().words[*idx]))
}

#[wasm_bindgen]
pub fn hard_mode_match_exists_with_progress(
    observations: JsValue,
    progress_callback: js_sys::Function,
) -> Result<JsValue, JsValue> {
    let games = parse_games(observations)?;
    let independent_candidates = independent_match_exists_with_options(
        &flatten_matches(&games),
        false,
        Some(&progress_callback),
    )
    .map_err(js_error)?;
    let possible = hard_mode_candidates(&games, &independent_candidates, Some(&progress_callback))
        .map_err(js_error)?;
    words_to_js(possible.iter().map(|idx| dictionary().words[*idx]))
}

#[wasm_bindgen]
pub fn hard_mode_match_exists_with_candidates(
    observations: JsValue,
    candidates: JsValue,
) -> Result<JsValue, JsValue> {
    let games = parse_games(observations)?;
    let candidate_words: Vec<String> = serde_wasm_bindgen::from_value(candidates)?;
    let candidate_idxs = candidate_words
        .iter()
        .map(|word| word_index(word))
        .collect::<Result<Vec<_>, _>>()
        .map_err(js_error)?;
    let possible = hard_mode_candidates(&games, &candidate_idxs, None).map_err(js_error)?;
    words_to_js(possible.iter().map(|idx| dictionary().words[*idx]))
}

#[wasm_bindgen]
pub fn generate_trials(
    answer: &str,
    n_trials: usize,
    seed: u64,
    hard_mode: bool,
) -> Result<JsValue, JsValue> {
    let trials = generate_trials_native(answer, n_trials, seed, hard_mode).map_err(js_error)?;
    serde_wasm_bindgen::to_value(&trials).map_err(JsValue::from)
}

fn dictionary() -> &'static Dictionary {
    static DICTIONARY: OnceLock<Dictionary> = OnceLock::new();
    DICTIONARY.get_or_init(Dictionary::load)
}

impl Dictionary {
    fn load() -> Self {
        let mut words = Vec::new();
        let mut counts = Vec::new();
        let mut lookup = HashMap::new();

        for line in WORDS_TXT.lines() {
            let word = line.trim();
            if word.is_empty() {
                continue;
            }

            let letters = parse_word(word).expect("words.txt must contain five-letter ASCII words");
            lookup.insert(word_key(&letters), words.len());
            counts.push(count_letters(&letters));
            words.push(letters);
        }

        Self {
            words,
            counts,
            lookup,
        }
    }
}

fn parse_word(word: &str) -> Result<Word, String> {
    let normalized = word.trim().to_ascii_lowercase();
    let bytes = normalized.as_bytes();
    if bytes.len() != WORD_LEN {
        return Err(format!("expected a five-letter word, got {word:?}"));
    }

    let mut letters = [0; WORD_LEN];
    for (idx, byte) in bytes.iter().enumerate() {
        if !byte.is_ascii_lowercase() {
            return Err(format!(
                "word contains a non-lowercase ASCII letter: {word:?}"
            ));
        }
        letters[idx] = byte - b'a';
    }

    Ok(letters)
}

fn word_to_string(word: &Word) -> String {
    word.iter()
        .map(|letter| char::from(b'a' + letter))
        .collect()
}

fn word_key(word: &Word) -> u32 {
    word.iter().fold(0_u32, |acc, letter| {
        acc * ALPHABET as u32 + u32::from(*letter)
    })
}

fn word_index(word: &str) -> Result<usize, String> {
    let letters = parse_word(word)?;
    dictionary()
        .lookup
        .get(&word_key(&letters))
        .copied()
        .ok_or_else(|| format!("{word:?} is not in words.txt"))
}

fn count_letters(word: &Word) -> [u8; ALPHABET] {
    let mut counts = [0; ALPHABET];
    for letter in word {
        counts[*letter as usize] += 1;
    }
    counts
}

fn score_match(guess: &Word, solution: &Word, solution_counts: &[u8; ALPHABET]) -> Match {
    let mut result = [BLACK; WORD_LEN];
    let mut green_counts = [0; ALPHABET];

    for idx in 0..WORD_LEN {
        if guess[idx] == solution[idx] {
            result[idx] = GREEN;
            green_counts[guess[idx] as usize] += 1;
        }
    }

    for idx in 0..WORD_LEN {
        if result[idx] == GREEN {
            continue;
        }

        let letter = guess[idx];
        let letter_idx = letter as usize;
        let remaining_count = solution_counts[letter_idx].saturating_sub(green_counts[letter_idx]);
        let used_before = guess[..idx]
            .iter()
            .enumerate()
            .filter(|(prev_idx, prev_letter)| **prev_letter == letter && result[*prev_idx] != GREEN)
            .count() as u8;

        if remaining_count > used_before {
            result[idx] = YELLOW;
        }
    }

    result
}

fn score_code(guess: &Word, solution: &Word, solution_counts: &[u8; ALPHABET]) -> u8 {
    match_code(&score_match(guess, solution, solution_counts))
}

fn match_code(row: &Match) -> u8 {
    row.iter().fold(0, |acc, tile| acc * 3 + tile)
}

fn parse_match_text(match_text: &str) -> Result<Match, String> {
    let mut row = [BLACK; WORD_LEN];
    let mut len = 0;

    for (idx, tile) in match_text.trim().chars().enumerate() {
        if idx >= WORD_LEN {
            return Err(format!("expected five tiles, got {match_text:?}"));
        }

        row[idx] = match tile {
            '⬛' | 'B' | 'b' => BLACK,
            '🟨' | 'Y' | 'y' => YELLOW,
            '🟩' | 'G' | 'g' => GREEN,
            _ => return Err(format!("unknown tile {tile:?} in {match_text:?}")),
        };
        len += 1;
    }

    if len != WORD_LEN {
        return Err(format!("expected five tiles, got {match_text:?}"));
    }

    Ok(row)
}

fn parse_match_input(input: MatchInput) -> Result<Match, String> {
    match input {
        MatchInput::Text(text) => parse_match_text(&text),
        MatchInput::Tiles(tiles) => {
            if tiles.len() != WORD_LEN || tiles.iter().any(|tile| *tile > GREEN) {
                return Err(format!("expected five tile values in 0..=2, got {tiles:?}"));
            }

            let mut row = [BLACK; WORD_LEN];
            row.copy_from_slice(&tiles);
            Ok(row)
        }
    }
}

fn parse_games(value: JsValue) -> Result<Vec<Game>, JsValue> {
    if let Ok(games) = serde_wasm_bindgen::from_value::<Vec<GameInput>>(value.clone()) {
        return games
            .into_iter()
            .map(|game| {
                let matches = game
                    .guesses
                    .into_iter()
                    .map(|guess| parse_match_input(guess.match_row))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok::<Game, String>(Game { matches })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(js_error);
    }

    if let Ok(game) = serde_wasm_bindgen::from_value::<GameInput>(value.clone()) {
        let matches = game
            .guesses
            .into_iter()
            .map(|guess| parse_match_input(guess.match_row))
            .collect::<Result<Vec<_>, _>>()
            .map_err(js_error)?;
        return Ok(vec![Game { matches }]);
    }

    if let Ok(rows) = serde_wasm_bindgen::from_value::<Vec<MatchInput>>(value.clone()) {
        let matches = rows
            .into_iter()
            .map(parse_match_input)
            .collect::<Result<Vec<_>, _>>()
            .map_err(js_error)?;
        return Ok(vec![Game { matches }]);
    }

    let row = serde_wasm_bindgen::from_value::<MatchInput>(value)
        .map_err(|_| js_error("expected a match row, list of rows, game, or list of games"))?;
    Ok(vec![Game {
        matches: vec![parse_match_input(row).map_err(js_error)?],
    }])
}

fn flatten_matches(games: &[Game]) -> Vec<Match> {
    games
        .iter()
        .flat_map(|game| game.matches.iter().copied())
        .collect()
}

fn independent_match_exists(matches: &[Match]) -> Vec<usize> {
    independent_match_exists_with_options(matches, false, None)
        .expect("no progress callback is present")
}

fn independent_match_exists_with_options(
    matches: &[Match],
    verbose: bool,
    progress_callback: Option<&js_sys::Function>,
) -> Result<Vec<usize>, String> {
    let dict = dictionary();
    let mut possible = vec![true; dict.words.len()];
    let total = matches.len();

    for (row_idx, row) in matches.iter().enumerate() {
        let target = match_code(row);
        let candidate_idxs = possible
            .iter()
            .enumerate()
            .filter_map(|(idx, is_possible)| is_possible.then_some(idx))
            .collect::<Vec<_>>();
        let mut possible_for_match = vec![false; candidate_idxs.len()];
        let mut remaining = candidate_idxs.len();

        for guess in &dict.words {
            if remaining == 0 {
                break;
            }

            for (pos, solution_idx) in candidate_idxs.iter().enumerate() {
                if possible_for_match[pos] {
                    continue;
                }

                let code = score_code(
                    guess,
                    &dict.words[*solution_idx],
                    &dict.counts[*solution_idx],
                );
                if code == target {
                    possible_for_match[pos] = true;
                    remaining -= 1;
                }
            }
        }

        for (pos, solution_idx) in candidate_idxs.iter().enumerate() {
            possible[*solution_idx] = possible_for_match[pos];
        }

        if verbose {
            println!(
                "down to {}",
                possible.iter().filter(|is_possible| **is_possible).count()
            );
        }

        let remaining_count = possible.iter().filter(|is_possible| **is_possible).count();
        call_progress(
            progress_callback,
            ProgressEvent {
                stage: "prefilter",
                current: row_idx + 1,
                total,
                remaining: remaining_count,
                label: format!("Independent row {} of {}", row_idx + 1, total),
            },
        )?;

        if possible.iter().all(|is_possible| !is_possible) {
            break;
        }
    }

    let available = possible
        .into_iter()
        .enumerate()
        .filter_map(|(idx, is_possible)| is_possible.then_some(idx))
        .collect::<Vec<_>>();

    if verbose {
        println!("down to {}", available.len());
    }

    Ok(available)
}

fn initial_hard_mode_state() -> HardModeState {
    HardModeState {
        greens: [-1; WORD_LEN],
        min_counts: [0; ALPHABET],
        forbidden: [[false; WORD_LEN]; ALPHABET],
    }
}

fn update_hard_mode_state(state: &HardModeState, guess: &Word, row: &Match) -> HardModeState {
    let mut next = state.clone();

    for idx in 0..WORD_LEN {
        let letter = guess[idx] as usize;
        match row[idx] {
            GREEN => next.greens[idx] = guess[idx] as i8,
            YELLOW => next.forbidden[letter][idx] = true,
            _ => {}
        }
    }

    for letter in 0..ALPHABET {
        let revealed_count = (0..WORD_LEN)
            .filter(|idx| guess[*idx] as usize == letter && row[*idx] != BLACK)
            .count() as u8;
        next.min_counts[letter] = next.min_counts[letter].max(revealed_count);
    }

    next
}

fn hard_mode_guess_idxs(state: &HardModeState) -> Vec<usize> {
    let dict = dictionary();
    let mut allowed = Vec::with_capacity(dict.words.len());

    'word_loop: for (word_idx, word) in dict.words.iter().enumerate() {
        for idx in 0..WORD_LEN {
            if state.greens[idx] != -1 && word[idx] != state.greens[idx] as u8 {
                continue 'word_loop;
            }
        }

        for letter in 0..ALPHABET {
            if dict.counts[word_idx][letter] < state.min_counts[letter] {
                continue 'word_loop;
            }

            for idx in 0..WORD_LEN {
                if state.forbidden[letter][idx] && word[idx] as usize == letter {
                    continue 'word_loop;
                }
            }
        }

        allowed.push(word_idx);
    }

    allowed
}

fn hard_mode_game_possible(
    solution_idx: usize,
    game: &Game,
    state_mask_cache: &mut HashMap<HardModeState, Vec<usize>>,
) -> bool {
    let dict = dictionary();
    let solution = &dict.words[solution_idx];
    let solution_counts = &dict.counts[solution_idx];
    let mut states = HashSet::from([initial_hard_mode_state()]);

    for row in &game.matches {
        let target = match_code(row);
        let mut next_states = HashSet::new();

        for state in states {
            let allowed_idxs = state_mask_cache
                .entry(state.clone())
                .or_insert_with(|| hard_mode_guess_idxs(&state));

            for guess_idx in allowed_idxs.iter().copied() {
                let guess = &dict.words[guess_idx];
                if score_code(guess, solution, solution_counts) == target {
                    next_states.insert(update_hard_mode_state(&state, guess, row));
                }
            }
        }

        if next_states.is_empty() {
            return false;
        }

        states = next_states;
    }

    true
}

fn hard_mode_candidates(
    games: &[Game],
    candidate_idxs: &[usize],
    progress_callback: Option<&js_sys::Function>,
) -> Result<Vec<usize>, String> {
    let mut possible = Vec::new();
    let mut state_mask_cache = HashMap::new();

    for (checked, solution_idx) in candidate_idxs.iter().enumerate() {
        if games
            .iter()
            .all(|game| hard_mode_game_possible(*solution_idx, game, &mut state_mask_cache))
        {
            possible.push(*solution_idx);
        }

        if progress_callback.is_some() && (checked + 1 == candidate_idxs.len() || checked % 25 == 0)
        {
            call_progress(
                progress_callback,
                ProgressEvent {
                    stage: "hard-mode",
                    current: checked + 1,
                    total: candidate_idxs.len(),
                    remaining: possible.len(),
                    label: format!(
                        "Hard-mode candidate {} of {}",
                        checked + 1,
                        candidate_idxs.len()
                    ),
                },
            )?;
        }
    }

    Ok(possible)
}

fn run_trial(
    answer_idx: usize,
    answer_word: &str,
    rng: &mut XorShift64,
    hard_mode: bool,
) -> TrialGame {
    let dict = dictionary();
    let answer = dict.words[answer_idx];
    let answer_counts = dict.counts[answer_idx];
    let mut candidate_idxs = (0..dict.words.len()).collect::<Vec<_>>();
    let mut hard_mode_state = initial_hard_mode_state();
    let mut guessed = HashSet::new();
    let mut guesses = Vec::new();

    for _ in 0..6 {
        if candidate_idxs.is_empty() {
            break;
        }

        let guess_idx = if candidate_idxs.len() == 1 {
            candidate_idxs[0]
        } else {
            let mut guess_pool = if hard_mode {
                hard_mode_guess_idxs(&hard_mode_state)
            } else {
                (0..dict.words.len()).collect()
            };
            guess_pool.retain(|idx| !guessed.contains(idx));
            if guess_pool.is_empty() {
                break;
            }
            guess_pool[rng.gen_range(guess_pool.len())]
        };
        guessed.insert(guess_idx);
        let guess = dict.words[guess_idx];
        let row = score_match(&guess, &answer, &answer_counts);
        let target = match_code(&row);
        candidate_idxs.retain(|candidate_idx| {
            score_code(
                &guess,
                &dict.words[*candidate_idx],
                &dict.counts[*candidate_idx],
            ) == target
        });

        guesses.push(TrialGuess {
            word: word_to_string(&guess),
            match_text: match_to_string(&row),
            remaining: candidate_idxs.len(),
        });
        hard_mode_state = update_hard_mode_state(&hard_mode_state, &guess, &row);

        if guess == answer {
            break;
        }
    }

    TrialGame {
        answer: answer_word.to_ascii_lowercase(),
        guesses,
    }
}

fn match_to_string(row: &Match) -> String {
    row.iter()
        .map(|tile| match *tile {
            BLACK => 'B',
            YELLOW => 'Y',
            GREEN => 'G',
            _ => '?',
        })
        .collect()
}

fn words_to_js(words: impl Iterator<Item = Word>) -> Result<JsValue, JsValue> {
    let words = words.map(|word| word_to_string(&word)).collect::<Vec<_>>();
    serde_wasm_bindgen::to_value(&words).map_err(JsValue::from)
}

fn call_progress(
    progress_callback: Option<&js_sys::Function>,
    event: ProgressEvent,
) -> Result<(), String> {
    if let Some(callback) = progress_callback {
        let value = serde_wasm_bindgen::to_value(&event).map_err(|err| err.to_string())?;
        callback.call1(&JsValue::NULL, &value).map_err(|err| {
            err.as_string()
                .unwrap_or_else(|| "progress callback failed".to_string())
        })?;
    }

    Ok(())
}

fn js_error(message: impl ToString) -> JsValue {
    JsValue::from_str(&message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score(guess: &str, solution: &str) -> String {
        let guess = parse_word(guess).unwrap();
        let solution = parse_word(solution).unwrap();
        let counts = count_letters(&solution);
        match_to_string(&score_match(&guess, &solution, &counts))
    }

    #[test]
    fn scores_basic_wordle_rows() {
        assert_eq!(score("party", "tardy"), "BGGYG");
    }

    #[test]
    fn scores_duplicate_letters_like_wordle() {
        assert_eq!(score("allee", "belle"), "BYGYG");
    }
}
