import argparse

import numpy as np
from numpy.typing import NDArray

from game import Game, toArray, toWord
from solve import (
    hard_mode_guess_mask,
    initial_hard_mode_state,
    score_guess,
    update_hard_mode_state,
    word_counts,
    words,
)

MAX_GUESSES = 6


def filter_candidates(
    guess: NDArray[np.uint8],
    match: NDArray[np.uint8],
    candidate_idxs: NDArray[np.int_],
) -> NDArray[np.int_]:
    """Keep candidate answers that would produce this match for the guess."""
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
    """Simulate one random game against a fixed answer."""
    candidate_idxs = np.arange(len(words))
    hard_mode_state = initial_hard_mode_state()
    game = Game(answer=answer_word)
    match = np.empty(5, dtype=np.uint8)
    answer_counts = np.array(
        [[np.count_nonzero(answer == letter) for letter in range(26)]],
        dtype=np.uint8,
    )

    for _ in range(MAX_GUESSES):
        if len(candidate_idxs) == 0:
            break

        guess_pool = np.flatnonzero(hard_mode_guess_mask(hard_mode_state)) if hard_mode else np.arange(len(words))
        if len(guess_pool) == 0:
            break

        guess_idx = rng.choice(guess_pool)
        guess = words[guess_idx]

        score_guess(guess, answer.reshape(1, 5), answer_counts, match.reshape(1, 5))
        candidate_idxs = filter_candidates(guess, match, candidate_idxs)
        game.add_guess(match, word=toWord(guess), remaining=len(candidate_idxs))
        hard_mode_state = update_hard_mode_state(hard_mode_state, guess, match)

        if np.array_equal(guess, answer):
            break

    return game


def generate_trials(
    answer_word: str,
    n_trials: int,
    seed: int | None = None,
    hard_mode: bool = True,
) -> list[Game]:
    """Generate synthetic games for one answer word."""
    answer = toArray(answer_word)
    if not np.any(np.all(words == answer, axis=1)):
        raise ValueError(f'{answer_word!r} is not in words.txt')

    rng = np.random.default_rng(seed)
    return [run_trial(answer, answer_word, rng, hard_mode) for _ in range(n_trials)]


def main() -> None:
    """CLI entrypoint for printing generated synthetic games."""
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
