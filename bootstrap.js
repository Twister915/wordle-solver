// noinspection JSUnresolvedFunction

import './style.scss';

import("./dist/wasm").then(module => {
    module.run_app();
});