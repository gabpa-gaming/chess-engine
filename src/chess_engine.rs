use crate::board_config::BoardConfig;
use crate::chess_board::{Chessboard, CurrentPlayer, GameStatus};
use crate::eval::Evaluator;
use crate::uci::SearchLimits;
use std::fmt::Debug;
use std::marker::PhantomData;

use rayon::prelude::*;

const ENG_INF: i32 = 99999999;

pub trait ChessEngine<C: BoardConfig>
where
    C: BoardConfig + Clone + Debug,
    [(); C::AREA]: Sized,
{
    fn get_next_move(&mut self, cb: &Chessboard<C>, depth: u32) -> Option<C::MoveType>;
    fn search(&mut self, cb: &Chessboard<C>, limits: SearchLimits) -> Option<C::MoveType>;
    fn get_nodes_searched(&self) -> u64;
    fn get_score(&self) -> i32;
}

#[derive(Clone, Debug)]
pub struct Engine<C, E>
where
    C: BoardConfig,
    E: Evaluator<C> + Send,
    [(); C::AREA]: Sized,
{
    pub evaluator: E,
    pub nodes_searched: u64,
    pub score: i32,
    pub _marker: PhantomData<C>,
}

impl<C, E> ChessEngine<C> for Engine<C, E>
where
    C: BoardConfig + PartialEq + Eq,

    E: Evaluator<C> + Clone + Send + Sync,

    C::MoveType: Send + Sync,

    [(); C::AREA]: Sized,
{
    fn search(&mut self, cb: &Chessboard<C>, limits: SearchLimits) -> Option<C::MoveType>
    where
        E: Clone + Send + Sync,
        C: BoardConfig + Clone + Debug + Send + Sync,
        C::MoveType: Send + Sync,
    {
        todo!();
    }
    
    fn get_next_move(&mut self, cb: &Chessboard<C>, depth: u32) -> Option<C::MoveType>
    where
        E: Clone + Send + Sync,
        C: BoardConfig + Clone + Debug + Send + Sync,
        C::MoveType: Send + Sync,
    {
        let mut root = cb.clone();

        let moves = root.generate_moves().ok()?;
        let player = root.current_player();
        let root = cb.clone();
        let mut evaluator = self.evaluator.clone();
        evaluator.init(&root);
        let jobs: Vec<_> = moves
            .into_iter()
            .map(|mov| {
                let mut child = cb.clone();
                child.apply_move(mov.clone()).unwrap();

                let eval = evaluator.clone();

                (mov, child, eval)
            })
            .collect();

        let mut weighted: Vec<_> = jobs
            .into_par_iter()
            .map(|(mov, mut child, mut eval)| {
                let mut nodes = 0u64;
                let score = match player {
                    CurrentPlayer::White => Self::alpha_beta_min(
                        &mut eval,
                        &mut child,
                        -ENG_INF,
                        ENG_INF,
                        depth as i32 - 1,
                        &mut nodes,
                    ),

                    CurrentPlayer::Black => Self::alpha_beta_max(
                        &mut eval,
                        &mut child,
                        -ENG_INF,
                        ENG_INF,
                        depth as i32 - 1,
                        &mut nodes,
                    ),
                };

                (score, mov, nodes)
            })
            .collect();

        self.nodes_searched = weighted.iter().map(|n| n.2).sum();

        let max = match player {
            CurrentPlayer::White => weighted.into_iter().max_by_key(|(score, _, _)| *score),

            CurrentPlayer::Black => weighted.into_iter().min_by_key(|(score, _, _)| *score),
        };

        self.score = if let Some(m) = max { m.0 } else { 0 };

        if let Some(m) = max {
            Some(m.1)
        } else {
            None
        }
    }

    fn get_score(&self) -> i32 {
        self.score
    }

    fn get_nodes_searched(&self) -> u64 {
        self.nodes_searched
    }
}

impl<C, E> Engine<C, E>
where
    C: BoardConfig + Clone + Debug + PartialEq + Eq,
    E: Evaluator<C> + Send,
    [(); C::AREA]: Sized,
{
    pub fn new(evaluator: E) -> Self {
        Self {
            evaluator,
            nodes_searched: 0,
            score: 0,
            _marker: PhantomData,
        }
    }

    fn alpha_beta_max(
        eval: &mut E,
        cb: &mut Chessboard<C>,
        alpha: i32,
        beta: i32,
        depth_left: i32,
        nodes_searched: &mut u64,
    ) -> i32 {
        let mut alpha = alpha;
        if (depth_left == 0) {
            return eval.evaluate(cb);
        }
        let mut best_val = -ENG_INF;

        let mut moves = match cb.generate_moves() {
            Ok(m) => m,
            Err(err) => {
                return match err {
                    GameStatus::Won(color) => match color {
                        CurrentPlayer::White => ENG_INF + depth_left,
                        CurrentPlayer::Black => -ENG_INF - depth_left,
                    },
                    _ => 0,
                }
            }
        };

        for mov in moves {
            *nodes_searched = *nodes_searched + 1;
            cb.apply_move(mov);
            eval.on_make_move(cb, &mov);
            let score = Self::alpha_beta_min(eval, cb, alpha, beta, depth_left - 1, nodes_searched);
            cb.undo_move();
            eval.on_undo_move(cb, &mov);
            if score > best_val {
                best_val = score;
                if score > alpha {
                    alpha = score;
                }
            }
            if score >= beta {
                return score;
            }
        }

        best_val
    }

    fn alpha_beta_min(
        eval: &mut E,
        cb: &mut Chessboard<C>,
        alpha: i32,
        beta: i32,
        depth_left: i32,
        nodes_searched: &mut u64,
    ) -> i32 {
        let mut beta = beta;
        if (depth_left == 0) {
            return eval.evaluate(cb);
        }
        let mut best_val = ENG_INF;

        let mut moves = match cb.generate_moves() {
            Ok(m) => m,
            Err(err) => {
                return match err {
                    GameStatus::Won(color) => match color {
                        CurrentPlayer::White => ENG_INF + depth_left,
                        CurrentPlayer::Black => -ENG_INF - depth_left,
                    },
                    _ => 0,
                }
            }
        };

        for mov in moves {
            *nodes_searched = *nodes_searched + 1;
            cb.apply_move(mov);
            eval.on_make_move(cb, &mov);
            let score = Self::alpha_beta_max(eval, cb, alpha, beta, depth_left - 1, nodes_searched);
            cb.undo_move();
            eval.on_undo_move(cb, &mov);
            if score < best_val {
                best_val = score;
                if score < beta {
                    beta = score;
                }
            }
            if score <= alpha {
                return score;
            }
        }

        best_val
    }
}
