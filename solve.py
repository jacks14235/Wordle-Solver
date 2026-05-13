from dataclasses import dataclass

import numpy as np
from numpy.typing import NDArray
import tqdm

from game import BLACK, GREEN, YELLOW, Game, toArray, toWord, toMatches

with open("words.txt", "r") as f:
    words_txt = [line.strip().lower() for line in f.readlines()]

N = len(words_txt)
# Each letter is converted to its corresponding 0-25 index
words = np.array([toArray(word) for word in words_txt], dtype=np.uint8)
# word_counts[word_idx, letter] stores how many times that letter appears in the word.
word_counts = np.stack(
    [np.count_nonzero(words == letter, axis=1) for letter in range(26)],
    axis=1,
).astype(np.uint8)
word_lookup = {tuple(int(letter) for letter in word): idx for idx, word in enumerate(words)}


@dataclass(frozen=True)
class HardModeState:
    """Accumulated constraints a hard-mode next guess must satisfy."""
    greens: tuple[int, int, int, int, int]
    min_counts: tuple[int, ...]
    forbidden: tuple[tuple[bool, bool, bool, bool, bool], ...]


def initial_hard_mode_state() -> HardModeState:
    """Create the empty hard-mode constraint state."""
    return HardModeState(
        greens=(-1, -1, -1, -1, -1),
        min_counts=(0,) * 26,
        forbidden=((False, False, False, False, False),) * 26,
    )


def update_hard_mode_state(
    state: HardModeState,
    guess: NDArray[np.uint8],
    match: NDArray[np.uint8],
) -> HardModeState:
    """Return the next hard-mode state after one hidden guess and score row."""
    greens = list(state.greens)
    min_counts = list(state.min_counts)
    forbidden = [list(row) for row in state.forbidden]

    for idx, tile in enumerate(match):
        letter = int(guess[idx])
        if tile == GREEN:
            greens[idx] = letter
        elif tile == YELLOW:
            forbidden[letter][idx] = True

    for letter in range(26):
        revealed_count = int(np.count_nonzero((guess == letter) & (match != BLACK)))
        min_counts[letter] = max(min_counts[letter], revealed_count)

    return HardModeState(
        greens=tuple(greens),
        min_counts=tuple(min_counts),
        forbidden=tuple(tuple(row) for row in forbidden),
    )


def hard_mode_guess_mask(state: HardModeState) -> NDArray[np.bool_]:
    """Return a mask of words that satisfy the current hard-mode constraints."""
    mask = np.ones(N, dtype=np.bool_)

    for idx, letter in enumerate(state.greens):
        if letter != -1:
            mask &= words[:, idx] == letter

    for letter, count in enumerate(state.min_counts):
        if count > 0:
            mask &= word_counts[:, letter] >= count

    for letter, forbidden_positions in enumerate(state.forbidden):
        for idx, is_forbidden in enumerate(forbidden_positions):
            if is_forbidden:
                mask &= words[:, idx] != letter

    return mask


def score_guesses_for_solution(
    guesses: NDArray[np.uint8],
    solution: NDArray[np.uint8],
    solution_counts: NDArray[np.uint8],
    result: NDArray[np.uint8],
) -> None:
    """Score many guesses against one solution, writing into result in place."""
    result.fill(BLACK)
    greens = guesses == solution
    result[greens] = GREEN

    for idx in range(5):
        letters = guesses[:, idx]
        solution_count = solution_counts[letters]
        green_count = np.count_nonzero(greens & (guesses == letters[:, None]), axis=1)
        remaining_count = solution_count - green_count
        used_before = np.count_nonzero((guesses[:, :idx] == letters[:, None]) & ~greens[:, :idx], axis=1)
        result[(~greens[:, idx]) & (remaining_count > used_before), idx] = YELLOW


def guess_word(word: NDArray[np.uint8], solution: NDArray[np.uint8], result: NDArray[np.uint8]) -> None:
    """Score one guess against one solution, writing into result in place."""
    # word = available[randint(0, N-1)]
    for (idx, letter) in enumerate(word):
        if (letter == solution[idx]):
            result[idx] = GREEN
        elif (letter in solution):
            solution_count = np.count_nonzero(solution == letter)
            used_count = np.count_nonzero(word[:idx] == letter)
            matches_after = np.count_nonzero(
                (word[idx+1:] == solution[idx+1:]) & (solution[idx+1:] == letter)
            )
            if solution_count > used_count + matches_after:
                result[idx] = YELLOW
            else:
                result[idx] = BLACK
        else:
            result[idx] = BLACK

def score_guess(
    guess: NDArray[np.uint8],
    solutions: NDArray[np.uint8],
    solution_counts: NDArray[np.uint8],
    result: NDArray[np.uint8],
) -> None:
    """Score one guess against many candidate solutions in a vectorized pass."""
    result.fill(BLACK)
    # Score all candidate solutions for one guess at once.
    greens = solutions == guess
    result[greens] = GREEN

    for idx, letter in enumerate(guess):
        is_green = greens[:, idx]
        solution_count = solution_counts[:, letter]
        green_count = np.count_nonzero(greens & (guess == letter), axis=1)
        remaining_count = solution_count - green_count
        # Earlier non-green copies of this letter consume the remaining yellow slots.
        used_before = np.count_nonzero((guess[:idx] == letter) & ~greens[:, :idx], axis=1)
        result[(~is_green) & (remaining_count > used_before), idx] = YELLOW

def filter_by_guess(guess: NDArray[np.uint8], match: NDArray[np.uint8]) -> NDArray[np.uint8]:
    """Return answers that would give a known match for a known guess."""
    result = np.empty_like(words)
    score_guess(guess, words, word_counts, result)
    available = words[np.all(result == match, axis=1)]
    print('down to ', len(available))
    return available

def get_matches(matches: Game | list[Game] | NDArray[np.uint8]) -> NDArray[np.uint8]:
    """Normalize games or raw match rows into an array shaped like (n, 5)."""
    if isinstance(matches, Game):
        return matches.matches

    if isinstance(matches, list) and all(isinstance(game, Game) for game in matches):
        match_arrays = [game.matches for game in matches if len(game.guesses) > 0]
        if len(match_arrays) == 0:
            return np.empty((0, 5), dtype=np.uint8)
        return np.vstack(match_arrays)

    match_array = np.asarray(matches, dtype=np.uint8)
    if match_array.ndim == 1:
        match_array = match_array.reshape(1, 5)

    return match_array


def get_games(games_or_matches: Game | list[Game] | NDArray[np.uint8]) -> list[Game]:
    """Normalize a game, list of games, or raw rows into a list of games."""
    if isinstance(games_or_matches, Game):
        return [games_or_matches]

    if isinstance(games_or_matches, list) and all(isinstance(game, Game) for game in games_or_matches):
        return games_or_matches

    game = Game()
    for match in get_matches(games_or_matches):
        game.add_guess(match)
    return [game]


def get_candidate_idxs(candidates: NDArray[np.uint8] | NDArray[np.bool_] | None) -> NDArray[np.int_]:
    """Normalize candidate words or a boolean mask into word-list indices."""
    if candidates is None:
        return np.arange(N)

    candidate_array = np.asarray(candidates)
    if candidate_array.dtype == np.bool_ and candidate_array.shape == (N,):
        return np.flatnonzero(candidate_array)

    if candidate_array.ndim == 1:
        candidate_array = candidate_array.reshape(1, 5)

    return np.array(
        [word_lookup[tuple(int(letter) for letter in word)] for word in candidate_array],
        dtype=np.int_,
    )


def match_exists(matches: Game | list[Game] | NDArray[np.uint8]) -> NDArray[np.uint8]:
    """Find answers where every observed row is possible for some hidden guess."""
    matches = get_matches(matches)

    if matches.ndim == 1:
        matches = matches.reshape(1, 5)

    possible = np.ones(N, dtype=np.bool_)

    for match in matches:
        # Keep only solutions where this score row could have come from some hidden guess.
        candidate_idxs = np.flatnonzero(possible)
        candidate_words = words[candidate_idxs]
        candidate_counts = word_counts[candidate_idxs]
        possible_for_match = np.zeros(len(candidate_idxs), dtype=np.bool_)
        result = np.empty_like(candidate_words)

        for guess in tqdm.tqdm(words, leave=False, desc='guesses'):
            score_guess(guess, candidate_words, candidate_counts, result)
            possible_for_match |= np.all(result == match, axis=1)
            # Once every candidate has at least one explaining guess, this row is done.
            if np.all(possible_for_match):
                break

        possible[candidate_idxs] = possible_for_match
        print('down to ', np.count_nonzero(possible))
        if not np.any(possible):
            break

    available = words[possible]
    print('down to ', len(available))
    return available


def hard_mode_game_possible(
    solution_idx: int,
    game: Game,
    state_mask_cache: dict[HardModeState, NDArray[np.int_]],
) -> bool:
    """Check whether one solution can explain one game under hard-mode rules."""
    states = {initial_hard_mode_state()}
    solution = words[solution_idx]
    solution_counts = word_counts[solution_idx]

    for match in game.matches:
        next_states: set[HardModeState] = set()

        for state in states:
            allowed_idxs = state_mask_cache.get(state)
            if allowed_idxs is None:
                allowed_idxs = np.flatnonzero(hard_mode_guess_mask(state))
                state_mask_cache[state] = allowed_idxs

            if len(allowed_idxs) == 0:
                continue

            guesses = words[allowed_idxs]
            result = np.empty_like(guesses)
            score_guesses_for_solution(guesses, solution, solution_counts, result)
            matching_guess_idxs = allowed_idxs[np.all(result == match, axis=1)]

            for guess_idx in matching_guess_idxs:
                next_states.add(update_hard_mode_state(state, words[guess_idx], match))

        if len(next_states) == 0:
            return False

        states = next_states

    return True


def hard_mode_match_exists(
    games_or_matches: Game | list[Game] | NDArray[np.uint8],
    candidates: NDArray[np.uint8] | NDArray[np.bool_] | None = None,
) -> NDArray[np.uint8]:
    """Find answers that can explain all games as sequential hard-mode play."""
    games = get_games(games_or_matches)
    if candidates is None:
        candidates = match_exists(games)

    candidate_idxs = get_candidate_idxs(candidates)
    possible_idxs = []
    state_mask_cache: dict[HardModeState, NDArray[np.int_]] = {}

    for solution_idx in tqdm.tqdm(candidate_idxs, desc='solutions'):
        if all(hard_mode_game_possible(int(solution_idx), game, state_mask_cache) for game in games):
            possible_idxs.append(solution_idx)

    available = words[np.array(possible_idxs, dtype=np.int_)]
    print('hard mode down to ', len(available))
    return available


def trial():
    """Placeholder for ad hoc experiments."""
    available = words.copy()

if __name__ == '__main__':
    from data_generator import generate_trials

    print(f"Loaded {N} words")
    print(words.shape)
    trials = generate_trials('raise', 10)

    for trial in trials:
        print(trial.toString(hidden=True))
        print(20*'='+'\n')

    possible_words = hard_mode_match_exists(toMatches(['BBBBB', 'BYBBB']))#trials)

    with open('possible-words.txt', 'w') as f:
        f.write('\n'.join(toWord(word) for word in possible_words))
        f.write('\n')
