# Rust / WASM Version

This crate ports the Python solver to Rust so it can run in a browser through
`wasm-bindgen`. It embeds `../words.txt` at compile time, so the browser does
not need to fetch the word list separately.

## Build

Run the terminal equivalent of `solve.py`'s main block:

```bash
cargo run --release
```

By default this generates 10 hard-mode trials for `fjord`, prints them, solves
the hidden score rows, and writes `../possible-words.txt`. You can override the
defaults:

```bash
cargo run --release -- party 10 --seed 1
cargo run --release -- party 10 --easy-mode
cargo run --release -- party 10 --verbose
```

Install `wasm-pack` if needed:

```bash
cargo install wasm-pack
```

Build a web package:

```bash
wasm-pack build --target web --release
```

The generated browser package is written to `pkg/`.

## Browser API

```js
import init, {
  score_word,
  match_exists,
  hard_mode_match_exists,
  generate_trials,
} from "./pkg/wordle_stuff.js";

await init();

console.log(score_word("party", "tardy")); // "BGGYG"

const games = generate_trials("party", 10, 1n, true);
const possible = hard_mode_match_exists(games);
console.log(possible);
```

Inputs can be score strings like `"BYGBG"`, tile arrays like `[0, 1, 2, 0, 1]`,
a single game object, or an array of game objects:

```js
hard_mode_match_exists([
  {
    guesses: [
      { match: "BBYGB" },
      { match: "YBGGB" },
    ],
  },
]);
```

Tile values match the Python version:

- `0`: black / absent
- `1`: yellow / present in another position
- `2`: green / correct position
