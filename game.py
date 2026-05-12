from dataclasses import dataclass, field

import numpy as np
from numpy.typing import NDArray

BLACK = 0
YELLOW = 1
GREEN = 2
RESET = '\033[0m'
WHITE = '\033[97m'
YELLOW_TEXT = '\033[33m'
GREEN_TEXT = '\033[32m'


def toArray(word: str) -> NDArray[np.uint8]:
    w = word.lower()
    arr = [ord(i) - 97 for i in w]
    return np.array(arr, dtype=np.uint8)


def toWord(word: NDArray[np.uint8]) -> str:
    return ''.join(chr(letter + 97) for letter in word)


def toMatch(match: str) -> NDArray[np.uint8]:
    tiles = {
        '⬛': BLACK,
        'B': BLACK,
        'b': BLACK,
        '🟨': YELLOW,
        'Y': YELLOW,
        'y': YELLOW,
        '🟩': GREEN,
        'G': GREEN,
        'g': GREEN,
    }
    return np.array([tiles[tile] for tile in match], dtype=np.uint8)


def toMatches(matches: list[str]) -> NDArray[np.uint8]:
    return np.array([toMatch(match) for match in matches], dtype=np.uint8)


def match_to_string(match: NDArray[np.uint8]) -> str:
    tiles = {
        BLACK: 'B',
        YELLOW: 'Y',
        GREEN: 'G',
    }
    return ''.join(tiles[tile] for tile in match)


@dataclass
class Guess:
    match: NDArray[np.uint8]
    word: str | None = None
    remaining: int | None = None

    def __post_init__(self) -> None:
        self.match = np.asarray(self.match, dtype=np.uint8)
        if self.match.shape != (5,):
            raise ValueError(f'Expected match shape (5,), got {self.match.shape}')

    @property
    def match_text(self) -> str:
        return match_to_string(self.match)


@dataclass
class Game:
    answer: str | None = None
    guesses: list[Guess] = field(default_factory=list)

    def add_guess(
        self,
        match: str | NDArray[np.uint8],
        word: str | None = None,
        remaining: int | None = None,
    ) -> None:
        match_array = toMatch(match) if isinstance(match, str) else match
        self.guesses.append(Guess(match_array.copy(), word, remaining))

    @property
    def matches(self) -> NDArray[np.uint8]:
        if not self.guesses:
            return np.empty((0, 5), dtype=np.uint8)
        return np.array([guess.match for guess in self.guesses], dtype=np.uint8)

    def toString(self, hidden: bool = False) -> str:
        colors = {
            BLACK: WHITE,
            YELLOW: YELLOW_TEXT,
            GREEN: GREEN_TEXT,
        }
        emojis = {
            BLACK: '⬛',
            YELLOW: '🟨',
            GREEN: '🟩',
        }
        lines = []

        for guess in self.guesses:
            if hidden:
                lines.append(''.join(emojis[tile] for tile in guess.match))
                continue

            word = guess.word if guess.word is not None else '?????'
            letters = [
                f'{colors[tile]}{letter}{RESET}'
                for letter, tile in zip(word.upper(), guess.match)
            ]
            lines.append(''.join(letters))

        return '\n'.join(lines)

    def __str__(self) -> str:
        return self.toString()
