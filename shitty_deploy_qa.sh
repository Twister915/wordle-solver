#!/usr/bin/env bash
set -e

trunk build --release

scp -r dist/* joey@mgk1.joey.sh:~/wordle-qa-html/;