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
[Ratatui](https://ratatui.rs) viewer. It needs a Rust toolchain. A language's
own interpreter or compiler is only needed for the V check.

| Key | |
| --- | --- |
| W A S D | pan |
| Q / E | zoom out / in |
| L | quine language, see [Languages](#languages) |
| O | put the code inside or outside the set |
| B | background gradient on / off |
| P, Space | palette, toggle colour cycling |
| C, X | copy the frame (OSC 52), save it to `./mandelquine-<time>.<ext>` |
| V | run the frame with its language's interpreter or compiler and check it prints itself |
| R, ?, Esc | reset, help, quit |

Every frame is a program that prints itself, not the next zoom step. Its
first statements are the frame size, iteration count, centre and scale as
literals, plus a flag for inside/outside placement, so the view is the
program's own state. The viewer computes the
layout natively in Rust (`viewer/src/lang/layout.rs`) using the same
arithmetic, in the same order, as each language's body (for example
`viewer/src/lang/ruby.rb`), which is what makes them agree byte for byte. Scores are integer escape
counts over a 2x2 supersample rather than smooth escape time, so no `log`
is involved and the result doesn't depend on libm.

Unlike `mandelquine.rb`, frames are never padded with a comment. Interior
cells are ranked by distance to the set's edge, so when the interior has more
cells than the program has characters the code traces the outline and the
middle is left as whitespace.

A new language is a `Language` impl in `viewer/src/lang/<name>.rs` plus an
entry in `lang::all()`; see the comments in `viewer/src/lang/mod.rs`. The
impl gives the name, file extension, the tool the V check needs, and the
layout (usually `layout::fill` and `layout::render` with the wrapper's
unbreakable head and tail, which the body must mirror exactly). Compiled
languages override `run` to build before running. The tests pick up every
language in `lang::all()` automatically.

## Languages

Each language the viewer supports has a line-by-line walkthrough of its
frame in `docs/`.

### Ruby

```ruby
eval(s=%w( ... )*"")
```

`%w(...)` splits the code on whitespace and `*""` joins it back, so the
program can be broken at any character, and `s` ends up holding its own
source. The body scores each cell, ranks them, picks as many cells as it has
characters, and prints itself into them. Walkthrough:
[docs/ruby.md](docs/ruby.md).

### Perl

```perl
eval($s=q( ... )=~y/!-~//cdr)
```

`q(...)` holds the code as a string that can be broken anywhere, and
`y/!-~//cdr` returns it with everything outside printable ASCII deleted,
which is the whitespace. `$s` then holds the program's own source and the
body does the same scoring and layout as the Ruby one. Needs Perl 5.14 or
later. Walkthrough: [docs/perl.md](docs/perl.md).

### JavaScript

```js
eval(s=`...`.replace(/[^!-~]/g,""))
```

Run with `node`. The template literal holds the code broken anywhere by
whitespace, and `replace` deletes everything outside printable ASCII, which
is exactly the whitespace; `s` keeps the stripped source. The body contains
no backticks, backslashes or `${`, so the template literal passes it through
untouched, and no keyword that needs a space after it (`let`, `return`), so
every variable is a sloppy-mode global and the frame must run as a plain
CommonJS script. Walkthrough: [docs/javascript.md](docs/javascript.md).

### Lua

```lua
s=[[ ... ]]load(s:gsub("%s",""))()
```

The long string `[[...]]` takes the code verbatim, line breaks and all, and
`gsub("%s","")` strips the whitespace back out before `load` compiles it.
Lua needs spaces between keywords and names, so the body is written to need
none: every condition is parenthesised and a `;` follows each `do`, `then`
and `end`. It does the same scoring, ranking and layout as the Ruby body.
Needs Lua 5.3 or later. Walkthrough: [docs/lua.md](docs/lua.md).

### Scheme

```scheme
(define(s)(string-filter char-set:graphic" ... "))(eval-string(s))
```

Runs on GNU Guile 3 (`brew install guile`). `(s)` returns the string with
its whitespace filtered out. Scheme needs spaces between atoms, so the body
is a short whitespace-free bootstrap followed by the real program inside a
`#|...|#` comment, with each of its spaces written as `_`. The bootstrap cuts
the program out of `(s)`, turns `_` back into spaces and evaluates it. That
program does the same scoring, ranking and layout as the Ruby body. It is
about 1,800 characters, so it needs a panel of at least 2,100 cells.
Walkthrough: [docs/scheme.md](docs/scheme.md).

### C

```c
#define Q(...)char*s=#__VA_ARGS__;__VA_ARGS__
Q( ... )
```

The macro compiles the body and also stringifies it into `s`, and since
stringifying turns any run of whitespace between tokens into one space, the
program recovers its own source by dropping the spaces. C can't take
whitespace inside a token, so the layout is token-aware: a token only starts
in a run of cells long enough to hold it, and the `#define` line and the one
string literal end their rows in a `\` line splice. The V check compiles with
`cc -O2`. The program is about 1.8k characters, so C needs a frame of at
least 2000 cells. Walkthrough: [docs/c.md](docs/c.md).
