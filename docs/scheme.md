# The Scheme frame, line by line

This walks through the program the viewer shows when the language is Scheme.
It targets GNU Guile 3 (`brew install guile`) and runs as
`guile --no-auto-compile frame.scm`. The flag only stops Guile from writing a
compiled copy into `~/.cache`; the frame runs the same without it.

The source is in two places:

- `viewer/src/lang/scheme.scm`: the main program, written as ordinary,
  indented Scheme.
- `viewer/src/lang/scheme.rs`: the wrapper and a short bootstrap, plus the
  code that packs `scheme.scm` into the body.

The layout itself is the one every language shares, `viewer/src/lang/layout.rs`.
[docs/ruby.md](ruby.md) explains that algorithm in detail. This document
covers what is different about Scheme and walks through the Scheme code; for
the reasoning behind the scoring, distance and ranking steps it points back
to the Ruby walkthrough.

## The problem Scheme poses

The shared layout breaks the program body at arbitrary characters and pours
it into the shape of the set. So the body has to survive whitespace being
inserted anywhere, including in the middle of a name like `vector-ref`.

Ruby, Perl, JavaScript and Lua manage this by holding the body in a string,
deleting the whitespace, and evaluating what is left. That only works if the
code needs no whitespace of its own. Those languages can be written that
way. Scheme cannot: `(define x 5)` needs both of its spaces, and
`(f a b)` needs a space between every pair of atoms. In Guile, `#`, `'` and
`|` do not end a name either, so none of the usual punctuation tricks help.
The only characters that separate atoms are whitespace, parentheses,
square brackets, `"` and `;`.

So the Scheme frame needs one extra step. Each space the code needs is
written as `_`, which survives the whitespace filter, and turned back into a
space just before the code is evaluated.

## The wrapper

A frame, stripped of its layout, has this shape:

```scheme
(define(s)(string-filter char-set:graphic"BOOTSTRAP#|PROGRAM|#"))(eval-string(s))
```

`(define(s) ...)` defines `s` as a procedure of no arguments. Calling `(s)`
takes the string literal, keeps only its graphic characters, and returns the
result. Spaces and newlines are not graphic, so `(s)` is the body with the
layout removed, however it was broken up. The `"` that opens the string is
the last character of the head and the one that closes it is the first
character of the tail.

`s` is a procedure rather than a plain variable for a reason that comes up
in the bootstrap: `(s)` is written with no spaces, `s` alone is not.

`(eval-string(s))` then evaluates the filtered body as Scheme source.

The head, `(define(s)(string-filter char-set:graphic"`, is 42 characters
and the tail, `"))(eval-string(s))`, is 19. Both contain spaces and names
that must not be split, so the layout pins each one to a single row, exactly
as it pins Ruby's `eval(s=%w(` and `)*"")`. Their length is why the Scheme
frame refuses panels narrower than 44 columns. The whole program is about
1,800 characters, more than twice Ruby's, so it also needs a panel of at
least 2,100 cells (60x35, say).

Inside the string the body may not contain `"`, which would end it, or
`\`, which would start an escape sequence. A raw newline or space inside a
Scheme string is fine, which is what lets the layout break the body anywhere.

## The bootstrap

The filtered body starts with this, the only code in the frame that has to
work with no whitespace at all:

```scheme
(eval-string(string-join(string-split(substring(s)104(-(string-length(s))2))(integer->char(+(+)95)))))
```

Reading inside out:

- `(substring(s)104(-(string-length(s))2))` cuts the program out of the
  body. The bootstrap is 102 characters long and is followed by `#|`, so the
  program starts at index 104; the last two characters of the body are `|#`.
  `scheme.rs` computes the 104 when it builds the body (it is the
  bootstrap's own length plus two, and the bootstrap contains that number, so
  it tries each value until one is consistent).
- `(integer->char(+(+)95))` is the character `_`. The usual spelling, `#\_`,
  has a backslash, which is not allowed inside the string. `integer->char`
  followed by `95` would need a space, so the 95 is wrapped as `(+(+)95)`:
  `(+)` is 0, and `(+ 0 95)` is 95 with a parenthesis on each side of every
  atom.
- `string-split` cuts the program at every `_` and `string-join` glues the
  pieces back with its default separator, a single space. That turns every
  `_` back into a space.
- `eval-string` evaluates the result.

Everything here is a name followed by `(`, or a number between `)` and `(`,
so no spaces are needed. This is also why `(s)` is a procedure call: the
procedures here all take `s` as their first argument, and `(substring s 104 ...)`
would need a space where `(substring(s)104 ...)` does not.

After the bootstrap comes `#|PROGRAM|#`. To Guile reading the body this is
a block comment, so the outer `eval-string` evaluates the bootstrap and then
skips the program. The bootstrap is what runs the program, from inside the
comment.

## Packing the program

`scheme.rs` turns `scheme.scm` into the program text with three steps:

1. Collapse every run of whitespace to one space.
2. Drop the spaces next to `(` and `)`, where Scheme does not need them.
3. Write each remaining space as `_`.

So

```scheme
(let* ((w W) (h H)
```

becomes `(let*((w_W)(h_H)`. Every `_` left in a frame is a space that
Scheme needs. The program never uses `_` itself, nor `"`, `\`, `|#`, or any
capital letter except the placeholders, and a unit test in `scheme.rs` checks
all of that.

The viewer then substitutes the view's numbers for the placeholders, exactly
as for Ruby.

## The view

```scheme
(let* ((w W) (h H) (m M) (a X0) (b Y0) (c SC) (u U)
       (n (* w h)) (t (1+ n)) (r vector-ref) (z vector-set!)
```

In a real frame these are literals, for example
`(let*((w_110)(h_48)(m_100)(a_-7.5e-1)(b_0e0)(c_3.09e-2)(u_0)`. They mean
the same as in Ruby: frame size, iteration limit, centre, cell width, and 0
or 1 for code inside or outside the set. Here the centre is `a`, `b` and the
cell width `c`, since every name costs characters. Guile reads the
exponent-form floats (`-7.5e-1`, `0e0`) to the same doubles Rust wrote.

`n` is the number of cells and `t` is `n+1`, used below to pack a sort key.
`r` and `z` are short names for `vector-ref` and `vector-set!`, which are
used often enough that the aliases pay for themselves.

The whole program is one `let*`. A `let*` binding costs `(w_W)` where a
top-level definition would cost `(define_w_W)`.

## A loop helper

```scheme
       (f (lambda (k p) (do ((i 0 (1+ i))) ((= i k)) (p i))))
```

`(f k p)` calls `p` with 0, 1, ..., `k-1`. `1+` is Guile's increment.

## The text to print

```scheme
       (q (string (integer->char 34)))
       (e (string-append (symbol->string '#{(define(s)(string-filter char-set:graphic}#) q (s)
                         q (symbol->string '#{))(eval-string(s))}#)))
       (l (string-length e))
```

`e` is the whole frame without layout: head, body, tail. It is what will be
poured into the cells.

The program cannot contain a string literal, since a `"` would end the
string that holds it. So the head and tail are built from symbols. Guile's
`#{...}#` syntax reads anything up to `}#` as a symbol name, spaces and
parentheses included, and `symbol->string` turns it back into text. The `"`
on each side comes from `(integer->char 34)`.

`(s)` is the body exactly as the frame must print it: underscores, bootstrap,
comment markers and all.

`l` is the number of characters to place.

## Scoring every cell

```scheme
       (v (map (lambda (j)
                  (apply + (map (lambda (p o)
                                  (let ((p (+ a (* (- (+ (+ (modulo j w) p) .25) (* w .5)) c)))
                                        (o (+ b (* (* (- (- (- (* h .5) (quotient j w)) o) .25) 2.) c))))
                                    (let g ((i 1) (x 0.) (y 0.))
                                      (if (> i m)
                                          (* m 2)
                                          (let ((x (+ (- (* x x) (* y y)) p)) (y (+ (* (* 2. x) y) o)))
                                            (if (> (+ (* x x) (* y y)) 4) i (g (1+ i) x y)))))))
                                '(0 .5 0 .5) '(0 0 .5 .5))))
                (iota n)))
```

This is the same score as Ruby's, so see [docs/ruby.md](ruby.md#scoring-every-cell)
for why. `v` is a list with one entry per cell `j`, the sum of the escape
counts of four samples.

The four samples come from mapping over two lists at once: the inner
`lambda` receives the pairs (0, 0), (.5, 0), (0, .5) and (.5, .5), the
offset of each sample within the cell. It first binds `p` and `o` to those
offsets and then rebinds them to the sample's position in the complex
plane. Each arithmetic operation is written with exactly two operands, in
the same order as `viewer/src/fractal.rs`, so that Guile and Rust round
identically.

The named `let g` is the iteration loop. Its `let` evaluates both new values
from the old `x` and `y` before binding either, so the real and imaginary
parts update together. It returns the step `i` at which `|z|^2 > 4`, or
`2m` if the sample never escapes within `m` steps.

Guile's evaluator runs this directly, without compiling it, so a deep zoom
with a high iteration limit takes a few seconds to check.

## Distance to the edge

```scheme
       (d (list->vector (map (lambda (a) (if (< a (* 8 m)) 0 n)) v)))
       (g (lambda (j p k) (if (> (r d j) (+ (r d p) k)) (z d j (+ (r d p) k))))))
  (for-each (lambda (k)
              (f n (lambda (i)
                     (let* ((j (if (< k 0) (- n i 1) i)) (p (- j k)) (o (- j (* k w))))
                       (if (= (floor-quotient p w) (quotient j w)) (g j p 1))
                       (if (< -1 o n) (g j o 2))))))
            '(1 -1))
```

The same two-pass distance transform as Ruby's: `d` starts at 0 outside the
set and `n` inside, then a forward pass looks left and up and a backward
pass looks right and down. A horizontal step costs 1 and a vertical step 2.
See [docs/ruby.md](ruby.md#distance-to-the-edge).

`g` relaxes `d[j]` through neighbour `p` with step cost `k`. `j` runs
forward when `k` is 1 and backward when it is -1.

The horizontal neighbour must be on the same row. `floor-quotient` rounds
down, so for `p = -1` it gives -1, which is never equal to row 0. Plain
`quotient` would round toward zero and give 0. The vertical neighbour `o`
is checked with `(< -1 o n)`, Scheme's chained comparison for
`0 <= o < n`.

## Ranking

```scheme
  (let* ((o (list-head (map (lambda (k) (modulo k t))
                            (sort (map (lambda (a j)
                                         (+ (* (+ (* (- (* a (- 1 (* (quotient a (* 8 m)) u)))) t) (r d j)) t) j))
                                       v (iota n))
                                  <))
                       l))
```

Ruby sorts cells by the triple `[-v*(1-v/(8m)*u), d, j]`. Writing a
comparison of triples in Scheme takes a lot of characters, so the triple is
packed into one exact integer instead:

    key = (score * t + d) * t + j

`d` and `j` are both between 0 and `n`, and `t` is `n+1`, so they are the
two low digits of a base-`t` number and the score is everything above them.
Comparing keys with `<` compares the score first, then `d`, then `j`,
exactly like the triple. The score is zero or negative, and that still
works, since only the top part of the number carries a sign. Scheme
integers have no fixed size, so nothing overflows.

After sorting, `(modulo k t)` recovers `j`. `list-head` keeps the best `l`
cells: `o` is the list of cells that will get a character, best first.

## Choosing the cells

```scheme
         (g (make-vector n 0))
         (a (apply min o))
         (a (min a (- (+ (- a (modulo a w)) w) 42))))
    (for-each (lambda (j) (z g j 1)) o)
    (f 42 (lambda (i) (z g (+ a i) 2)))
    (let* ((b (let p ((j (1- n))) (if (= (r g j) 1) j (p (1- j)))))
           (b (max b (+ (- b (modulo b w)) 18)))
           (c 0)
           (k -1))
      (f 19 (lambda (i) (z g (- b i) 2)))
      (f n (lambda (j) (if (> (r g j) 0) (set! c (1+ c)))))
      (for-each (lambda (j) (if (and (> c l) (= (r g j) 1)) (begin (z g j 0) (set! c (1- c)))))
                (reverse o))
```

This is the shared layout step from [docs/ruby.md](ruby.md#choosing-the-cells),
with Scheme's 42-character head and 19-character tail.

- Mark the chosen cells with 1 in `g`.
- `a` is the first chosen cell, the smallest index in `o`. The head needs 42
  cells in one row from there, so `a` moves left if it is too close to the
  end of its row. `(- a (modulo a w))` is the start of `a`'s row. Mark those
  42 cells 2.
- `b` is the last cell still marked 1, found by scanning back from the end.
  The tail needs the 19 cells ending at `b`, so `b` moves right if it is in
  the first 18 columns. Mark those 19 cells 2.
- Pinning may have marked up to 61 extra cells. Count the marked cells into
  `c`, then walk the chosen cells from worst to best, unmarking ordinary
  ones (never the pinned 2s) until exactly `l` remain.

The 42, 18 and 19 are written out rather than computed, to save characters.
A test in `scheme.rs` fails if the head or tail ever changes length.

## Printing

```scheme
      (f h (lambda (y)
             (display (string-trim-right
                       (list->string
                        (map (lambda (x)
                               (if (> (r g (+ (* y w) x)) 0)
                                   (begin (set! k (1+ k)) (string-ref e k))
                                   (integer->char 32)))
                             (iota w)))))
             (newline))))))
```

For each row `y`, each column `x` takes the next character of `e` if its
cell is marked and a space otherwise. `k` counts the characters used so far.
A space is `(integer->char 32)` because a literal `#\space` contains a
backslash. Trailing spaces are trimmed and each row ends in a newline.

The head contains a space of its own, before `char-set:graphic`. It is
printed like any other character, and since the head always ends in `"` it
is never trailing.

## Why the output is the input

The viewer computed the frame in Rust from the same literals: same scores,
same distances, same ranking, same pins. Guile repeats that computation
when it runs the frame and arrives at the same cells. It fills them with the
same characters, because `e` is the head, `(s)` and the tail, and `(s)` is
the body with exactly the layout's whitespace removed.

The viewer's V key runs the frame through `guile` and compares the output
with the frame byte for byte, and `cargo test` does the same for several
views, with the code both inside and outside the set.
