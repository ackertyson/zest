#!/opt/homebrew/bin/fish

for i in (seq 0 255); printf '\033[38;5;%dm%3d█\033[0m ' $i $i; if test (math $i % 16) -eq 15; echo; end; end
