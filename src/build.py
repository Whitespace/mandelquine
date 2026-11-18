"""Build seed from body.rb, run one full cycle + 1, verify it closes, render GIF + stills."""
import subprocess, sys, pathlib, re, time
from PIL import Image, ImageDraw, ImageFont

here = pathlib.Path(__file__).parent
frames = here / "frames"
frames.mkdir(exist_ok=True)
N = 60

body = "".join((here / "body.rb").read_text().split())
seed = 'eval(s=%w(' + body + ')*"")\n'
(here / "seed.rb").write_text(seed)
print("code chars:", len(body))

srcs = []
cur = here / "seed.rb"
t0 = time.time()
for i in range(N + 1):
    r = subprocess.run(["ruby", str(cur)], capture_output=True, text=True)
    if r.returncode or r.stderr:
        print("frame", i, "FAILED rc", r.returncode, "\n", r.stderr)
        sys.exit(1)
    cur = frames / f"f{i:02d}.rb"
    cur.write_text(r.stdout)
    srcs.append(r.stdout)
print("total run time %.1fs" % (time.time() - t0))

print("CYCLE CLOSES (gen 60 == gen 0 byte-identical):", srcs[N] == srcs[0])
print("distinct among gens 0..59:", len(set(srcs[:N])))
norm = lambda t: re.sub(r"k=-?\d+", "k=K", "".join(t.split()), count=1)
ref = norm(seed)
for i, t in enumerate(srcs[:N]):
    j = "".join(t.split())
    assert j.startswith(f"eval(s=%w(k={i};"), i
    code, _, fill = j[:-5].partition("#")
    assert norm(code + ')*"")') == ref, i
    lines = t.split("\n")
    assert len(lines) == 49 and max(map(len, lines)) <= 110, (i, len(lines))
    assert "eval(s=%w(" in t and ')*"")' in t, i
    print(f"{i}:{len(j)}ch/{len(fill) + (1 if _ else 0)}fill", end="  " if (i + 1) % 8 else "\n")
print("\nall frames: code identical modulo k")

W, H = 110, 48
grid = lambda t: [[c != " " for c in l.ljust(W)] for l in t.split("\n")[:H]]
gs = [grid(t) for t in srcs[:N]]
flips = [sum(a != b for ra, rb in zip(gs[i], gs[(i + 1) % N]) for a, b in zip(ra, rb)) for i in range(N)]
print("cells flipped per step (last entry is the wrap 59->0):", flips)

font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 12)
cw, ch = font.getbbox("M")[2], 15
imgs = []
for t in srcs[:N]:
    im = Image.new("L", (cw * W + 16, ch * H + 16), 12)
    d = ImageDraw.Draw(im)
    for j, l in enumerate(t.split("\n")[:H]):
        d.text((8, 8 + j * ch), l, font=font, fill=230)
    imgs.append(im)
imgs[0].save(here / "zoom.gif", save_all=True, append_images=imgs[1:], duration=80, loop=0)
