#!/usr/bin/env ruby
# Builds mandelquine.rb from src/body.rb and verifies one full cycle.
# Run from anywhere: `ruby src/build.rb`.
require "open3"
require "rbconfig"
require "fileutils"

ROOT   = File.expand_path("..", __dir__)
TMP    = File.join(ROOT, "tmp")
CYCLE  = 120
WIDTH  = 110
HEIGHT = 48
HEAD   = "eval(s=%w("
TAIL   = ')*"")'

FileUtils.mkdir_p(TMP)

# The body is written one statement per line for readability. None of that
# whitespace is meaningful, so stripping all of it gives the real program.
body = File.read(File.join(__dir__, "body.rb")).split.join
seed = HEAD + body + TAIL + "\n"
puts "code: #{body.size} characters"

# The seed has k=-1 and no layout. Running it yields generation 0.
gens = []
src = seed
path = File.join(TMP, "build.rb")
(CYCLE + 1).times do |i|
  File.write(path, src)
  out, err, status = Open3.capture3(RbConfig.ruby, path)
  abort "generation #{i} failed:\n#{err}" unless status.success? && err.empty?
  gens << out
  src = out
end

def check(ok, msg)
  abort "FAILED: #{msg}" unless ok
end

strip   = ->(text) { text.split.join }
unstamp = ->(code) { code.sub(/k=-?\d+/, "k=K") }
want    = unstamp.(strip.(seed))

check gens[CYCLE] == gens[0], "generation #{CYCLE} is not identical to generation 0"
check gens.first(CYCLE).uniq.size == CYCLE, "generations within a cycle are not all distinct"

padded = []
gens.first(CYCLE).each_with_index do |text, i|
  flat = strip.(text)
  code, hash, _pad = flat[0...-TAIL.size].partition("#")
  check flat.start_with?("#{HEAD}k=#{i};"), "generation #{i} has the wrong counter"
  check unstamp.(code + TAIL) == want, "generation #{i}'s code differs from the seed"
  check text.include?(HEAD) && text.include?(TAIL), "generation #{i} has a split wrapper"
  lines = text.chomp.split("\n", -1)
  check lines.size == HEIGHT && lines.map(&:size).max <= WIDTH, "generation #{i} is not #{WIDTH}x#{HEIGHT}"
  padded << i unless hash.empty?
end
puts "cycle closes: generation #{CYCLE} == generation 0, #{CYCLE} distinct generations"
puts "generations padded with a comment: #{padded.empty? ? 'none' : padded.join(' ')}"

File.write(File.join(ROOT, "mandelquine.rb"), gens[0])
puts "wrote mandelquine.rb"
