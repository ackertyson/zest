#!/usr/bin/env bash

for i in $(seq 0 255); do
    printf '\033[38;5;%dm%3d█\033[0m ' $i $i
    if (( i % 16 == 15 )); then echo; fi
done
