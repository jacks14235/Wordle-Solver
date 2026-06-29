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
- `rust_version/`: Rust port of the solver, compiled to WASM for the browser.
- `react/`: Vite + React UI that runs the WASM solver in a Web Worker.
- `words.txt`: word list used for guesses and candidate answers (embedded into the WASM build).
- `words_all.txt`: larger word list kept for reference or experiments.
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

## React Site

The browser UI lives in `react/`. It runs the Rust solver through WASM inside a Web Worker so the page stays responsive while the search runs.

The app has two tabs:

- **Solver**: generate synthetic games or paste real shared grids, then run the solver and inspect surviving answers.
- **How it works**: background on the independent-row prefilter and the sequential hard-mode pass.

In **Generate** mode, you pick an answer, number of games, seed, and whether trial generation uses hard mode. In **Paste grids** mode, paste emoji rows like `⬛🟨🟩🟩⬛`, separated by blank lines or `====` between games.

### Build and run

The WASM package must be built before the React app can use an updated solver or word list. From the repo root:

```bash
cd rust_version
wasm-pack build --target web --release
```

Then start the React dev server:

```bash
cd ../react
npm install
npm run dev
```

`npm run dev` and `npm run build` automatically run `sync-wasm`, which copies the generated files from `rust_version/pkg/` into `react/src/wasm/`. If you change `words.txt`, rebuild WASM with `wasm-pack` and restart the dev server so the new dictionary is picked up.

Production build:

```bash
cd react
npm run build
npm run preview
```

See `rust_version/README.md` for the native Rust CLI and WASM API details.

### GitHub Pages

Pushing to GitHub does not publish the site by itself. This repo includes `.github/workflows/deploy-pages.yml`, which builds WASM + React on every push to `main` and deploys `react/dist` to GitHub Pages.

After the first successful deploy, the site will be available at:

```text
https://<your-github-username>.github.io/<repo-name>/
```

For example, if the repo is `wordle-stuff`:

```text
https://yourname.github.io/wordle-stuff/
```

Make sure `words.txt` is committed. It is embedded into the WASM build at compile time, so the Pages workflow needs that file in the repository.

## Notes

The hard-mode solver first uses the faster independent-row pass as a prefilter, then performs the more expensive sequential check on the survivors. This keeps the solver correct while avoiding deeper analysis for answers that cannot explain the score rows at all.

The React app uses the same two-stage flow. The dictionary size shown in the UI comes from `word_count()` in the embedded WASM build, not from reading `words.txt` at runtime.
