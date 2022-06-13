use yew::prelude::*;
use std::borrow::Borrow;
use crate::wordle::{Coloring, Colorings, Guess, is_wordle_str, N_RECOMMENDATIONS, NUM_TURNS, ScoredCandidate, Solver, WORD_SIZE};

pub struct App {
    solver: Solver<'static>,
    recommendations: Vec<ScoredCandidate<'static>>,
    filled_guess: [Option<char>; WORD_SIZE],
    filled_colors: [Coloring; WORD_SIZE],
}

#[derive(Debug)]
pub enum Msg {
    PickRecommendation(String),
    UpdateColoring(usize),
    MakeGuess,
    ClearGuess,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        let mut out = Self {
            solver: Solver::default(),
            recommendations: Vec::default(),
            filled_guess: [None; WORD_SIZE],
            filled_colors: [Coloring::Excluded; WORD_SIZE],
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
            },
            UpdateColoring(idx) => {
                let src = &mut self.filled_colors[idx];
                let mut next_coloring = match *src {
                    Coloring::Excluded => Coloring::Misplaced,
                    Coloring::Misplaced => Coloring::Correct,
                    Coloring::Correct => Coloring::Excluded,
                };
                std::mem::swap(src, &mut next_coloring);
                true
            }
            MakeGuess => self.make_guess(),
            ClearGuess => {
                if self.enable_reset_button() {
                    let reset_entire_game = !self.has_any_guess_state();
                    if reset_entire_game {
                        self.solver.reset();
                        self.update_recommendations();
                    }

                    self.clear_guess();
                    true
                } else {
                    false
                }
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="body">
                {self.show_content_html(ctx)}
                {self.show_recommendation_html(ctx)}
            </div>
        }
    }
}

impl App {
    fn update_recommendations(&mut self) {
        self.recommendations.clear();
        self.recommendations.extend(self.solver.top_k_guesses::<{ N_RECOMMENDATIONS }>());
    }

    fn show_content_html(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="content">
                {Self::show_title_html()}
                {self.show_game(ctx)}
            </div>
        }
    }

    fn show_title_html() -> Html {
        html! {
            <div class="title">
                <div class="hero">{"Joey's Wordle Bot"}</div>
                <div class="detail">
                    <div>
                        <>{"Solves "}</>
                        <a href="https://www.nytimes.com/games/wordle/index.html" target="_blank" class="click-text">{"Wordle"}</a>
                        <>{" by recommending guesses & updating as you play!"}</>
                    </div>
                    <div>
                        {"Click on a recommendation, then click on the boxes below to change their color, then hit OK when the colors match the game you're playing"}
                    </div>
                </div>
            </div>
        }
    }

    fn show_recommendation_html(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="suggestions">
                <div class="title">{format!("Suggestions ({})", self.recommendations.len())}</div>
                { self.show_recommendation_details() }
                {
                    if let Some(top_guess) = self.recommendations.get(0) {
                        Self::show_recommendation_instructions(ctx, top_guess.word)
                    } else {
                        html! {<></> }
                    }
                }
                { self.show_recommendation_list(ctx) }
            </div>
        }
    }

    fn show_recommendation_details(&self) -> Html {
        html! {
            <div class="detail">
                <div class="possibilities">
                    {format!(
                        "{}/{} possible words remaining...",
                        self.solver.num_remaining_possibilities(),
                        self.solver.num_total_possibilities(),
                    )}
                </div>
                <div class="entropy">
                    {format!(
                        "{:.02} bits of entropy",
                        self.solver.remaining_entropy(),
                    )}
                </div>
            </div>
        }
    }

    fn show_recommendation_instructions(ctx: &Context<Self>, top_guess: &'static str) -> Html {
        html! {
            <div class="instructions">
                <>{"You can click on any word below to guess it, or you can "}</>
                <span class="click-text" onclick={ctx.link().callback(move |_| Msg::PickRecommendation(top_guess.to_string()))}>{"click here"}</span>
                <>{format!(" to pick the best guess ({})", top_guess)}</>
            </div>
        }
    }

    fn show_recommendation_list(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="list">
                {
                    self.recommendations.iter()
                        .enumerate()
                        .map(|(idx, item)| Self::show_recommendation_item(idx, item, ctx))
                        .collect::<Html>()
                }
            </div>
        }
    }

    fn show_recommendation_item(idx: usize, item: &ScoredCandidate<'static>, ctx: &Context<Self>) -> Html {
        let word_cloned = item.word.clone();
        html! {
            <div class="item" onclick={ctx.link().callback(move |_| Msg::PickRecommendation(word_cloned.to_string()))}>
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
        let guesses: Vec<Guess> = self.solver.iter_guesses().collect();
        html! {
            <div class="game">
                {(0..NUM_TURNS).map(|idx| self.show_wordle_row(ctx, &guesses, idx)).collect::<Html>()}
            </div>
        }
    }

    fn show_wordle_row(&self, ctx: &Context<Self>, guesses: &Vec<Guess>, idx: usize) -> Html {
        if let Some(guess) = guesses.get(idx) {
            self.show_wordle_guessed_row(guess)
        } else if idx == guesses.len() && self.solver.can_guess() {
            self.show_wordle_active_row(ctx)
        } else {
            self.show_wordle_empty_row()
        }
    }

    fn show_wordle_guessed_row(&self, guess: &Guess) -> Html {
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
            </div>
        }
    }

    fn show_wordle_active_row(&self, ctx: &Context<Self>) -> Html {
        let active_idx = self.next_chr_idx();
        log::debug!("active_idx = {:?}", &active_idx);
        html! {
            <div class="game-row active">
                {
                    self.filled_guess.iter().copied().zip(self.filled_colors.iter()).enumerate().map(|(idx, (chr, coloring))| html! {
                        <div class={classes!(
                            "game-cell",
                            active_idx.filter(|a_idx| *a_idx == idx).map(|_| "active").unwrap_or("inactive"),
                            chr.map(|_| "filled").unwrap_or("unfilled"),
                            match coloring {
                                Coloring::Excluded => "c-excluded",
                                Coloring::Misplaced => "c-misplaced",
                                Coloring::Correct => "c-correct",
                            })}
                            onclick={ctx.link().callback(move |_| Msg::UpdateColoring(idx))}
                        >
                            { chr.unwrap_or(' ') }
                        </div>
                    }).collect::<Html>()
                }
                <div class="buttons">
                    <div class={classes!(
                        "button",
                        "reset-button",
                        if self.enable_reset_button() { "enabled" } else { "disabled" },
                    )} onclick={ctx.link().callback(move |_| Msg::ClearGuess)}>{"X"}</div>
                    <div class={classes!(
                        "button",
                        "confirm-button",
                        if self.enable_confirm_button() { "enabled" } else { "disabled" },
                    )} onclick={ctx.link().callback(move |_| Msg::MakeGuess)}>{"OK"}</div>
                </div>
            </div>
        }
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
        debug_assert!(is_wordle_str(suggestion));

        let bs = suggestion.as_bytes();
        for (src, target) in bs.iter().copied().zip(self.filled_guess.iter_mut()) {
            *target = Some(src as char);
        }
    }

    fn make_guess(&mut self) -> bool {
        let guess_str = if let Some(g) = self.guess_str() {
            g
        } else {
            return false;
        };

        let colorings = Colorings(self.filled_colors.clone());
        if let Err(err) = self.solver.make_guess(&guess_str, colorings) {
            log::warn!("weird error when guessing {:?} {:?}", guess_str, err);
        }

        self.clear_guess();
        if !self.solver.can_guess() {
            self.solver.reset();
        }
        self.update_recommendations();
        true
    }

    fn guess_str(&self) -> Option<String> {
        let mut guess= [0; WORD_SIZE];
        for i in 0..WORD_SIZE {
            if let Some(c) = *&self.filled_guess[i] {
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

    fn enable_reset_button(&self) -> bool {
        self.has_any_guess_state() || self.solver.num_guesses() > 0
    }

    fn enable_confirm_button(&self) -> bool {
        if let Some(g) = self.guess_str() {
            self.solver.can_use_guess(g.borrow())
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
}