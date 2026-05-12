import argparse

import numpy as np
from numpy.typing import NDArray

from game import Game, toArray, toWord
from solve import score_guess, word_counts, words

MAX_GUESSES = 6


def filter_candidates(
    guess: NDArray[np.uint8],
    match: NDArray[np.uint8],
    candidate_idxs: NDArray[np.int_],
) -> NDArray[np.int_]:
    candidate_words = words[candidate_idxs]
    candidate_counts = word_counts[candidate_idxs]
    result = np.empty_like(candidate_words)

    score_guess(guess, candidate_words, candidate_counts, result)
    return candidate_idxs[np.all(result == match, axis=1)]


def run_trial(
    answer: NDArray[np.uint8],
    answer_word: str,
    rng: np.random.Generator,
    hard_mode: bool = True,
) -> Game:
    candidate_idxs = np.arange(len(words))
    game = Game(answer=answer_word)
    match = np.empty(5, dtype=np.uint8)
    answer_counts = np.array(
        [[np.count_nonzero(answer == letter) for letter in range(26)]],
        dtype=np.uint8,
    )

    for _ in range(MAX_GUESSES):
        if len(candidate_idxs) == 0:
            break

        guess_pool = candidate_idxs if hard_mode else np.arange(len(words))
        guess_idx = rng.choice(guess_pool)
        guess = words[guess_idx]

        score_guess(guess, answer.reshape(1, 5), answer_counts, match.reshape(1, 5))
        candidate_idxs = filter_candidates(guess, match, candidate_idxs)
        game.add_guess(match, word=toWord(guess), remaining=len(candidate_idxs))

        if np.array_equal(guess, answer):
            break

    return game


def generate_trials(
    answer_word: str,
    n_trials: int,
    seed: int | None = None,
    hard_mode: bool = True,
) -> list[Game]:
    answer = toArray(answer_word)
    if not np.any(np.all(words == answer, axis=1)):
        raise ValueError(f'{answer_word!r} is not in words.txt')

    rng = np.random.default_rng(seed)
    return [run_trial(answer, answer_word, rng, hard_mode) for _ in range(n_trials)]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument('answer')
    parser.add_argument('n_trials', type=int)
    parser.add_argument('--seed', type=int, default=None)
    parser.add_argument('--easy-mode', action='store_true')
    args = parser.parse_args()

    trials = generate_trials(args.answer, args.n_trials, args.seed, hard_mode=not args.easy_mode)
    for trial_idx, trial in enumerate(trials, start=1):
        print(f'Trial {trial_idx}')
        for guess_idx, guess in enumerate(trial.guesses, start=1):
            print(f'{guess_idx}: {guess.word} {guess.match_text} remaining={guess.remaining}')


if __name__ == '__main__':
    main()
