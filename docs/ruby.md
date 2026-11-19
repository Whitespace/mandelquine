# The Ruby frame, line by line

This walks through the program the viewer shows when the language is Ruby.
The source is `viewer/src/lang/ruby.rb`, written one statement per line. The
viewer strips every whitespace character from it, substitutes the view's
numbers for the capitalised placeholders, wraps it, and pours it into the
shape of the set. Running the result prints the same text back.

Nothing below is special to one view. The only things that change from frame
to frame are the seven literals at the top.

## The wrapper

```ruby
eval(s=%w( ...the body, broken up by spaces and newlines... )*"")
```

`%w(...)` is Ruby's word-array literal: it splits its contents on whitespace
and gives back an array of strings. `*""` on an array is `join("")`, so the
pieces are glued back together with nothing between them. However the body
is broken across lines, the result is the same string.

That string is assigned to `s` and passed to `eval`. So by the time the body
runs, `s` holds the body's own source with all whitespace removed. That is
the whole trick that makes this a quine: the program never has to describe
itself, it just has to print `s` with the right wrapper and the right
whitespace.

Two pieces of the wrapper cannot contain whitespace: `eval(s=%w(` at the
start (10 characters) and `)*"")` at the end (5 characters). A space inside
`%w(` or between the two quotes would change the meaning. The layout below
has to keep each of them on one row.

The body itself has three constraints, all because of where it lives:

- No whitespace is significant, since all of it is removed. Every statement
  ends in `;` and nothing relies on a space between tokens.
- Parentheses must balance, since an unmatched `)` would end the `%w(`.
- No `#`, which would comment out the rest of the one-line program.

## The view

```ruby
w=W;
h=H;
m=M;
x0=X0;
y0=Y0;
sc=SC;
u=U;
```

In a real frame these are literals, for example
`w=148;h=41;m=140;x0=-1.042e0;y0=0e0;sc=1.62e-2;u=1;`.

- `w`, `h`: the frame is `w` columns by `h` rows.
- `m`: the iteration limit.
- `x0`, `y0`: the complex number at the centre of the frame.
- `sc`: the width of one character cell in the complex plane. A cell is
  treated as twice as tall as it is wide, so a row is `2*sc` high.
- `u`: 0 puts the code inside the set, 1 puts it outside.

This is the program's entire state. Panning, zooming and the inside/outside
toggle in the viewer only ever change these numbers.

The floats are written in exponent form because that is the shortest text
that parses back to exactly the same double, in both Rust and Ruby.

## Scoring every cell

```ruby
v=(0...w*h).map{|j|
a=0;
4.times{|q|
cr=x0+(j%w+q%2*0.5+0.25-w*0.5)*sc;
ci=y0+(h*0.5-j/w-q/2*0.5-0.25)*2*sc;
x=y=0.0;
i=(1..m).find{r=x*x-y*y+cr;y=2*x*y+ci;x=r;x*x+y*y>4};
a+=i||m*2
};
a
};
```

Cells are numbered `j = 0 ... w*h-1` in reading order, so `j%w` is the
column and `j/w` (integer division) is the row.

Each cell is sampled at four points, a 2x2 grid inside the cell. `q%2` and
`q/2` are the sample's column and row within the cell, and the `0.25` and
`0.5` offsets put the samples at the quarter points. `cr` and `ci` are the
real and imaginary parts of the sample's position: measure from the centre
of the frame in cells, scale by `sc`, add the centre.

The inner block is the Mandelbrot iteration `z = z*z + c` written out on real
numbers: `r` is the new real part, `y` becomes the new imaginary part (using
the old `x`), then `x` takes `r`. `find` returns the first step `i` at which
`|z|^2 > 4`, or `nil` if the point never escapes within `m` steps.

A sample scores `i` if it escaped and `2*m` if it did not. The cell's score
`a` is the sum over its four samples. So:

- A cell that is entirely inside the set scores exactly `8*m`, the maximum.
- A cell far from the set scores a small number.
- A cell near the set, or straddling its edge, scores somewhere in between.

The scores are integers. An earlier version used the usual smooth escape
time, which needs `log`. Integer counts were chosen so that the Rust viewer
and Ruby cannot disagree because of a math library.

The exact order of the arithmetic matters for the same reason. `x*x-y*y+cr`
must be computed as `((x*x)-(y*y))+cr` in both places, and it is.

## Distance to the edge

```ruby
d=v.map{|z|z<8*m&&0||w*h};
[1,-1].each{|k|
(w*h).times{|i|
j=(w*h-1)*(1-k)/2+k*i;
(j-k)/w==j/w&&d[j]>d[j-k]+1&&(d[j]=d[j-k]+1);
n=j-k*w;
n>=0&&n<w*h&&d[j]>d[n]+2&&(d[j]=d[n]+2)
}
};
```

For each interior cell this computes how far it is from the nearest cell
that is not interior. It is what lets the code hug the outline of the set
and leave the middle empty.

`d` starts at 0 for every cell that is not fully inside, and at `w*h` (more
than any real distance) for interior cells. `z<8*m&&0||w*h` is a ternary
written without `?:`; it works because 0 is truthy in Ruby.

Then two sweeps, the standard two-pass distance transform:

- With `k=1`, `j` runs forward from 0. Each cell looks at its left
  neighbour `j-1` and the cell above `j-w`.
- With `k=-1`, `j` runs backward from the last cell. Each cell looks at its
  right neighbour `j+1` and the cell below `j+w`.

The expression `(w*h-1)*(1-k)/2+k*i` is `i` when `k=1` and `w*h-1-i` when
`k=-1`.

A horizontal step costs 1 and a vertical step costs 2, matching the 1:2
shape of a cell, so the band of code comes out about equally thick on all
sides.

`(j-k)/w==j/w` checks that the horizontal neighbour is on the same row, so
distances do not wrap around the edge of the frame. It also covers `j-k`
being -1, because Ruby's integer division rounds down and `-1/w` is -1. The
vertical neighbour gets an explicit bounds check.

Cells beyond the frame are not treated as an edge. If the interior runs off
the side of the frame, the code does not line up along the frame's border.

## Ranking

```ruby
o=(0...w*h).sort_by{|j|[-v[j]*(1-v[j]/(8*m)*u),d[j],j]};
```

`o` is every cell index, best first. The sort key is a triple.

The first part decides inside versus outside. `v[j]/(8*m)` is integer
division, so it is 1 for an interior cell and 0 for everything else.

- With `u=0` the factor is always 1 and the key is `-v[j]`: highest score
  first, so interior cells lead, then the exterior cells closest to the set.
- With `u=1` an interior cell's key becomes `-v[j]*0 = 0`, which sorts after
  every negative number. Exterior cells come first, highest score first, and
  the interior is only used if the exterior runs out.

The second part, `d[j]`, only matters among interior cells, which all share
the same score. Smaller distance first means the edge of the set fills
before its middle.

The third part, `j`, breaks any remaining tie by reading order. With it the
ordering is total, so Ruby and Rust produce the same `o` without depending
on how either one's sort treats equal keys.

## Choosing the cells

```ruby
l=s.size+15;
g=[nil]*(w*h);
o[0,l].each{|j|g[j]=1};
a=g.index(1);
a=[a,a/w*w+w-10].min;
10.times{|i|g[a+i]=2};
b=g.rindex(1);
b=[b,b/w*w+4].max;
5.times{|i|g[b-i]=2};
c=g.count{|z|z};
o[0,l].reverse.each{|j|c>l&&g[j]==1&&(g[j]=nil;c-=1)};
```

`l` is the length of the whole program: the body `s` plus the 15 characters
of wrapper. The program needs exactly `l` cells.

`g` marks which cells get a character. Start by marking the `l` best cells
with 1.

Then pin the two unbreakable ends of the wrapper, marking them 2:

- `a` is the first marked cell. The head `eval(s=%w(` needs 10 cells in a
  row starting there. If `a` is within 10 cells of the end of its row, it is
  moved left so the head fits: `a/w*w+w-10` is the last position in that row
  where it can start.
- `b` is the last marked cell. The tail `)*"")` needs the 5 cells ending
  there. If `b` is in the first 4 columns of its row it is moved right to
  column 4.

Since characters are placed in reading order, the first 10 marked cells get
the head and the last 5 get the tail, each on one row.

Pinning can mark cells that were not among the best `l`, so the count `c`
may now be up to 15 over. The last line walks the chosen cells from worst to
best and unmarks ordinary ones (value 1, never the pinned 2s) until exactly
`l` remain. Removing cells between `a` and `b` never changes which cells are
first and last, so the pins stay valid.

## Printing

```ruby
e=('eval(s=%w('+s+')*"")').chars;
puts((0...h).map{|y|(0...w).map{|x|g[y*w+x]&&e.shift||32.chr}.join.rstrip})
```

`e` is the full program as a list of characters: wrapper, body, wrapper.

For each row, each cell prints the next character of `e` if it is marked and
a space otherwise. `32.chr` is a space; it has to be spelled that way because
a literal space in the body would be stripped. Trailing spaces are removed
from each row, and `puts` prints the rows one per line.

The parentheses inside those string literals balance (`eval(` and `%w(`
open two, `)*"")` closes two), which is what keeps the enclosing `%w(`
intact.

## Why the output is the input

The frame the viewer shows was produced by the same steps, carried out in
Rust (`viewer/src/lang/layout.rs`): same literals, same samples, same
scores, same ranking, same pins. When Ruby runs the frame it reads those
literals, repeats the computation, arrives at the same set of cells, and
fills them with the same characters in the same order.

Whitespace in the input does not affect `s`, so the program does not even
need to be laid out correctly to work. Any arrangement of the same
characters, with the head and tail intact, prints the canonical frame, and
the canonical frame prints itself.

The viewer's V key runs the frame through `ruby` and compares the output
with the frame byte for byte. `cargo test` does the same for a handful of
views.
