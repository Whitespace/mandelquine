# The C frame, line by line

This walks through the program the viewer shows when the language is C. The
body is `viewer/src/lang/c.c`, written with line breaks for reading. The
viewer strips every whitespace character from it, substitutes the view's
numbers for the capitalised placeholders, wraps it, and pours it into the
shape of the set. Compiling the result with `cc` and running it prints the
same text back.

The Ruby walkthrough ([ruby.md](ruby.md)) explains the scoring, distance and
ranking steps in more detail. The C body does the same arithmetic in the same
order, so this document concentrates on what is different: how a C program
gets hold of its own source without `eval`, and how it can be broken into a
fractal shape when C, unlike Ruby's `%w`, does not tolerate whitespace inside
a token.

## The wrapper

```c
#define Q(...)char*s=#__VA_ARGS__;__VA_ARGS__
Q( ...the body, broken up by spaces and newlines... )
```

`Q` is a variadic macro. Its expansion uses the arguments twice:

- `#__VA_ARGS__` turns them into a string literal, and `char*s=` stores it.
- `__VA_ARGS__` pastes them back in as ordinary code.

So the body is compiled once as code, and the program also gets its text in
`s`. When the preprocessor stringifies, it deletes whitespace before the
first token and after the last, and turns every run of whitespace between
tokens into exactly one space. Newlines, indentation and the gaps the layout
leaves all become single spaces, and dropping the spaces from `s` gives the
body with no whitespace at all, the same string whatever the shape was. That
is the whole quine trick, and it is the C counterpart of Ruby's
`%w(...)*""`.

Variadic (`...`) rather than `Q(a)` because the body contains commas outside
parentheses, for instance between declarators, and a one-parameter macro
would treat those as argument separators.

Like Ruby's body, the C body has constraints because all of its whitespace
is removed:

- No two identifier-like tokens can be adjacent, since `int x` would become
  `intx`. The body uses parenthesised declarators instead: `int(w)=110` and
  `int(main)()` are ordinary C. Function bodies avoid `else if` and
  `return x` for the same reason (`return(x)` is fine).
- No string or character literals except one, because a space inside a
  literal would be part of its value. Characters the output needs, such as
  `"`, `\`, space and newline, are written as numbers: 34, 92, 32, 10.
- No `->`, `-=` or `/*` (see breaking below).

The directive itself has one space that cannot be removed, between `define`
and `Q`. The program spells the directive with `/**/` there instead; a
comment counts as whitespace in a directive, and it has no space in it.

## The view

```c
char*t="#define/**/Q(...)char*s=#__VA_ARGS__;__VA_ARGS__";
int(putchar)(int);
int(w)=W,(h)=H,(m)=M,(n),(i),(j),(k),(q),(x),(y),(a),(z),(Z),(L),(N),(A),(B),(R),(b),
(v)[1<<18],(d)[1<<18],(p)[1<<18],(G)[1<<18];
double(f)=X0,(g)=Y0,(c)=SC,(o)[1<<18],(P),(I),(E),(T),(V);
int(u)=U;
_Atomic(double)(F),(K);
char(e)[1<<16],(O)[1<<18];
```

`t` is the directive, which the program has to print but which is not part
of the macro's argument, so it is not in `s`. It is the body's only string
literal.

`putchar` is declared by hand instead of including `<stdio.h>`, because
`#include` would need a line of its own. Calling an undeclared function is
an error in current gcc and clang.

In a real frame the placeholders are literals, for example
`int(w)=110,(h)=48,(m)=100` and `double(f)=-7.5e-1,(g)=0e0,(c)=3.09e-2`, with
`int(u)=1`. They mean what they mean in Ruby: frame size, iteration limit,
centre `f + g i`, cell width `c` (a row is `2*c` high), and `u` for code
outside (1) or inside (0) the set. The floats are Rust's shortest
round-trip form, which C parses to the same double.

Everything else is a global so it starts at zero. The arrays are sized for
2^18 cells, enough for a 500x500 terminal; the viewer refuses larger frames.

## Scoring every cell

```c
n=w*h;
for(j=0;j<n;j++){
for(q=0;q<4;q++){
F=(j%w+q%2*.5+.25-w*.5)*c;P=f+F;
F=(h*.5-j/w-q/2*.5-.25)*2*c;I=g+F;
for(E=T=i=0;F=E*E,K=T*T,F+K<=4&&i++<m;){V=F-K+P;F=2*E*T;T=F+I;E=V;}
v[j]+=i>m?2*m:i;
}
d[j]=v[j]<8*m?0:n;
}
```

The same 2x2 supersampled escape count as Ruby: `P + I i` is the sample,
`E + T i` is `z`, and a sample scores the step at which `|z|^2 > 4`, or `2*m`
if it never escapes. A fully interior cell scores `8*m`.

The loop condition does the escape test before each step. Tracing it: `i`
is incremented only after `|z|^2 <= 4` passes, so if `z` after step `i`
escapes, the next test fails with `i` still equal to that step. If all `m`
steps pass, `i++<m` fails with `i` at `m+1`, which is how `i>m` means "never
escaped".

`F` and `K` are `_Atomic(double)`. That is not about threads. A compiler is
allowed to fuse `a*b+c` into one fused multiply-add instruction, which skips
the rounding of `a*b` and can change the result in the last bit. Clang does
this by default within an expression on Apple silicon, and gcc does it
across statements whenever the target has FMA instructions, which every
arm64 CPU does. The Rust viewer never fuses, and a single
bit of difference near the edge of the set changes which cells get code.
Storing every product into an atomic forces it to be rounded to a double
before it is used, because a compiler does not optimise through atomic
loads and stores. `volatile` would do the same job more cheaply, but
`volatile double` is two adjacent words. The cost is about four times the
run time at deep zoom on x86, where an atomic store is a locked
instruction; on a shallow view it is a few milliseconds.

The last line sets up the distance transform: 0 for cells that are not
fully inside, `n` (more than any distance) for interior ones.

## Distance to the edge

```c
for(k=1;k>-2;k=k-2)for(i=0;i<n;i++){
j=(n-1)*(1-k)/2+k*i;
if((j-k+w)/w==(j+w)/w&&d[j]>d[j-k]+1)d[j]=d[j-k]+1;
q=j-k*w;
if(q>=0&&q<n&&d[j]>d[q]+2)d[j]=d[q]+2;
}
```

The two-pass chamfer distance from the Ruby walkthrough: forward with
`k=1`, backward with `k=-1`, horizontal steps cost 1, vertical steps 2.

The one change is the same-row test. Ruby writes `(j-k)/w==j/w`, which
relies on integer division rounding down so that `-1/w` is -1. C rounds
toward zero, so `-1/w` is 0 and the first cell would look at `d[-1]`. Adding
`w` to both sides keeps everything non-negative, where the two kinds of
division agree.

## Ranking

```c
for(j=0;j<n;j++)o[j]=((v[j]*(v[j]/8/m*u-1)+8*m)*262144.+d[j])*262144+j;
for(q=n/2;q;q=q/2)for(i=q;i<n;i++){P=o[i];for(k=i;k>=q&&o[k-q]>P;k=k-q)o[k]=o[k-q];o[k]=P;}
for(i=0;i<n;i++)G[(long)o[i]%262144]=i;
```

Ruby sorts by the triple `[-v*(1-v/(8m)*u), d, j]`. C has no tuple sort,
and `qsort` would need `<stdlib.h>` or a long hand-written declaration, so
the triple is packed into one number. The first part is shifted by `8*m` to
make it non-negative, and each part gets 18 bits (262144 = 2^18). The total
stays below 2^53, so a `double` holds it exactly, and comparing the packed
keys compares the triples. Because `j` is part of the key, no two keys are
equal and the order is fully determined.

The sort is a Shell sort, the shortest sort that is fast enough. Then `G`
is turned around into a rank: `G[j]` is the cell's place in the order, so
"the best `L` cells" is simply every cell with `G[j]<L`. The low 18 bits of
the key are the cell index.

## The program as one string

```c
for(i=0;t[i];i++)if(t[i]>32)e[Z++]=t[i];
A=Z;e[Z++]=81;e[Z++]=40;
for(i=0;s[i];i++)if(s[i]>32)e[Z++]=s[i];
e[Z++]=41;
for(i=Z;i--;)if(e[i]==34){B=i;if(!R)R=i;}
```

`e` is the whole program, `Z` characters long, with no whitespace: the
directive `t`, then `Q(` (81 and 40), then the body recovered from `s`, then
`)`. Both copies are filtered with `>32`, which drops the spaces the layout
puts inside `t` and the ones stringification puts in `s`.

Three positions are remembered for the layout:

- `A`, where `Q(` starts, the first character after the directive.
- `B` and `R`, the opening and closing quotes of `t` inside the body. The
  directive has no quotes, so they are the first and last `"` in `e`.

## Where the program may break

```c
int(C)(int(z)){return(z-48<10u||(z|32)-97<26u||z==95||z==46||z==45);}
int(D)(int(k)){return(!k||k>B&&k<=R||k-12&&C(e[k-1])!=C(e[k]));}
int(S)(int(k)){return(k>0&&k<A||k>B&&k<=R);}
```

Ruby's body can be broken anywhere. C's can't: `while` with a space in it is
two identifiers. So the layout needs to know where whitespace may go.

`C(z)` sorts characters into two classes: "word" characters (digits,
letters, `_`, `.` and `-`) and everything else. `z-48<10u` is the digit test
written with an unsigned comparison so that one test covers both ends, and
`(z|32)-97<26u` does the same for letters of either case.

`D(k)` says whether whitespace may go just before `e[k]`, which is the same
as saying a token starts there:

- At the start, and anywhere inside `t`'s literal (more on that below).
- Between two characters of different classes. Inside the body that is
  always a token boundary. `.` and `-` count as word characters so numbers
  like `3.09e-2` and expressions like `j-k` stay in one piece, which is why
  the body can't use `->` or `-=`: they would be split.
- Never between two characters of the same class. That is sometimes too
  strict (`)(` could be split) but never wrong.
- Never at position 12, between `Q` and `(` in the directive. A space there
  would make `Q` an object-like macro.

`S(k)` says whether a line break just before `e[k]` would be fatal. That is
true inside the directive (`0<k<A`), since a directive ends at the end of its
line, and inside `t`'s literal, since a string literal can't contain a
newline. In those places a row ends with a backslash instead: backslash
followed by newline is deleted before the compiler sees anything else, so
the row joins onto the next. Inside the literal the next row's leading
spaces become part of `t`, and the `>32` filter discards them. In the
directive they become spaces between tokens, which is harmless.

## Pouring the program into `L` cells

```c
int(J)(int(L)){
for(j=n;j--;)p[j]=(G[j]<L)*(j%w<w-1?p[j+1]+1:1);
for(k=y=N=0;y<h;y++){
for(a=-1,x=0;x<w;x++)if(G[y*w+x]<L)a=x;
for(x=0;x<w;x++){
j=y*w+x;O[j]=32;
if(G[j]<L&&k<Z&&(k-A||y-N)){
if(D(k)){
for(q=1;k+q<Z&&!D(k+q);q++);
O[j]=x==a&&S(k)?92:q<=p[j]-(S(k+q)&&j+p[j]-1==y*w+a)?(N=y,e[k++]):32;
}else{N=y;O[j]=e[k++];}
}
}
if(a<0&&S(k))O[y*w]=92;
}
return(k==Z);
}
```

`J(L)` tries to lay the program out in the best `L` cells and returns
whether all of it fit. The frame goes into `O`, one character per cell.

`p[j]` is the length of the run of chosen cells starting at `j` and going
right to the end of the row; it is computed backwards so each cell can add
one to its right neighbour. `a` is the column of the last chosen cell on the
current row, or -1 if the row has none. `k` is the next character of `e` to
place, and `N` the row of the last character placed.

Walking the chosen cells in reading order:

- If the next character starts a token, `q` is that token's length.
  - If this is the row's last chosen cell and a line break here would be
    fatal, the cell gets a backslash and the token waits for the next row.
  - Otherwise the token is placed only if it fits in what is left of this
    run. If the row would end straight after the token in a place where a
    bare line break is fatal, one more cell is needed for the backslash.
  - If it doesn't fit, the cell stays blank and the token tries the next
    run.
- If the next character is inside a token, the token already fits, so it
  goes straight in.

`(k-A||y-N)` keeps the rest of the directive's last row empty, so the
directive ends at the end of that row and `Q(` starts a new line. And a row
with no chosen cells at all, in the middle of the directive or the literal,
still gets a backslash in its first column so the line keeps going.

Blank cells inside the body are whitespace between tokens, blank cells inside
`t` are filtered out, and every row that could not end with a newline ends
with a backslash, so whatever `J` produces compiles to the same program.

## Choosing `L`

```c
for(i=Z-1,b=Z,z=1;!J(b);z=z*2)i=b,b=i+z<n?i+z:n;
for(;b-i>1;)J(z=(i+b)/2)?b=z:(i=z);
J(b);
```

Every token that doesn't fit at the end of a run wastes cells, so the
program needs somewhat more than `Z` cells, and how many depends on the
shape. Too many and the bottom of the shape is left blank.

The first line gallops: try `Z` cells, then `Z+1`, `Z+3`, `Z+7` and so on,
capped at `n`, until the program fits. `i` is the last count that didn't fit
and `b` the first that did. The second line bisects between them until they
are adjacent. The third pours once more with `b` so that `O` holds that
layout. This takes about 20 pours instead of the hundreds a count-up would.

Fitting isn't strictly monotonic in `L` (one more cell can shift a token
onto a different row), so this finds a count that fits right after one that
doesn't, not necessarily the smallest. That is fine: the viewer does the
same search and gets the same answer.

## Printing

```c
for(y=0;y<h;y++){for(a=w;a&&O[y*w+a-1]==32;a--);for(x=0;x<a;x++)putchar(O[y*w+x]);putchar(10);}
```

Each row without its trailing spaces, then a newline.

## Why the output is the input

The viewer (`viewer/src/lang/c.rs`) runs the same steps in Rust: same
literals, same scores, same ranking, same token classes, same search. When
the compiled frame runs, it reads those literals, repeats the computation
and fills the same cells with the same characters.

As with Ruby, the input doesn't have to be laid out correctly to work. Any
arrangement of the same tokens that keeps the directive on its own logical
line and the literal intact prints the canonical frame.

## Compiling

The viewer's V key compiles the frame with `cc -O2` into a scratch
directory, runs the binary and compares its output with the frame byte for
byte. Timings on the cloud build box (gcc 13, x86), for a frame of the whole
set at 110x48:

| | compile | run |
| --- | --- | --- |
| `cc -O0` | 40 ms | 20 ms |
| `cc -O2` | 110 ms | 14 ms |

Compiling dominates on shallow views, and `-O0` would be quicker there. At
deep zoom the iteration count goes into the thousands and the run time
dominates: a 200x60 frame at `m=1360` took 2.0 s at `-O0` and 1.6 s at
`-O2`, with the atomic stores the main cost. `-O2` is the better default
across the range. Either way, a V check on a shallow view takes well under
the 130 ms it takes Ruby just to run its frame.

The frame needs no flags. It was checked with gcc and clang at `-O0`, `-O2`
and `-O3 -march=native -ffp-contract=fast`, and with `-std=c11` and C23.
