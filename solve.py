import numpy as np
from numpy.typing import NDArray
import tqdm

from game import BLACK, GREEN, YELLOW, Game, toArray, toWord

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


def guess_word(word: NDArray[np.uint8], solution: NDArray[np.uint8], result: NDArray[np.uint8]) -> None:
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
    result = np.empty_like(words)
    score_guess(guess, words, word_counts, result)
    available = words[np.all(result == match, axis=1)]
    print('down to ', len(available))
    return available

def get_matches(matches: Game | list[Game] | NDArray[np.uint8]) -> NDArray[np.uint8]:
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

def match_exists(matches: Game | list[Game] | NDArray[np.uint8]) -> NDArray[np.uint8]:
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


def trial():
    available = words.copy()

if __name__ == '__main__':
    from data_generator import generate_trials

    print(f"Loaded {N} words")
    print(words.shape)
    trials = generate_trials('rates', 5)

    possible_words = match_exists(trials)

    with open('possible-words.txt', 'w') as f:
        f.write('\n'.join(toWord(word) for word in possible_words))
        f.write('\n')
