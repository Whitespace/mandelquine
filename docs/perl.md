# The Perl frame, line by line

This walks through the program the viewer shows when the language is Perl.
The source is `viewer/src/lang/perl.pl`, written one statement per line. The
viewer strips every whitespace character from it, substitutes the view's
numbers for the capitalised placeholders, wraps it, and pours it into the
shape of the set. Running the result prints the same text back.

It computes exactly what the Ruby frame computes ([ruby.md](ruby.md)), so
the two walkthroughs have the same sections. The differences are in how Perl
lets you write it.

## The wrapper

```perl
eval($s=q( ...the body, broken up by spaces and newlines... )=~y/!-~//cdr)
```

`q(...)` is a single-quoted string with parentheses as delimiters. Nothing
is interpolated, and nested parentheses are allowed as long as they balance.
So the body sits in the program as plain text, line breaks and all.

`y/!-~//cdr` is a transliteration applied to that string. `!-~` is the range
of printable ASCII characters other than space. The flags do the work:

- `c` complements the range, so it now means everything except those
  characters, which here is spaces and newlines.
- `d` deletes the matched characters, since the replacement list is empty.
- `r` returns the changed copy instead of modifying in place. This flag
  needs Perl 5.14.

`=~` binds tighter than `=`, so the stripped string is what gets assigned to
`$s`, and that assignment is the argument to `eval`. By the time the body
runs, `$s` holds the body's own source with all whitespace removed.

Two pieces of the wrapper cannot contain whitespace: `eval($s=q(` at the
start (10 characters) and `)=~y/!-~//cdr)` at the end (14 characters). The
layout keeps each on one row. Perl would tolerate spaces in a few places
inside them, but not inside `y/!-~//cdr`, and keeping each whole is simpler.

The body has the same three constraints as the Ruby one:

- No whitespace is significant. Perl is forgiving here: `for$j(0..$n-1)`,
  `print$t`, `shift@e` and `reverse@o` all parse without spaces because a
  sigil ends the keyword. Statement modifiers such as `... for 0..9` do need
  a space, so every loop and conditional uses the block form.
- Parentheses must balance, or `q(` would end early.
- No `#`, which would comment out the rest of the one-line program. That
  rules out `$#array`, so lengths are computed from `$n` and `$l` instead.

There is also no backslash anywhere, which sidesteps the question of how
`q()` treats escapes.

## The view

```perl
$w=W;
$h=H;
$m=M;
$x0=X0;
$y0=Y0;
$sc=SC;
$u=U;
$n=$w*$h;
```

In a real frame these are literals, for example
`$w=168;$h=46;$m=136;$x0=-7.5e-1;$y0=0e0;$sc=1.7e-2;$u=1;`.

- `$w`, `$h`: the frame is `$w` columns by `$h` rows, and `$n` is the
  number of cells.
- `$m`: the iteration limit.
- `$x0`, `$y0`: the complex number at the centre of the frame.
- `$sc`: the width of one character cell in the complex plane. A cell is
  treated as twice as tall as it is wide, so a row is `2*$sc` high.
- `$u`: 0 puts the code inside the set, 1 puts it outside.

Nothing is declared with `my`. The program runs without `strict`, every
variable is a package global, and that saves characters.

## Scoring every cell

```perl
for$j(0..$n-1){
$t=0;
for$q(0..3){
$cr=$x0+($j%$w+$q%2*0.5+0.25-$w*0.5)*$sc;
$ci=$y0+($h*0.5-int($j/$w)-int($q/2)*0.5-0.25)*2*$sc;
$x=$y=$i=0;
for(1..$m){
$r=$x*$x-$y*$y+$cr;
$y=2*$x*$y+$ci;
$x=$r;
if($x*$x+$y*$y>4){$i=$_;last}
}
$t+=$i||$m*2
}
$v[$j]=$t;
$d[$j]=$t<8*$m?0:$n;
$r[$j]=-$t*(1-int($t/(8*$m))*$u)
}
```

Cells are numbered `$j = 0 .. $n-1` in reading order, so `$j%$w` is the
column and `int($j/$w)` is the row. Perl's `/` is floating-point division,
which is why the `int` is there; Ruby's integer `/` did not need it.

Each cell is sampled at four points, a 2x2 grid inside the cell. `$q%2` and
`int($q/2)` are the sample's column and row within the cell. `$cr` and `$ci`
are the real and imaginary parts of the sample's position.

The innermost loop is the Mandelbrot iteration `z = z*z + c` on real
numbers. `$i` starts at 0 and is set to the step number `$_` at the first
step where `|z|^2 > 4`, after which `last` leaves the loop. If the point
never escapes, `$i` stays 0.

`$t+=$i||$m*2` adds the escape step, or `2*$m` when `$i` is 0. A cell's
score `$t` is the sum over its four samples, so a cell entirely inside the
set scores exactly `8*$m`.

The last three lines store three things per cell:

- `$v[$j]`, the score.
- `$d[$j]`, the starting value for the distance transform below: 0 for a
  cell that is not fully inside, `$n` (larger than any real distance) for an
  interior cell.
- `$r[$j]`, the first part of the sort key, explained under Ranking. It is
  computed here because the score is at hand. `@r` the array and `$r` the
  scalar used in the iteration are unrelated variables in Perl.

The arithmetic is done in the same order as in the Rust viewer and the Ruby
frame, on the same doubles, so all three get the same scores.

## Distance to the edge

```perl
for$k(1,-1){
for$i(0..$n-1){
$j=($n-1)*(1-$k)/2+$k*$i;
$p=$j-$k;
if($p>=0&&$p<$n&&int($p/$w)==int($j/$w)&&$d[$j]>$d[$p]+1){$d[$j]=$d[$p]+1}
$p=$j-$k*$w;
if($p>=0&&$p<$n&&$d[$j]>$d[$p]+2){$d[$j]=$d[$p]+2}
}
}
```

For each interior cell this finds the distance to the nearest cell that is
not interior, with the standard two-pass transform:

- With `$k=1`, `$j` runs forward. Each cell looks at its left neighbour and
  the cell above.
- With `$k=-1`, `$j` runs backward. Each cell looks at its right neighbour
  and the cell below.

`($n-1)*(1-$k)/2+$k*$i` is `$i` when `$k` is 1 and `$n-1-$i` when it is -1.

`$p` is the neighbour's index. For the horizontal neighbour, the bounds
check and `int($p/$w)==int($j/$w)` make sure it exists and is on the same
row. Ruby got the lower bound for free from rounding-down division; in Perl
a negative index would silently read from the end of the array, so the
check is explicit.

A horizontal step costs 1 and a vertical step costs 2, matching the shape
of a cell.

## Ranking

```perl
@o=sort{$r[$a]<=>$r[$b]||$d[$a]<=>$d[$b]||$a<=>$b}0..$n-1;
```

`@o` is every cell index, best first. Inside a `sort` block `$a` and `$b`
are the two items being compared, which is why no other variable in the
program uses those names.

The comparison chains three keys with `||`, moving to the next only on a
tie:

1. `$r[...]`, computed earlier as `-$t*(1-int($t/(8*$m))*$u)`.
   `int($t/(8*$m))` is 1 for an interior cell and 0 otherwise. With `$u=0`
   the key is just `-$t`, so the highest scores come first and the interior
   leads. With `$u=1` an interior cell's key becomes 0, which sorts after
   every negative number, so exterior cells come first.
2. `$d[...]`, the distance to the edge. It only separates interior cells,
   and puts the ones nearest the edge first.
3. The index itself, so the order is total and does not depend on how
   Perl's sort handles equal elements.

## Choosing the cells

```perl
$l=length($s)+24;
for(0..$n-1){$g[$_]=0}
for(0..$l-1){$g[$o[$_]]=1}
$f=0;
while(!$g[$f]){$f++}
$p=$f-$f%$w+$w-10;
if($p<$f){$f=$p}
for(0..9){$g[$f+$_]=2}
$e=$n-1;
while($g[$e]!=1){$e--}
$p=$e-$e%$w+13;
if($p>$e){$e=$p}
for(0..13){$g[$e-$_]=2}
$c=0;
for(0..$n-1){$c+=$g[$_]>0}
for(reverse@o[0..$l-1]){if($c>$l&&$g[$_]==1){$g[$_]=0;$c--}}
```

`$l` is the length of the whole program: the body plus the 24 characters of
wrapper. The program needs exactly `$l` cells.

`@g` marks which cells get a character: 0 for none, 1 for an ordinary cell,
2 for a pinned one. Start by marking the `$l` best cells with 1.

Then pin the wrapper's two ends:

- `$f` is the first marked cell. The head needs 10 cells in a row starting
  there. `$f-$f%$w` is the start of its row, so `$f-$f%$w+$w-10` is the
  last place in the row the head can start, and `$f` moves left to it if
  needed.
- `$e` is the last cell marked 1. The tail needs the 14 cells ending there.
  If `$e` is in the first 13 columns of its row it moves right to column 13.

Pinning can mark cells that were not among the best `$l`, so the count `$c`
may now be up to 24 over. `$c+=$g[$_]>0` adds 1 for each marked cell, since
a true comparison is 1 in Perl. The last line walks the chosen cells from
worst to best, unmarking ordinary ones until exactly `$l` remain.

`$e` is a scalar here and `@e` below is an array; again they are separate.

## Printing

```perl
@e=split//,'eval($s=q('.$s.')=~y/!-~//cdr)';
for$y(0..$h-1){
$t='';
for$x(0..$w-1){$t.=$g[$y*$w+$x]?shift@e:chr(32)}
$t=~s/[^!-~]+$//;
print$t,chr(10)
}
```

`@e` is the full program as a list of characters: wrapper, body, wrapper.
`split//` with an empty pattern splits between every character. The wrapper
is in single quotes so that `$s` inside it is literal text; only the `.$s.`
in the middle is the variable.

For each row, each marked cell takes the next character from `@e` and every
other cell takes a space. `chr(32)` is a space and `chr(10)` a newline,
spelled that way because the literal characters would be stripped out of
the body.

`s/[^!-~]+$//` removes trailing spaces from the row, using the same
character range as the wrapper instead of a backslash class.

The parentheses in the two string pieces balance each other (`eval(` and
`q(` open two, `)=~y/!-~//cdr)` closes two), which keeps the enclosing `q(`
intact.

## Why the output is the input

The frame the viewer shows was produced by the same steps in Rust
(`viewer/src/lang/layout.rs`), given the same literals and the head and tail
lengths 10 and 14. When Perl runs the frame it repeats the computation,
arrives at the same set of cells, and fills them with the same characters in
the same order.

Whitespace in the input does not affect `$s`, so any arrangement of the same
characters with the head and tail intact prints the canonical frame, and the
canonical frame prints itself.

The viewer's V key runs the frame through `perl` and compares the output
byte for byte. `cargo test` does the same for a handful of views. This was
checked on Perl 5.38.
