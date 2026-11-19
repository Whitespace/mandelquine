# The Lua frame, line by line

This walks through the program the viewer shows when the language is Lua.
The source is `viewer/src/lang/lua.lua`, written one statement per line. The
viewer strips every whitespace character from it, substitutes the view's
numbers for the capitalised placeholders, wraps it, and pours it into the
shape of the set. Running the result with `lua` prints the same text back.

The algorithm is the same as the Ruby frame's ([docs/ruby.md](ruby.md)), and
`viewer/src/lang/layout.rs` mirrors both. What is different is how Lua is
persuaded to run code that has no whitespace in it, and a handful of places
where the body is bent to stay short. Those get the most attention here.

## The wrapper

```lua
s=[[ ...the body, broken up by spaces and newlines... ]]load(s:gsub("%s",""))()
```

`[[...]]` is a Lua long string. Nothing inside it is interpreted: no escapes,
and line breaks are kept. So `s` is the body exactly as it appears in the
frame, spaces and newlines included.

`s:gsub("%s","")` replaces every whitespace character (`%s` is Lua's pattern
class for whitespace) with nothing. `load` compiles the resulting string into
a function, and the trailing `()` calls it. Inside, the body reads the same
global `s` to learn its own text.

`gsub` returns two values, the new string and the number of replacements, and
both are passed to `load`. The count lands in `load`'s second parameter, the
chunk name used in error messages, where a number is accepted and ignored for
our purposes. Wrapping the call in another pair of parentheses would drop it,
but would cost two characters in the part of the frame that cannot be broken.

The head `s=[[` (4 characters) and the tail `]]load(s:gsub("%s",""))()` (25
characters) are the unbreakable ends. A space inside `[[` or `]]`, `load`,
`"%s"` or `""` would change the meaning. The layout keeps each on one row,
which is why a Lua frame must be at least 25 columns wide.

The body has three constraints, all because of where it lives:

- It must mean the same with every whitespace character deleted. This is the
  hard one, and the next section is about it.
- It must not contain `]]`, which would end the long string early. Indexing
  with an index, `g[o[i]]`, is the obvious way to write it by accident; the
  body uses a temporary `j=o[i]` instead.
- It must not contain `--`, which starts a comment. With the newlines gone,
  a comment would swallow the rest of the program. This also means no
  placeholder may follow a `-`, since its value can be negative.

## Writing Lua without spaces

Ruby's body can drop whitespace freely because almost every Ruby token can
touch its neighbour. Lua needs a space wherever a keyword meets a name or a
number: `while j<n do a=0` stripped is `whilej<ndoa=0`, which is nonsense.
The body avoids every such junction with three habits.

**Parenthesised conditions.** `while(j<n)do`, `if(p>=0)then`,
`return(a<b)`. A `(` can follow a keyword directly, and a `)` can precede
one. The same goes for `and` and `or` between operands:
`(a<8*m)and(0)or(n)`. A string can also touch a keyword, so
`(g[j]>0)and"#"or"\32"` needs no parentheses round the strings.

**A `;` after `do`, `then` and `end`.** `do;a=0` and `end;q=q+1`. Lua 5.2 and
later accept `;` as an empty statement anywhere, so these cost one character
each and keep the keyword from fusing with the next name.

**A `;` between statements, except after a closing bracket.** `a=0;q=0` needs
the `;` (stripped, `a=0q=0` is a malformed number). But a statement ending in
`)`, `]`, `}` or a quote can be followed directly by the next one, because
nothing can fuse across the bracket: `v={}d={}o={}g={}` is four statements.
The body uses this wherever it can, purely to save characters.

There is no `for` loop anywhere. `for` has to be followed by a name, and there
is no punctuation that can go between them. Every loop is a `while` with a
counter.

None of this was checked by eye. Compiling `lua.lua` and its stripped form
with `luac -l` gives identical bytecode listings (apart from line numbers),
which is the real test that removing the whitespace changes nothing.

## The view

```lua
w=W;
h=H;
m=M;
E=SC;
```

In a real frame these are literals, for example
`w=120;h=40;m=100;E=3.06e-2;`. Three more placeholders appear further down,
inside the expressions that use them: `X0` and `Y0` (the centre) and `U`
(0 for code inside the set, 1 for outside). They are used once each, so
giving them a variable would only cost characters. `SC`, the width of a cell
in the complex plane, is used twice and gets the variable `E`.

The viewer replaces only the first appearance of each placeholder, so `SC`
can come before `X0` and `Y0` here. What matters is that none of them occurs
earlier by accident, and no literal it puts in contains another placeholder.

```lua
n=w*h;
z=math.min;
v={}d={}o={}g={}
```

`n` is the number of cells. `z` is `math.min`, used six times. There is no
alias for `math.max`; where a maximum is needed the body writes `-z(-a,-b)`.

The four tables hold, per cell: `v` the score, `d` the distance to the
edge, `g` whether the cell gets a character; `o` is the list of cells in
rank order. Lua tables are happy with index 0, so cells are numbered from 0
as in Ruby and Rust. Only `o` is a proper 1-based sequence, because
`table.sort` needs one.

## Scoring every cell

```lua
j=0;
while(j<n)do;
  a=0;
  q=0;
  while(q<4)do;
    R=X0+(j%w+q%2*0.5+0.25-w*0.5)*E;
    I=Y0+(h*0.5-j//w-q//2*0.5-0.25)*2*E;
    x=0;
    y=0;
    i=0;
    while(i<m)and(x*x+y*y<=4)do;
      i=i+1;
      p=x*x-y*y+R;
      y=2*x*y+I;
      x=p;
    end;
    a=a+((x*x+y*y>4)and(i)or(m*2))
    q=q+1;
  end;
```

Each cell `j` is sampled at four points, a 2x2 grid at the quarter points of
the cell, exactly as in Ruby. `j%w` is the column and `j//w` the row. `//` is
floor division, which Lua has had since 5.3; `/` would give a float.

`R` and `I` are the real and imaginary parts of the sample. The arithmetic
must happen in exactly the order the Rust viewer uses, since the frame is
only a quine if both sides get bit-identical doubles. Lua evaluates
`j%w+q%2*0.5+0.25-w*0.5` left to right with the usual precedence, the same
as Rust and Ruby, and each operation is a separate rounding: the Lua VM has
no fused multiply-add.

Lua 5.3 and later have two number types, integers and floats. `j`, `w`, `q`
are integers; the moment one meets `0.5` the result is a float, converted
exactly. That is why `x=0;y=0` can start as integers: the first step computes
`0*0-0*0+R`, which is the float `R`, the same value Rust gets from
`0.0*0.0-0.0*0.0+cr`.

The iteration loop runs while fewer than `m` steps have been taken and the
point has not escaped. When it stops, either `|z|^2>4`, in which case `i` is
the step at which it escaped, or `i` reached `m` without escaping. A sample
scores `i` in the first case and `2*m` in the second, the same as Ruby's
`find`. The cell's score `a` sums its four samples, so a cell entirely inside
the set scores `8*m`.

```lua
  v[j]=-a*(1-a//(8*m)*U)*n*n*2;
  d[j]=(a<8*m)and(0)or(n)
  o[j+1]=j;
  g[j]=0;
  j=j+1;
end;
```

Rather than keep the raw score, the loop stores the first part of the sort
key straight away. `-a*(1-a//(8*m)*U)` is Ruby's `-v[j]*(1-v[j]/(8*m)*u)`:
with `U=0` it is `-a`, so higher scores sort first and the interior leads;
with `U=1` an interior cell's value becomes 0 and sorts after every exterior
cell. The `*n*n*2` makes room for the other two parts of the key, explained
under Ranking.

`d` starts at 0 for cells that are not fully inside and at `n` (more than any
real distance) for interior ones. `(c)and(x)or(y)` is Lua's ternary; it works
here because 0 is truthy in Lua, as in Ruby.

`o` starts as the cells in reading order, and `g` as all zeros.

## Distance to the edge

```lua
k=1;
while(k>-2)do;
  i=0;
  while(i<n)do;
    j=(n-1)*(1-k)//2+k*i;
    if((j-k)//w==j//w)then;d[j]=z(d[j],d[j-k]+1)end;
    d[j]=z(d[j],(d[j-k*w]or(n))+2)
    i=i+1;
  end;
  k=k-2;
end;
```

The two-pass distance transform from the Ruby frame: `k=1` sweeps forward
looking left and up, `k=-1` sweeps backward looking right and down. `k` goes
1, -1, and the loop ends at -3. `(n-1)*(1-k)//2+k*i` is `i` going forward and
`n-1-i` going backward. A horizontal step costs 1 and a vertical step 2,
matching the 1:2 shape of a cell.

`(j-k)//w==j//w` keeps the horizontal neighbour on the same row. Floor
division makes it false for `j-k=-1` and for `j-k=n`, so `d[j-k]` is only read
when it exists.

The vertical neighbour `j-k*w` can be off the top or bottom of the frame.
Ruby checks the bounds; Lua gets it for free, because reading a missing table
entry gives `nil` and `nil or n` is `n`. `n+2` is never smaller than `d[j]`,
so an out-of-frame neighbour never changes anything, exactly as if it had
been skipped.

## Ranking

```lua
table.sort(o,function(a,b)return(v[a]+d[a]*n+a<v[b]+d[b]*n+b)end)
```

Ruby sorts by the triple `[score part, d[j], j]`. Lua's `table.sort` takes a
comparison function, and comparing triples would need three branches. So the
triple is packed into one integer: `v[j]+d[j]*n+j`.

This only works if packing preserves the order. `d[j]` is between 0 and `n`
and `j` is between 0 and `n-1`, so `d[j]*n+j` is less than `n*n+n`, which is
less than `2*n*n`. The score parts are multiples of `2*n*n` (that is the
`*n*n*2` above), so two cells with different score parts are always ordered
by them, and only cells with equal score parts are ordered by distance, then
by index. Since every cell has a different `j`, no two keys are equal, the
order is total, and it does not matter that `table.sort` is not stable.

The largest key is about `16*m*n*n`. For a 300x100 frame with a few thousand
iterations that is around 10^14, comfortably inside a 64-bit integer.

## Choosing the cells

```lua
t=s:gsub("%s","")
l=#t+29;
```

`t` is the body without whitespace (assigning a two-value call to one name
keeps the first value). `l` is the length of the whole program: the body plus
the 4 characters of head and 25 of tail.

```lua
f=n;
e=0;
i=1;
while(i<=l)do;
  j=o[i]
  g[j]=1;
  f=z(f,j)
  e=-z(-e,-j)
  i=i+1;
end;
```

Mark the `l` best cells with 1, keeping track of the first (`f`) and last
(`e`) of them. Ruby finds those afterwards with `index` and `rindex`; doing
it during the marking is shorter in Lua.

```lua
f=z(f,f-f%w+w-4)
e=-z(-e,e%w-e-24)
```

Make room for the head and tail on one row each. `f-f%w+w-4` is the last
column in `f`'s row where the 4-character head still fits; if `f` is further
right it moves left. `e-e%w+24` is column 24 of `e`'s row, the first place
the 25-character tail can end; if `e` is further left it moves right. That
one is a maximum, written as a negated minimum: `-z(-e,e%w-e-24)`.

```lua
c=l;
i=0;
while(i<25)do;
  j=f+i%4;
  c=c+1-z(g[j],1)
  g[j]=2;
  j=e-i;
  c=c+1-z(g[j],1)
  g[j]=2;
  i=i+1;
end;
```

Pin the head's 4 cells and the tail's 25, marking them 2. One loop does both:
`f+i%4` cycles over the head's cells (setting each several times is
harmless), `e-i` walks back over the tail's.

`c` counts marked cells. It starts at `l` and goes up by one each time a
cell that was 0 becomes 2. `1-z(g[j],1)` is 1 for an unmarked cell and 0 for
one already marked 1 or 2, so a head cell pinned for the second time is not
counted again. Ruby counts the marks afterwards instead; again, this is
shorter.

```lua
i=l;
while(i>0)do;
  j=o[i]
  if(c>l)and(g[j]==1)then;g[j]=0;c=c-1;end;
  i=i-1;
end;
```

Pinning may have marked up to 29 cells beyond the best `l`. Walk the chosen
cells from worst to best and unmark ordinary ones (never the 2s) until
exactly `l` remain. The first and last marked cells are pinned, so they stay
put.

## Printing

```lua
b=""
j=0;
while(j<n)do;
  b=b..((g[j]>0)and"#"or"\32")..((j%w==w-1)and"\n"or"")
  j=j+1;
end;
```

Build a mask of the frame: `#` for a cell that gets a character, a space for
one that does not, and a newline after the last cell of each row. `"\32"` is
a decimal escape for a space; a literal space would be stripped along with
everything else. The body sits in a long string, where escapes are not
processed, so `s` holds the four characters `\32` and the escape is only
interpreted once `load` compiles the body.

```lua
e="s=[".."["..t.."]"..']load(s:gsub("%s",""))()'
```

`e` is the whole program without whitespace: head, body, tail. The head and
tail are spelled in pieces, `"s=[".."["` and `"]"..']load...'`, because
writing `[[` or `]]` in the body would end the long string it lives in. The
second piece uses single quotes so the double quotes inside need no escaping.

```lua
k=0;
io.write((b:gsub("\32*\n","\n"):gsub("#",function()k=k+1;return(e:sub(k,k))end)))
```

Two substitutions on the mask finish the job. The first deletes the spaces
at the end of each row, Ruby's `rstrip`. The second replaces each `#`, in
order, with the next character of `e`; `gsub` calls the function once per
match from left to right, and the text it returns is not scanned again, so a
`#` in the program (there is one, in `#t`) does no harm. The outer
parentheses drop `gsub`'s count so that `io.write` prints only the frame.

The last row ends with a newline like every other, which is how the viewer
writes the frame too.

## Why the output is the input

The viewer computed the frame with the same steps in Rust: same literals,
same samples, same scores, same ranking, same pins. When Lua runs the frame
it reads those literals, repeats the computation, arrives at the same set of
cells, and fills them with the same characters in the same order.

As with Ruby, whitespace in the input does not affect what runs, so any
arrangement of the same characters with the head and tail intact prints the
canonical frame, and the canonical frame prints itself.

The viewer's V key runs the frame through `lua` and compares the output with
the frame byte for byte, and `cargo test` does the same for a handful of
views in both placements.
