/*
 * MIT License
 *
 * Copyright (c) 2022 Joseph Sacchini
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use super::global_key_hook::*;
use crate::wordle::*;
use std::borrow::Borrow;
use yew::prelude::*;

pub struct App {
    solver: StaticSolver,
    recommendations: Vec<ScoredCandidate<'static>>,
    filled_guess: [Option<char>; WORD_SIZE],
    filled_colors: [Coloring; WORD_SIZE],

    #[allow(dead_code)]
    keydown_listener: KeyListener,
}

#[derive(Debug, Clone)]
pub enum Msg {
    PickRecommendation(String),
    UpdateColoring(usize),
    MakeGuess,
    ClearGuess,
    OnKeyDown(KeyEvent),
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let mut out = Self {
            solver: Solver::default(),
            recommendations: Vec::default(),
            filled_guess: [None; WORD_SIZE],
            filled_colors: [Coloring::Excluded; WORD_SIZE],
            keydown_listener: KeyListener::create(ctx.link().callback(Msg::OnKeyDown))
                .expect("should be able to attach key listener"),
        };
        out.update_recommendations();
        out
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        log::debug!("app msg {:?}", &msg);
        use Msg::*;
        match msg {
            PickRecommendation(recommendation) => {
                self.accept_suggestion(recommendation.as_str());
                true
            }
            UpdateColoring(idx) => {
                if self.solver.can_guess() {
                    let src = &mut self.filled_colors[idx];
                    let mut next_coloring = match *src {
                        Coloring::Excluded => Coloring::Misplaced,
                        Coloring::Misplaced => Coloring::Correct,
                        Coloring::Correct => Coloring::Excluded,
                    };
                    std::mem::swap(src, &mut next_coloring);
                    true
                } else {
                    false
                }
            }
            MakeGuess => self.make_guess(),
            ClearGuess => {
                if self.enable_reset_button() {
                    let reset_entire_game = !self.has_any_guess_state();
                    if reset_entire_game {
                        self.reset();
                    }

                    self.clear_guess();
                    true
                } else {
                    false
                }
            }
            OnKeyDown(mut event) => self.handle_keydown(&mut event),
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="viewport">
                <div class="body">
                    {self.show_game(ctx)}
                    {self.show_recommendation_html(ctx)}
                </div>
                { Self::show_footer_safe() }
            </div>
        }
    }
}

impl App {
    fn update_recommendations(&mut self) {
        self.recommendations.clear();
        self.recommendations
            .extend(self.solver.top_k_guesses::<{ N_RECOMMENDATIONS }>());
    }

    fn show_info_html() -> Html {
        html! {
            <div class="info">
                <h2>{"Instructions"}</h2>
                <p class="instructions">
                    {"To solve a wordle puzzle, you follow these simple steps:"}
                    <ol class="steps">
                        <li>{"Click on a Suggestion on the sidebar (or type a word)"}</li>
                        <li>{"Guess the Suggestion in your Wordle game"}</li>
                        <li>{
                            "Input the colors that Wordle gave to your guess by clicking on the \
                            squares. Each click will change to the next color."
                        }</li>
                        <li>
                            {"Hit the ✔️ button once the colors match those provided by Wordle"}
                        </li>
                    </ol>
                    {"Suggestions will be updated after you make each guess, until the puzzle \
                    is solved."}
                </p>

                <h2>{"Methodology"}</h2>
                <p>
                    <>{"Math based on "}</>
                    { Self::show_link(
                        "https://www.youtube.com/watch?v=v68zYyaEmEA",
                        "Grant Sanderson (3blue1brown)'s Video")
                    }
                    <>{" about using "}</>
                    { Self::show_link(
                        "https://en.wikipedia.org/wiki/Entropy_(information_theory)",
                        "Information Theory")
                    }
                    <>{" to solve Wordle."}</>
                </p>
                <p>
                    <>{"To suggest a "}</>
                    <em>{"good guess"}</em>
                    <>{" we basically calculate how much 'information' is gained, on average, for \
                    a given guess. A guess will receive a 'coloring' and these colorings eliminate \
                    possible answers. A 'high information' coloring is one where the coloring \
                    eliminates the most candidate answers. Guesses which often produce high \
                    information colorings are ranked highly, and suggested to the user."}</>
                </p>
                <p>
                    <>{"The "}</>
                    <em>{"expected information"}</em>
                    <>{" is combined with data about the frequency of word use in English, so that \
                    we suggest words that are likely to be the answer"}</>
                </p>
                <p>
                    <>{"This summary is incomplete and oversimplified and it is highly recommended \
                    you watch the "}</>
                    { Self::show_link("https://www.youtube.com/watch?v=v68zYyaEmEA", "video" )}
                    <>{" which visualizes the scoring computation quite well."}</>
                </p>
            </div>
        }
    }

    fn show_recommendation_html(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="suggestions">
                <div class="title">{format!("Suggestions ({})", self.num_suggestions())}</div>
                if self.solver.can_guess() {
                    { self.show_recommendation_details() }
                    {
                        if let Some(top_guess) = self.recommendations.get(0) {
                            Self::show_recommendation_instructions(ctx, top_guess.word)
                        } else {
                            html! {<></> }
                        }
                    }
                }
                { self.show_recommendation_list(ctx) }
            </div>
        }
    }

    fn show_recommendation_details(&self) -> Html {
        html! {
            <div class="detail">
                <div class="possibilities">{ self.possibilities_remaining_msg() }</div>
                <div class="entropy">
                    {format!(
                        "Entropy is {:.02} bits",
                        self.solver.remaining_entropy(),
                    )}
                </div>
            </div>
        }
    }

    fn possibilities_remaining_msg(&self) -> String {
        let remaining = self.num_suggestions();
        let total = self.solver.num_total_possibilities();
        let count_str = if remaining == total {
            total.to_string()
        } else {
            format!("{}/{}", remaining, total)
        };

        format!("{} possible words remaining...", count_str)
    }

    fn num_suggestions(&self) -> usize {
        if self.solver.can_guess() {
            self.solver.num_remaining_possibilities()
        } else {
            0
        }
    }

    fn show_recommendation_instructions(ctx: &Context<Self>, top_guess: &'static str) -> Html {
        html! {
            <div class="instructions">
                <>{"You can click on any word below to guess it, or you can "}</>
                <span
                    class="click-text"
                    onclick={ ctx
                        .link()
                        .callback(move |_|
                            Msg::PickRecommendation(top_guess.to_string()))}>
                    {"click here"}
                </span>
                <>{format!(" to pick the best guess ({})", top_guess)}</>
            </div>
        }
    }

    fn show_recommendation_list(&self, ctx: &Context<Self>) -> Html {
        let empty_list = !self.solver.can_guess() || self.recommendations.is_empty();
        html! {
            <div class="list">
                if empty_list {
                    <div class="empty-msg">
                        <>{"This game is complete, press the X button to reset, or "}</>
                        <span
                            class="click-text"
                            onclick={ ctx
                                .link()
                                .callback(move |_|
                                    Msg::ClearGuess)}>
                            {"click here"}
                        </span>
                        <>{" to reset..."}</>
                    </div>
                } else {
                    {self.recommendations
                        .iter()
                        .enumerate()
                        .map(|(idx, item)| Self::show_recommendation_item(idx, item, ctx))
                        .collect::<Html>()}
                }
            </div>
        }
    }

    fn show_recommendation_item(
        idx: usize,
        item: &ScoredCandidate<'static>,
        ctx: &Context<Self>,
    ) -> Html {
        html! {
            <div
                class="item"
                onclick={ ctx
                    .link()
                    .callback(|_| Msg::PickRecommendation(item.word.to_string()))}>
                <div class="ordinal">{format!("#{:02}", idx + 1)}</div>
                <div class="word">{&item.word}</div>
                <div class="details">
                    <span class="score">{format!("{:.2}", item.score.abs)}</span>
                    <span class="expected-info">{format!("{:.2}", item.score.expected_info)}</span>
                    <span class="weight">{format!("{:.4}", item.score.weight)}</span>
                </div>
            </div>
        }
    }

    fn show_game(&self, ctx: &Context<Self>) -> Html {
        let guesses: Vec<&Guess> = self.solver.iter_guesses().collect();
        html! {
            <div class="game-ctr">
                <h1 class="title">
                    <img alt="[W]" src="static/favicon.png" class="logo" />
                    {"Joey's Wordle Solver"}
                </h1>
                <p class="tagline">
                    <>{"Solves "}</>
                    {Self::show_link("https://www.nytimes.com/games/wordle/index.html", "Wordle")}
                    <>{" by suggesting guesses & updating as you play!"}</>
                </p>
                <div class="game">
                    {
                        (0..NUM_TURNS)
                            .map(|idx| self.show_wordle_row(ctx, &guesses, idx))
                            .collect::<Html>()
                    }
                </div>
                {Self::show_info_html()}
            </div>
        }
    }

    fn show_wordle_row(&self, ctx: &Context<Self>, guesses: &[&Guess], idx: usize) -> Html {
        if let Some(guess) = guesses.get(idx) {
            self.show_wordle_guessed_row(ctx, guess, idx)
        } else if idx == guesses.len() {
            self.show_wordle_active_row(ctx)
        } else {
            self.show_wordle_empty_row()
        }
    }

    fn show_wordle_guessed_row(&self, ctx: &Context<Self>, guess: &Guess, index: usize) -> Html {
        let show_reset = index == NUM_TURNS - 1;
        html! {
            <div class="game-row filled inactive">
                {
                    (0..WORD_SIZE).zip(guess.word.iter().copied()).map(|(idx, chr)| html! {
                        <div class={classes!(
                            "game-cell",
                            "filled",
                            match guess.coloring[idx] {
                                Coloring::Excluded => "c-excluded",
                                Coloring::Misplaced => "c-misplaced",
                                Coloring::Correct => "c-correct",
                            }
                        )}>{chr as char}</div>
                    }).collect::<Html>()
                }
                if show_reset {
                    <div class="buttons">
                        {self.show_reset_button(ctx)}
                    </div>
                }
                <div class="entropy">
                    {format!(
                        "{:.02} bits (expected {:.02} bits)",
                        guess.entropy_delta,
                        guess.expected_info)}
                </div>
            </div>
        }
    }

    fn show_wordle_active_row(&self, ctx: &Context<Self>) -> Html {
        let active_idx = self.next_chr_idx();
        let can_play = self.solver.can_guess();
        log::debug!("active_idx = {:?}", &active_idx);
        html! {
            <div class={classes!("game-row", if can_play { "active" } else { "empty inactive" })}>
                {
                    self.filled_guess
                        .iter()
                        .copied()
                        .zip(self.filled_colors.iter())
                        .enumerate()
                        .map(|(idx, (chr, coloring))| html! {
                            <div class={classes!(
                                "game-cell",
                                active_idx
                                    .filter(|a_idx| *a_idx == idx)
                                    .map(|_| "active")
                                    .unwrap_or("inactive"),
                                chr.map(|_| "filled").unwrap_or("unfilled"),
                                match coloring {
                                    Coloring::Excluded => "c-excluded",
                                    Coloring::Misplaced => "c-misplaced",
                                    Coloring::Correct => "c-correct",
                                })}
                                onclick={ctx.link().callback(move |_| Msg::UpdateColoring(idx))}>
                                { chr.unwrap_or(' ') }
                            </div>
                        }).collect::<Html>()
                }
                <div class="buttons">
                    {self.show_reset_button(ctx)}
                    if can_play {
                        {self.show_confirm_button(ctx)}
                    }
                </div>
            </div>
        }
    }

    fn wordle_button(
        ctx: &Context<Self>,
        c: &'static str,
        emoji: &'static str,
        enabled: bool,
        msg: Msg,
    ) -> Html {
        html! {
            <div
                class={classes!("button", c, if enabled { "enabled" } else { "disabled" })}
                onclick={ctx.link().callback(move|_| msg.clone())}>
                {emoji}
            </div>
        }
    }

    fn show_reset_button(&self, ctx: &Context<Self>) -> Html {
        Self::wordle_button(
            ctx,
            "reset-button",
            "❌",
            self.enable_reset_button(),
            Msg::ClearGuess,
        )
    }

    fn show_confirm_button(&self, ctx: &Context<Self>) -> Html {
        Self::wordle_button(
            ctx,
            "confirm-button",
            "✔️",
            self.enable_confirm_button(),
            Msg::MakeGuess,
        )
    }

    fn show_wordle_empty_row(&self) -> Html {
        html! {
            <div class="game-row empty inactive">
                {
                    (0..WORD_SIZE).map(|_| html! {
                        <div class="game-cell empty inactive"></div>
                    }).collect::<Html>()
                }
            </div>
        }
    }

    fn next_chr_idx(&self) -> Option<usize> {
        for (idx, c) in self.filled_guess.iter().enumerate() {
            if c.is_none() {
                return Some(idx);
            }
        }

        None
    }

    fn accept_suggestion(&mut self, suggestion: &str) {
        assert!(is_wordle_str(suggestion));

        let bs = suggestion.as_bytes();
        for (src, target) in bs.iter().copied().zip(self.filled_guess.iter_mut()) {
            *target = Some(src as char);
        }
    }

    fn make_guess(&mut self) -> bool {
        if !self.solver.can_guess() {
            return false;
        }

        let guess_str = if let Some(g) = self.guess_str() {
            g
        } else {
            return false;
        };

        if !self.solver.is_guess_permitted(&guess_str) {
            return false;
        }

        let colorings = Colorings(self.filled_colors);
        if let Err(err) = self.solver.make_guess(&guess_str, colorings) {
            log::warn!("weird error when guessing {:?} {:?}", guess_str, err);
        }

        self.clear_guess();
        self.update_recommendations();
        self.pre_fill_answer();
        true
    }

    fn guess_str(&self) -> Option<String> {
        let mut guess = [0; WORD_SIZE];
        #[allow(clippy::needless_range_loop)]
        for i in 0..WORD_SIZE {
            if let Some(c) = self.filled_guess[i] {
                guess[i] = c as u8;
            } else {
                return None;
            }
        }

        Some(String::from_utf8_lossy(&guess).to_string())
    }

    fn clear_guess(&mut self) {
        self.filled_guess = [None; WORD_SIZE];
        self.filled_colors = [Coloring::Excluded; WORD_SIZE];
    }

    fn pre_fill_answer(&mut self) {
        if !self.solver.can_guess() {
            return;
        }

        // if there's literally just one recommendation, then it's the right answer
        if let Some(only) = self.recommendations.get(0) {
            if self.recommendations.len() == 1 {
                let only_word = &*only.word;
                self.accept_suggestion(only_word);
                self.filled_colors = [Coloring::Correct; WORD_SIZE];
                return;
            }
        }

        // copy forward green squares from the previous guess, if there are any guesses
        if let Some(prev_guess) = self.solver.iter_guesses().last() {
            for i in 0..WORD_SIZE {
                let coloring = prev_guess.coloring.0[i];
                if coloring == Coloring::Correct {
                    self.filled_guess[i] = Some(prev_guess.word[i] as char);
                    self.filled_colors[i] = coloring;
                }
            }
        }
    }

    fn enable_reset_button(&self) -> bool {
        self.has_any_guess_state() || self.solver.num_guesses() > 0
    }

    fn enable_confirm_button(&self) -> bool {
        if let Some(g) = self.guess_str() {
            self.solver.is_guess_permitted(g.borrow())
        } else {
            false
        }
    }

    fn has_any_guess_state(&self) -> bool {
        self.next_chr_idx() != Some(0) || self.has_any_coloring_state()
    }

    fn has_any_coloring_state(&self) -> bool {
        self.filled_colors.iter().any(|c| c != &Coloring::Excluded)
    }

    fn show_footer_safe() -> Html {
        #[cfg(debug_assertions)]
        html! {
            <div class="footer debug">
                {"Joey's Wordle Solver -- DEBUG BUILD -- NOT FOR PRODUCTION"}
            </div>
        }

        #[cfg(not(debug_assertions))]
        Self::show_footer()
    }

    #[cfg(not(debug_assertions))]
    fn show_footer() -> Html {
        html! {
            <div class="footer">
                <>{format!("Joey's Wordle Solver -- v{} -- built with ", crate::GIT_VERSION)}</>
                {Self::show_link("https://www.rust-lang.org/", "Rust")}
                <>{" and "}</>
                {Self::show_link("https://yew.rs/", "Yew")}
                <>{". Available on "}</>
                <span class="coming-soon">{"GitHub"}</span>
                <>{" (coming soon!)."}</>
            </div>
        }
    }

    fn show_link(target: &'static str, text: &'static str) -> Html {
        html! {
            <a href={target} target="_blank" rel="noopener noreferrer" class="click-text">
                {text}
            </a>
        }
    }

    fn handle_keydown(&mut self, event: &mut KeyEvent) -> bool {
        if event.is_control_key() {
            return false;
        }

        match event.code() {
            "Backspace" => self.handle_backspace(event),
            "Enter" => self.handle_enter(event),

            // all letter keys are of the form "KeyA" or "KeyB" etc
            code if code.starts_with("Key") && code.len() == 4 => {
                let l = code.as_bytes()[3] as char;
                if ('A'..='Z').contains(&l) {
                    self.handle_letter_entered(event, l.to_ascii_lowercase())
                } else {
                    false
                }
            }
            other => {
                log::debug!("unhandled key {}", other);
                false
            }
        }
    }

    fn handle_letter_entered(&mut self, event: &mut KeyEvent, letter: char) -> bool {
        if !self.solver.can_guess() {
            return false;
        }
        // next_chr_idx = where the next character should go
        match self.next_chr_idx() {
            // None means that the word is completely filled & we have nowhere to put the character
            None => false,

            Some(idx) => {
                self.filled_guess[idx] = Some(letter);

                // if this character is the same as the previous guess & previously it was correct,
                // then we can automatically label this as green
                if let Some(previous) = self.solver.iter_guesses().last() {
                    let p_coloring = previous.coloring[idx];
                    let p_letter = previous.word[idx] as char;

                    if p_coloring == Coloring::Correct && p_letter == letter {
                        self.filled_colors[idx] = Coloring::Correct;
                    }
                }
                event.prevent_default();
                true
            }
        }
    }

    fn handle_backspace(&mut self, event: &mut KeyEvent) -> bool {
        if !self.solver.can_guess() {
            return false;
        }
        // figure out what index to clear...
        // next_chr_idx is the index where a new character would go
        let idx_clear = match self.next_chr_idx() {
            // if the next character goes at the start, then that means there's no characters
            // entered, and we should do nothing
            Some(0) => return false,

            Some(idx) => idx,

            // if next_char_idx returns None, the guess is filled (all 5 chars are entered)
            // and we need to clear the last character
            None => self.filled_guess.len(),
        } - 1; // we subtract 1 from the next_chr_idx because this index is after the last filled character

        self.filled_guess[idx_clear] = None;
        self.filled_colors[idx_clear] = Coloring::Excluded;
        event.prevent_default();
        true
    }

    fn handle_enter(&mut self, event: &mut KeyEvent) -> bool {
        // enter will either... submit the current answer (if possible), or if the game is over,
        // it will reset the game (think of it as a shortcut to hitting the X button)
        let out = self.make_guess()
            || if !self.solver.can_guess() {
                self.reset();
                true
            } else {
                false
            };

        if out {
            event.prevent_default();
        }

        out
    }

    fn reset(&mut self) {
        self.solver.reset();
        self.update_recommendations();
    }
}
