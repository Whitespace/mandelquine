#!/usr/bin/env ruby
# Runs mandelquine.rb through one full cycle and records it two ways:
#   mandelquine.cast  asciicast v2, for `asciinema play`
#   mandelquine.svg   animated SVG, which GitHub renders in the README
require "json"
require "open3"
require "rbconfig"
require "fileutils"

ROOT   = File.expand_path("..", __dir__)
TMP    = File.join(ROOT, "tmp")
CYCLE  = 120
WIDTH  = 110
HEIGHT = 48
FRAME  = 0.08 # seconds per generation

FileUtils.mkdir_p(TMP)

gens = [File.read(File.join(ROOT, "mandelquine.rb"))]
path = File.join(TMP, "cast.rb")
CYCLE.times do |i|
  File.write(path, gens.last)
  out, err, status = Open3.capture3(RbConfig.ruby, path)
  abort "generation #{i + 1} failed:\n#{err}" unless status.success? && err.empty?
  gens << out
end
abort "cycle does not close; run src/build.rb" unless gens[CYCLE] == gens[0]

# --- asciicast: show generation 0, type the command, play one cycle ---------
events = []
clock = 0.0
emit = lambda do |data, pause|
  events << [clock.round(3), "o", data]
  clock += pause
end
screen = ->(text) { text.chomp.gsub("\n", "\r\n") }

emit.("\e[2J\e[H\e[?25l", 0)
emit.("$ cat mandelquine.rb\r\n", 0.6)
emit.(screen.(gens[0]), 2.5)
emit.("\e[2J\e[H$ ", 0.4)
"./run.sh".each_char { |ch| emit.(ch, 0.05) }
emit.("\r\n", 0.6)
gens.drop(1).each { |text| emit.("\e[H\e[2J" + screen.(text), FRAME) }
clock += 2.0
emit.("\e[?25h", 0)

header = { version: 2, width: WIDTH, height: HEIGHT + 1, title: "mandelquine" }
rows = ([header] + events).map { |row| JSON.generate(row) }
File.write(File.join(ROOT, "mandelquine.cast"), rows.join("\n") + "\n")
puts "wrote mandelquine.cast (#{clock.round(1)}s)"

# --- animated SVG: one group per generation, shown in turn ------------------
# Every line gets an explicit textLength, so the grid stays aligned whatever
# monospace font the viewer has.
CW  = 6.0  # cell width
CH  = 12.0 # cell height
PAD = 10.0
escape = ->(s) { s.gsub("&", "&amp;").gsub("<", "&lt;").gsub(">", "&gt;") }
total  = (CYCLE * FRAME).round(2)
slot   = (100.0 / CYCLE).round(4)

svg = []
svg << %(<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 #{(WIDTH * CW + 2 * PAD).to_i} #{(HEIGHT * CH + 2 * PAD).to_i}" xml:space="preserve">)
svg << "<style>"
svg << "text{font:10px ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;fill:#d8dee9;white-space:pre}"
svg << "g{visibility:hidden;animation:f #{total}s step-end infinite}"
svg << "@keyframes f{0%{visibility:visible}#{slot}%{visibility:hidden}}"
svg << "</style>"
svg << %(<rect width="100%" height="100%" fill="#0d1117"/>)
gens.first(CYCLE).each_with_index do |text, i|
  svg << %(<g style="animation-delay:#{(i * FRAME).round(2)}s">)
  text.chomp.split("\n", -1).each_with_index do |line, row|
    body = line.lstrip
    next if body.empty?
    x = PAD + (line.size - body.size) * CW
    y = PAD + row * CH + 9
    svg << %(<text x="#{x}" y="#{y}" textLength="#{body.size * CW}">#{escape.(body)}</text>)
  end
  svg << "</g>"
end
svg << "</svg>"
File.write(File.join(ROOT, "mandelquine.svg"), svg.join("\n") + "\n")
puts "wrote mandelquine.svg (#{CYCLE} frames, #{total}s loop)"
