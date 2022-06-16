# ![W](static/favicon.png) Joey's Wordle Solver

This is a [Rust](https://rust-lang.org) + [Yew](https://yew.rs/) [WASM](https://webassembly.org/) web-app which 
implements an interactive [Wordle](https://www.nytimes.com/games/wordle/index.html) solver, described originally by
[Grant Sanderson (3blue1brown)](https://www.youtube.com/c/3blue1brown)'s 
[YouTube video on solving wordle](https://youtu.be/v68zYyaEmEA) with 
[information theory](https://en.wikipedia.org/wiki/Information_theory).

It's running live on my site, at [https://wordle.joey.sh](https://wordle.joey.sh) where you can try it out. It's best 
viewed on a desktop, and works in recent versions of all major browsers.

![screencap](./doc/screencap_wordle_site.jpg)

The mathematics are described in code documentation, and the webpage is supposed to be styled like a notebook & as such
also documents the mathematics/methodology.

## Building

The site is built with [trunk](https://trunkrs.dev/). It is recommended to run the [setup.sh](./setup.sh) script to 
install the required tools.

You must have the following installed:
* [Rust](https://rust-lang.org) (check out: [rustup](https://rustup.rs/) to get this working), and obviously a working 
  cargo installation.
* wasm32-unknown-unknown target for Rust (`rustup target add wasm32-unknown-unknown`)
* [wasm-opt](https://github.com/MrRefactoring/wasm-opt) which requires [node.js](https://nodejs.org/en/)
* [sass](https://sass-lang.com/) which can be installed from npm.

The `setup.sh` script only requires that `cargo` and `npm` be installed.

Trunk is capable of installing some (perhaps all) of the tools it depends on (like sass, wasm-opt, etc). However, I 
develop on an M1 Mac (in June 2022) and trunk does not correctly install required tools because the architecture is not 
supported. This is why I have created `setup.sh`. It is possible that simply installing trunk and trying to build will
be sufficient to build this project.