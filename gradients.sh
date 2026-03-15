#!/bin/sh
# Color swatches shared across zest animations

swatch() {
    name=$1; shift
    printf '%-15s' "$name"
    for c in "$@"; do
        printf '\033[38;5;%dm█' "$c"
    done
    printf '\033[0m\n'
}

shine_swatch() {
    printf '%-15s' "shine"
    fgs="231 226 220 214 178"
    bgs="100  58 238 237 236"
    n=5
    i=1
    for fg in $fgs; do
        bg=$(echo $bgs | cut -d' ' -f$i)
        printf '\033[38;5;%dm\033[48;5;%dm█' "$fg" "$bg"
        i=$((i+1))
    done
    printf '\033[0m\n'
}

swatch "green"   157 120  83  46  40  34  28  22
swatch "orange"  226 220 214 208 202 196 160  88
swatch "blue"    231 195 159 123  87  51  45  39  33  27  21  18  17
swatch "purple"  219 213 207 201 165 129  93  57  55
swatch "pink"    198 198 198 198 198 198
swatch "red"     196 160 124  88  52
swatch "scan"    231 195 189 183
shine_swatch
