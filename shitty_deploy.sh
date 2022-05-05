#!/usr/bin/env bash
set -e

rm -fr target/ dist/

trunk build --release

scp dist/* joey@mgk1.joey.sh:~/wordle-html/;