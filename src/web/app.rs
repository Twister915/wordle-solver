use instant::{Duration, Instant};
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};
use crate::web::solver_agent::{GameStateDto, ScoredCandidateDto, SolverAgent, SolverReq, SolverResp};
use crate::wordle::SolverErr;

pub struct App {
    worker: Box<dyn Bridge<SolverAgent>>,

    game_state: Option<GameStateDto>,
    recommendations: RecommendationState,
    latest_err: Option<SolverErr>,
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
        }
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        log::debug!("app msg {:?}", &msg);
        use Msg::*;
        match msg {
            SolverMsg(data) => self.handle_solver_msg(data),
            PickRecommendation(_) => false,
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
}