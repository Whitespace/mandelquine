# The JavaScript frame, line by line

Every frame the viewer shows in JavaScript mode is a complete Node program.
Run it with `node frame.js` and it prints itself, byte for byte. This walks
through the program from the outside in. The body lives in
`viewer/src/lang/javascript.js`; the viewer strips its whitespace,
substitutes the view's parameters, and lays it out with `layout.rs`, which
repeats the same steps natively so it can draw the frame without running
Node.

## The wrapper

A frame looks like this, shrunk (the real one is shaped like the set):

```js
                eval(s=`w=1
             10;h=48;m=100;x0=-7.5
          e-1;y0=0e0;sc=3.09e-2;u=1;n=
                 ...
             `.replace(/[^!-~]/g,""))
```

Four things make this work.

1. **A template literal can span lines.** Everything between the backticks
   is one string, newlines and indentation included. So the code can be
   broken at any character, even in the middle of `Math.floor` or a number
   like `-7.5e-1`, and poured into whatever shape the set has.

2. **`.replace(/[^!-~]/g,"")` puts it back together.** `!` (0x21) to `~`
   (0x7E) is every printable ASCII character except space. Deleting
   everything else deletes the spaces and newlines the layout added, and
   nothing the code itself uses, because the body is written with no
   whitespace at all. The result is the original one-line body.

3. **`s=` keeps a copy.** The assignment's value is passed to `eval`, and
   `s` stays behind as a global holding the program's own source, minus the
   wrapper. That is the quine's self-reference.

4. **The template literal must pass the body through untouched.** Template
   literals interpret `\` escapes, end at a backtick, and evaluate `${...}`.
   The body therefore contains none of those three. It needs a backtick
   (and spaces and newlines) in its output, and gets them from
   `String.fromCharCode`.

The two ends, ``eval(s=` `` and ``.replace(/[^!-~]/g,""))``, can't be
broken: a newline inside `eval` or inside the regex would be a syntax error,
and these parts are outside the string, so `replace` never sees them. The
layout keeps each on a single row (see [Layout](#layout)).

One consequence of the no-whitespace rule: no `let`, `var`, `return`,
`new`, `of` or `in`, all of which need a space after them. Every variable is
an undeclared global, which only works in sloppy mode. Node runs a plain
`.js` file as a sloppy CommonJS script, so this is fine unless the file sits
under a `package.json` with `"type": "module"`.

## The body

What follows is `javascript.js` with whitespace for readability. The
viewer removes all of it.

### The view is the program's state

```js
w=W;
h=H;
m=M;
x0=X0;
y0=Y0;
sc=SC;
u=U;
```

The viewer fills these placeholders with the frame's width and height in
cells, the iteration limit, the centre, the scale (width of one cell in the
complex plane) and the placement flag (`1` puts the code outside the set,
`0` inside). Floats are written with Rust's `{:e}` formatting, the shortest
literal that round-trips, so `-0.75` becomes `-7.5e-1` and `0` becomes
`0e0`. JavaScript parses those to exactly the same doubles.

Because the view is written into the program, the program needs no input.
Running it recomputes the same frame from those seven numbers.

```js
n=w*h;
F=Math.floor;
Q=String.fromCharCode;
v=[];
d=[];
```

`n` is the cell count. `F` is integer division's other half: JavaScript has
only floating division, so `F(j/w)` is the row of cell `j`. `Q` builds the
characters the body can't contain literally.

### Escape counts

```js
for(j=0;j<n;j++){
  t=0;
  for(q=0;q<4;q++){
    cr=x0+(j%w+q%2*0.5+0.25-w*0.5)*sc;
    ci=y0+(h*0.5-F(j/w)-F(q/2)*0.5-0.25)*2*sc;
```

Each cell is sampled four times, on a 2x2 grid at offsets of a quarter and
three quarters of a cell. `q%2` picks the column of the sample and `F(q/2)`
the row. The imaginary axis is scaled by 2 because a terminal cell is about
twice as tall as it is wide.

The order of operations here is not cosmetic. `fractal.rs` computes the
same expressions in exactly this order, with plain multiplies and adds, so
every intermediate double is identical. Reordering `a+b+c` to `a+(b+c)`
could flip a sample near the boundary and the frame would no longer match.

```js
    x=y=i=0;
    for(z=1;z<=m;z++){
      r=x*x-y*y+cr;
      y=2*x*y+ci;
      x=r;
      if(x*x+y*y>4){i=z;break}
    }
    t+=i||m*2
  }
```

The standard iteration of `z ← z² + c`, recording the step `i` at which
`|z|² > 4`. A sample that never escapes counts as `2m`, so it outranks
every escaping one. `i||m*2` works because a sample that escapes does so
at step 1 or later, never 0.

These are integer counts, not the smooth `log`-based escape time the
original `mandelquine.rb` uses. Integer counts mean no `Math.log`, whose
last bit can differ between libm implementations.

```js
  v[j]=t;
  d[j]=t<8*m?0:n
}
```

`v[j]` is the cell's score, from 4 (escapes immediately) to `8m` (all four
samples inside). `d[j]` starts the distance transform below: 0 for a cell
that isn't fully inside the set, and "infinity" (`n`) for one that is.

### Distance to the edge

```js
for(k=1;k>-2;k-=2)
  for(i=0;i<n;i++){
    j=(n-1)*(1-k)/2+k*i;
```

Two passes of a chamfer distance transform: with `k=1`, `j` runs forwards
from 0 to `n-1`; with `k=-1`, backwards. The loop is written as
`k=1;k>-2;k-=2` because `for(k of[1,-1])` would need a space after `of`.

```js
    p=j-k;
    if(p>=0&&p<n&&F(p/w)==F(j/w)&&d[j]>d[p]+1)d[j]=d[p]+1;
    p=j-k*w;
    if(p>=0&&p<n&&d[j]>d[p]+2)d[j]=d[p]+2
  }
```

Each cell takes the smaller of its own distance and its already-visited
neighbour's plus a step: 1 for the horizontal neighbour (if it's on the
same row), 2 for the vertical one (cells are twice as tall). After both
passes `d[j]` is, for interior cells, the distance to the nearest
non-interior cell.

### Ranking

```js
K=j=>[-v[j]*(1-F(v[j]/(8*m))*u),d[j],j];
o=[...Array(n).keys()].sort((a,b)=>(a=K(a),b=K(b),a[0]-b[0]||a[1]-b[1]||a[2]-b[2]));
```

`o` lists the cells best first, by the key `K`, compared element by
element.

- With `u=0`, the first element is `-v[j]`, so interior cells (score `8m`)
  come first, and among them, the ones nearest the edge (smallest `d`).
  The code traces the inside of the outline and leaves the middle empty.
- With `u=1`, `F(v[j]/(8*m))` is 1 exactly for interior cells, which zeroes
  their first element and sends them to the back. The code sits in the
  exterior cells that took longest to escape, which hug the set from
  outside.
- `j` breaks every remaining tie, so the order is total and doesn't depend
  on the sort algorithm.

The comparator reuses its arguments `a` and `b` as scratch, which saves
declaring anything.

### Layout

```js
l=s.length+32;
g=Array(n).fill(0);
for(i=0;i<l;i++)g[o[i]]=1;
```

The program has `l` characters: the body `s` plus 32 for the wrapper (8
for ``eval(s=` `` and 24 for ``.replace(/[^!-~]/g,""))``). The `l`
best cells are marked 1.

```js
f=g.indexOf(1);
f=Math.min(f,f-f%w+w-8);
for(i=0;i<8;i++)g[f+i]=2;
```

The wrapper's head must sit on one row. Starting at the first marked cell
`f`, it takes the next 8 cells, moved left if needed so they don't run off
the end of the row (`f-f%w` is the start of `f`'s row). They are marked 2,
pinned.

```js
e=g.lastIndexOf(1);
e=Math.max(e,e-e%w+23);
for(i=0;i<24;i++)g[e-i]=2;
```

Same for the 24-character tail, ending at the last cell still marked 1 and
moved right if it would start before its row does.

```js
c=g.filter(z=>z).length;
for(i=l-1;i>=0;i--)if(c>l&&g[o[i]]==1){g[o[i]]=0;c--}
```

Pinning may have claimed up to 32 cells that weren't among the best `l`.
To get back to exactly `l`, unpin cells starting from the worst-ranked,
skipping the pinned ones.

### Printing

```js
e=[...('eval(s='+Q(96)+s+Q(96)+'.replace(/[^!-~]/g,""))')];
```

The full program text, without layout whitespace: head, body, tail.
`Q(96)` is the backtick. The tail's regex can be written literally because
`[^!-~]` has no backslash, which is the reason for choosing it over `\s`.
`e` is reused here; its old value was only needed for the pinning above.

```js
r=[];
for(y=0;y<h;y++){
  t='';
  for(x=0;x<w;x++)t+=g[y*w+x]?e.shift():Q(32);
  r.push(t.trimEnd())
}
console.log(r.join(Q(10)))
```

Walk the grid row by row. A marked cell takes the next character of the
program, anything else a space. Trailing spaces are trimmed, rows are
joined with newlines, and `console.log` adds the final one.

The output is the same frame: the same view, so the same scores, the same
ranking and the same cells, filled with the same characters. Since the
characters in the cells are exactly `e`, and `e` is what the frame itself
spelled out (once whitespace is removed), the program has printed itself.
