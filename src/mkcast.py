"""Run the quine N generations and write an asciicast v2 file of the sources."""
import json, pathlib, subprocess, sys, tempfile

quine, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
N = int(sys.argv[3]) if len(sys.argv) > 3 else 60
W, H, DT = 110, 48, 0.12

frames = [quine.read_text()]
with tempfile.TemporaryDirectory() as d:
    cur = pathlib.Path(d) / "ruby.rb"
    for _ in range(N):
        cur.write_text(frames[-1])
        r = subprocess.run(["ruby", str(cur)], capture_output=True, text=True, check=True)
        assert not r.stderr, r.stderr
        frames.append(r.stdout)

for f in frames:
    lines = f.rstrip("\n").split("\n")
    assert len(lines) <= H and max(map(len, lines)) <= W, (len(lines), max(map(len, lines)))

ev, t = [], 0.0
def emit(s, dt):
    global t
    ev.append([round(t, 3), "o", s]); t += dt

cmd = "while ruby ruby.rb | sponge ruby.rb; do clear; cat ruby.rb; done"
emit("\x1b[2J\x1b[H\x1b[?25l", 0)
emit("$ cat ruby.rb\r\n", 0.6)
emit(frames[0].rstrip("\n").replace("\n", "\r\n"), 2.5)
emit("\x1b[2J\x1b[H$ ", 0.4)
for ch in cmd:
    emit(ch, 0.035)
emit("\r\n", 0.6)
for f in frames[1:]:
    emit("\x1b[H\x1b[2J" + f.rstrip("\n").replace("\n", "\r\n"), DT)
t += 2.0
emit("\x1b[?25h", 0)

hdr = {"version": 2, "width": W, "height": H + 1, "title": "Zooming Mandelbrot quine (Ruby)", "env": {"TERM": "xterm-256color", "SHELL": "/bin/bash"}}
with out.open("w") as fh:
    fh.write(json.dumps(hdr) + "\n")
    for e in ev:
        fh.write(json.dumps(e) + "\n")
print("frames", len(frames), "events", len(ev), "duration %.1fs" % t, "bytes", out.stat().st_size)
