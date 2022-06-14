#!/usr/bin/env bash
set -e

rm -fr target/ dist/

trunk build --release

scp -r dist/* joey@mgk1.joey.sh:~/wordle-qa-html/;