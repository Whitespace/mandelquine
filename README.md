# mandelquine

A Ruby program whose source code is laid out in the shape of the Mandelbrot
set. Running it prints another program, shaped like the set zoomed in about
3.3%. After 120 runs the output is byte-identical to the original.

![One full cycle of the quine](mandelquine.svg)

## Run it

```bash
mise run quine   # or ./run.sh
```

It copies the quine to `./tmp` and feeds each generation's output back in as
the next program, forever. Ctrl-C to stop. It needs only bash and Ruby.
`mise install` gets the pinned Ruby; tested on 3.1, 3.2 and 3.3.

`mandelquine.cast` is an asciicast of one cycle: `asciinema play
mandelquine.cast`.

## How it works

- The file is `eval(s=%w( ... )*"")`. `%w` splits on whitespace and `*""`
  joins it back, so the code can be broken at any character and still run,
  and `s` holds the program's own source.
- Each run scores every cell of a 110x48 grid by smooth escape time, then
  fills exactly as many cells as the next program has characters: interior
  cells first, then the closest exterior cells.
- When the interior is larger than the program, the surplus is a trailing
  comment holding a second copy of the code, cut where its parentheses can
  be closed so `%w(` still terminates.
- The zoom is about the fixed point of the affine map that sends the whole
  set onto the period-3 minibrot on the real axis, scaled so generation 120
  frames the minibrot the way generation 0 frames the whole set. The counter
  is `(k+1)%120`, which is what makes the cycle close exactly.

## Rebuild

| Task | Does |
| --- | --- |
| `mise run build` | Builds `mandelquine.rb` from `src/body.rb`, the program before layout, and verifies that generation 120 equals generation 0. |
| `mise run cast` | Runs one cycle and writes `mandelquine.cast` and the animated `mandelquine.svg` shown above. |

## Viewer

`mise run viewer` (or `cargo run --release` in `viewer/`) opens a
[Ratatui](https://ratatui.rs) viewer. It needs a Rust toolchain, plus Ruby
only for the V check.

| Key | |
| --- | --- |
| W A S D | pan |
| Q / E | zoom out / in |
| L | quine language (Ruby only so far) |
| O | put the code inside or outside the set |
| B | background gradient on / off |
| P, Space | palette, toggle colour cycling |
| C, X | copy the frame (OSC 52), save it to `./mandelquine-<time>.rb` |
| V | run the frame with `ruby` and check it prints itself |
| R, ?, Esc | reset, help, quit |

Every frame is a program that prints itself, not the next zoom step. Its
first statements are the frame size, iteration count, centre and scale as
literals, plus a flag for inside/outside placement, so the view is the
program's own state. The viewer computes the
layout natively in Rust (`viewer/src/lang/ruby.rs`) using the same
arithmetic, in the same order, as the Ruby body (`viewer/src/lang/ruby.rb`),
which is what makes the two agree byte for byte. Scores are integer escape
counts over a 2x2 supersample rather than smooth escape time, so no `log`
is involved and the result doesn't depend on libm.

Unlike `mandelquine.rb`, frames are never padded with a comment. Interior
cells are ranked by distance to the set's edge, so when the interior has more
cells than the program has characters the code traces the outline and the
middle is left as whitespace.

A new language is a `Language` impl in `viewer/src/lang/` plus an entry in
`lang::all()`.
