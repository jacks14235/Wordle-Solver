# Wordle Stuff

Tools for exploring how much information is leaked by hidden Wordle score grids when the actual guesses are not known.

The code represents words as NumPy `uint8` arrays where `a=0`, `b=1`, and so on. Tile results are encoded as:

- `B` / `⬛` = black / absent
- `Y` / `🟨` = yellow / present in another position
- `G` / `🟩` = green / correct position

## Files

- `game.py`: shared `Game` and `Guess` classes, word/tile conversion helpers, and terminal rendering.
- `data_generator.py`: generates synthetic games for a known answer using random guesses.
- `solve.py`: filters possible answers from observed score rows, including a deeper hard-mode solver.
- `words.txt`: word list used for guesses and candidate answers.
- `possible-words.txt`: output file written by `solve.py`.

## Generate Synthetic Data

Generate random games for a known answer:

```bash
python data_generator.py party 3
```

Use a deterministic seed:

```bash
python data_generator.py party 3 --seed 1
```

By default, generated games use Wordle hard mode: each next guess must satisfy all information revealed by previous rows. To generate unrestricted random guesses instead:

```bash
python data_generator.py party 3 --easy-mode
```

## Solve From Hidden Score Rows

Run the current experiment:

```bash
python solve.py
```

`solve.py` currently generates sample games, prints them, runs the hard-mode solver, and writes surviving answers to:

```text
possible-words.txt
```

The main solver APIs are:

```python
match_exists(games_or_matches)
hard_mode_match_exists(games_or_matches, candidates=None)
```

`match_exists` is the fast independent-row pass. It asks whether each observed score row could have come from some unknown guess against a candidate answer.

`hard_mode_match_exists` is the deeper sequential pass. It preserves game boundaries and checks whether each candidate answer can explain each game as a sequence of hard-mode-legal hidden guesses.

## Rendering Games

`Game.toString()` prints colored terminal letters:

```python
print(game.toString())
```

Hide the guess letters and print only score emojis:

```python
print(game.toString(hidden=True))
```

## Notes

The hard-mode solver first uses the faster independent-row pass as a prefilter, then performs the more expensive sequential check on the survivors. This keeps the solver correct while avoiding deeper analysis for answers that cannot explain the score rows at all.
