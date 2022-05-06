use std::collections::HashSet;
use instant::Instant;
use yew_agent::{Agent, AgentLink, HandlerId, Public};
use serde::{Serialize, Deserialize};
use crate::wordle::{ColoringsArray, Guess, ScoredCandidate, Solver, SolverErr, WORD_SIZE, WordleFloat};

pub const N_RECOMMENDATIONS: usize = 24;

pub struct SolverAgent {
    link: AgentLink<Self>,
    subscribers: HashSet<HandlerId>,
    solver: Solver<'static>,

    cached_recommendations: Option<Vec<ScoredCandidateDto>>,
    cached_state: Option<GameStateDto>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SolverReq {
    Init,
    Reset,
    MakeGuess(GuessDto),
    MakeRecommendations,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum SolverResp {
    UpdateRecommendations(Vec<ScoredCandidateDto>),
    StartComputingRecommendations,
    UpdateGameState(GameStateDto),
    GuessFailed(SolverErr),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct GameStateDto {
    pub guesses: Vec<GuessDto>,
    pub uncertainty: WordleFloat,
    pub solved: bool,
    pub can_guess: bool,
}

impl GameStateDto {
    fn with_solver(solver: &Solver) -> Self {
        Self {
            guesses: solver.iter_guesses().map(|g| g.into()).collect(),
            uncertainty: solver.uncertainty(),
            solved: solver.is_solved(),
            can_guess: solver.can_guess(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct GuessDto {
    pub guess: [u8; WORD_SIZE],
    pub colorings: ColoringsArray,
}

impl From<Guess> for GuessDto {
    fn from(other: Guess) -> Self {
        Self {
            guess: other.word,
            colorings: other.coloring.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScoredCandidateDto {
    pub word: String,
    pub score: WordleFloat,
    pub expected_info: WordleFloat,
    pub weight: WordleFloat,
}

impl From<ScoredCandidate<'_>> for ScoredCandidateDto {
    fn from(other: ScoredCandidate<'_>) -> Self {
        Self {
            word: other.word.to_string(),
            score: other.score.abs,
            expected_info: other.score.expected_info,
            weight: other.score.weight,
        }
    }
}

impl Agent for SolverAgent {
    type Reach = Public<Self>;
    type Message = ();
    type Input = SolverReq;
    type Output = SolverResp;

    fn create(link: AgentLink<Self>) -> Self {
        log::debug!("creating solver agent...");
        let start_at = Instant::now();
        let solver = Solver::default();
        let setup_time = start_at.elapsed();
        log::debug!("solver setup in {:.02}s", setup_time.as_secs_f64());
        Self {
            link,
            subscribers: HashSet::with_capacity(8),
            solver,

            cached_recommendations: None,
            cached_state: None,
        }
    }

    fn update(&mut self, _: Self::Message) {}

    fn connected(&mut self, id: HandlerId) {
        self.subscribers.insert(id);
    }

    fn handle_input(&mut self, msg: Self::Input, _: HandlerId) {
        use SolverReq::*;
        log::debug!("worker msg {:?}", &msg);
        match msg {
            Init => {
                self.reset();
                self.send_recommendations();
            }
            Reset => self.reset(),
            MakeGuess(guess) => self.handle_guess(guess),
            MakeRecommendations => self.send_recommendations(),
        }
    }

    fn disconnected(&mut self, id: HandlerId) {
        self.subscribers.remove(&id);
        if self.subscribers.is_empty() {
            self.reset();
        }
    }

    fn name_of_resource() -> &'static str {
        "worker.js"
    }
}

impl SolverAgent {
    fn handle_guess(&mut self, guess: GuessDto) {
        if let Err(err) = self.solver.make_guess(guess.guess.as_str(), guess.colorings.into()) {
            self.broadcast(SolverResp::GuessFailed(err));
        } else {
            self.invalidate();
            self.send_game_state();
        }
    }

    fn send_recommendations(&mut self) {
        let recommendations = self.recommendations().clone();
        self.broadcast(SolverResp::UpdateRecommendations(recommendations));
    }

    fn recommendations(&mut self) -> &Vec<ScoredCandidateDto> {
        if self.cached_recommendations.is_none() {
            self.broadcast(SolverResp::StartComputingRecommendations);

            self.cached_recommendations = Some(self.solver
                .top_k_guesses::<N_RECOMMENDATIONS>()
                .map(|item| item.into())
                .collect());
        }

        self.cached_recommendations.as_ref().unwrap()
    }

    fn invalidate(&mut self) {
        self.invalidate_recommendations();
        self.invalidate_game_state();
    }

    fn invalidate_recommendations(&mut self) {
        self.cached_recommendations = None;
    }

    fn send_game_state(&mut self) {
        let msg = SolverResp::UpdateGameState(self.game_state().clone());
        self.broadcast(msg);
    }

    fn game_state(&mut self) -> &GameStateDto {
        if self.cached_state.is_none() {
            self.cached_state = Some(GameStateDto::with_solver(&self.solver));
        }

        self.cached_state.as_ref().unwrap()
    }

    fn invalidate_game_state(&mut self) {
        self.cached_state = None;
    }

    fn reset(&mut self) {
        self.solver.reset();
        self.invalidate();
        self.send_game_state();
        self.broadcast(SolverResp::UpdateRecommendations(Vec::default()));
    }

    fn broadcast(&self, msg: SolverResp) {
        log::debug!("broadcast {:?}", msg);
        for sub in &self.subscribers {
            self.link.respond(*sub, msg.clone());
        }
    }
}
