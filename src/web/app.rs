use instant::{Duration, Instant};
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};
use crate::web::solver_agent::{GameStateDto, GuessDto, ScoredCandidateDto, SolverAgent, SolverReq, SolverResp};
use crate::wordle::{Coloring, is_wordle_str, NUM_TURNS, SolverErr, WORD_SIZE};

pub struct App {
    worker: Box<dyn Bridge<SolverAgent>>,

    game_state: Option<GameStateDto>,
    recommendations: RecommendationState,
    latest_err: Option<SolverErr>,
    filled_guess: [Option<char>; WORD_SIZE],
    filled_colors: [Coloring; WORD_SIZE],
}

#[derive(Debug)]
struct RecommendationState {
    loading: RecommendationLoadStatus,
    current: Vec<ScoredCandidateDto>,
}

#[derive(Debug)]
enum RecommendationLoadStatus {
    Loading(Instant),
    Completed(Duration)
}

impl Default for RecommendationLoadStatus {
    fn default() -> Self {
        Self::Loading(Instant::now())
    }
}

impl RecommendationLoadStatus {
    fn is_loading(&self) -> bool {
        match self {
            Self::Loading(_) => true,
            _ => false,
        }
    }

    fn is_ready(&self) -> bool {
        !self.is_loading()
    }

    fn start(&mut self) -> bool {
        let mut old = Self::default();
        std::mem::swap(self, &mut old);
        old.is_ready()

    }

    fn finish(&mut self) -> bool {
        match self {
            Self::Loading(start_at) => {
                let elapsed = start_at.elapsed();
                *self = Self::Completed(elapsed);
                true
            },
            _ => false,
        }
    }
}

impl Default for RecommendationState {
    fn default() -> Self {
        Self {
            loading: RecommendationLoadStatus::default(),
            current: Vec::default(),
        }
    }
}

#[derive(Debug)]
pub enum Msg {
    SolverMsg(SolverResp),
    PickRecommendation(String),
    UpdateColoring(usize),
    MakeGuess,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let mut worker = SolverAgent::bridge(ctx.link().callback(Msg::SolverMsg));
        worker.send(SolverReq::Init);
        Self {
            worker,
            game_state: None,
            recommendations: RecommendationState::default(),
            latest_err: None,
            filled_guess: [None; WORD_SIZE],
            filled_colors: [Coloring::Excluded; WORD_SIZE],
        }
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        log::debug!("app msg {:?}", &msg);
        use Msg::*;
        match msg {
            SolverMsg(data) => self.handle_solver_msg(data),
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
            MakeGuess => self.make_guess()
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="body">
                {self.show_center_html(ctx)}
                {self.show_recommendation_html(ctx)}
            </div>
        }
    }
}

impl App {
    fn handle_solver_msg(&mut self, msg: SolverResp) -> bool {
        use SolverResp::*;
        match msg {
            UpdateRecommendations(recommendations) => {
                self.recommendations.loading.finish();
                self.recommendations.current = recommendations;
                self.latest_err = None;
                true
            }
            StartComputingRecommendations => {
                self.recommendations.loading.start()
            }
            UpdateGameState(new_state) => {
                self.latest_err = None;
                self.game_state
                    .replace(new_state)
                    .and_then(|old|
                        self.game_state.as_ref()
                            .map(|new| new != &old))
                    .unwrap_or(true)
            }
            GuessFailed(err) => {
                self.latest_err = Some(err);
                true
            }
        }
    }

    fn show_center_html(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="center">
                {Self::show_title_html()}
                {self.show_game(ctx)}
            </div>
        }
    }

    fn show_title_html() -> Html {
        html! {
            <div class="title">
                {"Joey's Wordle Bot"}
            </div>
        }
    }

    fn show_recommendation_html(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="suggestions">
                {
                    match &self.recommendations.loading {
                        RecommendationLoadStatus::Loading(_) => self.show_recommendation_loading(),
                        RecommendationLoadStatus::Completed(dur) => self.show_recommendation_load_time(dur),
                    }
                }
                { self.show_recommendation_list(ctx) }
            </div>
        }
    }

    fn show_recommendation_loading(&self) -> Html {
        html! {
            <div class="loading">{"loading..."}</div>
        }
    }

    fn show_recommendation_load_time(&self, dur: &Duration) -> Html {
        html! {
            <div class="load-time">{format!("loaded in {:.02}s...", dur.as_secs_f64())}</div>
        }
    }

    fn show_recommendation_list(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="list">
                {
                    self.recommendations.current.iter()
                        .enumerate()
                        .map(|(idx, item)| Self::show_recommendation_item(idx, item, ctx))
                        .collect::<Html>()
                }
            </div>
        }
    }

    fn show_recommendation_item(idx: usize, item: &ScoredCandidateDto, ctx: &Context<Self>) -> Html {
        let word_cloned = item.word.clone();
        html! {
            <div class="item" onclick={ctx.link().callback(move |_| Msg::PickRecommendation(word_cloned.clone()))}>
                <div class="ordinal">{format!("#{:02}", idx + 1)}</div>
                <div class="word">{&item.word}</div>
                <div class="details">
                    <span class="score">{format!("{:.2}", item.score)}</span>
                    <span class="expected-info">{format!("{:.2}", item.expected_info)}</span>
                    <span class="weight">{format!("{:.4}", item.weight)}</span>
                </div>
            </div>
        }
    }

    fn show_game(&self, ctx: &Context<Self>) -> Html {
        match &self.game_state {
            Some(state) => self.show_wordle_game(ctx, state),
            None => html! { <></> },
        }
    }

    fn show_wordle_game(&self, ctx: &Context<Self>, state: &GameStateDto) -> Html {
        html! {
            <div class="game">
                {(0..NUM_TURNS).map(|idx| self.show_wordle_row(ctx, state, idx)).collect::<Html>()}
            </div>
        }
    }

    fn show_wordle_row(&self, ctx: &Context<Self>, state: &GameStateDto, idx: usize) -> Html {
        if let Some(guess) = state.guesses.get(idx) {
            self.show_wordle_guessed_row(ctx, state, guess, idx)
        } else if idx == state.guesses.len() && state.can_guess {
            self.show_wordle_active_row(ctx, state, idx)
        } else {
            self.show_wordle_empty_row()
        }
    }

    fn show_wordle_guessed_row(&self, ctx: &Context<Self>, state: &GameStateDto, guess: &GuessDto, idx: usize) -> Html {
        html! {
            <div class="game-row filled inactive">
                {
                    (0..WORD_SIZE).zip(guess.guess.iter().copied()).map(|(idx, chr)| html! {
                        <div class="game-cell filled inactive">{chr as char}</div>
                    }).collect::<Html>()
                }
            </div>
        }
    }

    fn show_wordle_active_row(&self, ctx: &Context<Self>, state: &GameStateDto, idx: usize) -> Html {
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
                    <div class="reset-button button">{"X"}</div>
                    <div class="confirm-button button" onclick={ctx.link().callback(move |_| Msg::MakeGuess)}>{"OK"}</div>
                </div>
            </div>
        }
    }

    fn show_wordle_empty_row(&self) -> Html {
        html! {
            <div class="game-row empty inactive">
                {
                    (0..WORD_SIZE).map(|idx| html! {
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

    fn can_guess(&self) -> bool {
        self.filled_guess.iter().all(|v| v.is_some())
    }

    fn make_guess(&mut self) -> bool {
        let mut guess= [0; WORD_SIZE];
        for i in 0..WORD_SIZE {
            if let Some(c) = *&self.filled_guess[i] {
                guess[i] = c as u8;
            } else {
                return false;
            }
        }

        self.worker.send(SolverReq::MakeGuess(GuessDto {
            guess,
            colorings: *&self.filled_colors,
        }));

        self.worker.send(SolverReq::MakeRecommendations);

        self.filled_guess = [None; WORD_SIZE];
        self.filled_colors = [Coloring::Excluded; WORD_SIZE];
        true
    }
}