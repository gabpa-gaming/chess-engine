use std::thread;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use dashmap::{DashMap};
use crate::board_config::BoardConfig;
use crate::chess_board::{Chessboard, CurrentPlayer, GameStatus};
use crate::eval::Evaluator;
use crate::game_controller::GameController;
use crate::uci::SearchLimits;
use std::fmt::Debug;
use std::marker::PhantomData;

use rayon::prelude::*;

const ENG_INF: f32 = 99999999.0;
const MATE_SCORE: f32 = 1000000.0;

#[derive(Debug, Clone, Copy)]
pub struct TTEntry<C: BoardConfig> 
where
    C: BoardConfig + Clone + Debug,
    [(); C::AREA]: Sized,
{
    depth: i32,
    best_move: C::MoveType,
    score: f32
}

pub trait ChessEngine<C: BoardConfig>
where
    C: BoardConfig + Clone + Debug,
    [(); C::AREA]: Sized,
{
    fn get_next_move(&self) -> Option<C::MoveType>;
    fn search(&mut self, cb: &Chessboard<C>, limits: SearchLimits, stop: Arc<AtomicBool>);
    fn get_nodes_searched(&self) -> u64;
    fn get_score(&self) -> f32;
    fn evaluate(&mut self, cb: &Chessboard<C>) -> f32;
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
    pub score: f32,
    pub best_moves: Arc<DashMap<u64, TTEntry<C>>>,
    pub best_move: Option<C::MoveType>,
    pub _marker: PhantomData<C>,
}

impl<C, E> ChessEngine<C> for Engine<C, E>
where
    C: BoardConfig + PartialEq + Eq,

    E: Evaluator<C> + Clone + Send + Sync,

    C::MoveType: Send + Sync,
    
    [(); C::AREA]: Sized,
{
    fn search(&mut self, cb: &Chessboard<C>, limits: SearchLimits, stop: Arc<AtomicBool>)
    where
        E: Clone + Send + Sync,
        C: BoardConfig + Clone + Debug + Send + Sync,
        C::MoveType: Send + Sync,
    {
        let max_depth = limits.depth.unwrap_or(99);

        let cb = cb.clone();

        let timer_stop = Arc::clone(&stop);
        let time_budget = limits.time_budget(cb.current_player());
        let (done_tx, done_rx) = mpsc::channel::<()>();
        
        let timer = time_budget.map(|time| {
            let timer_stop = Arc::clone(&stop);
        
            thread::spawn(move || {
                if matches!(
                    done_rx.recv_timeout(time),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ) {
                    timer_stop.store(true, Ordering::Relaxed);
                }
            })
        });        
        self.best_move = None;
        self.evaluator.init(&cb);
        for d in 1..max_depth {
            let result = Self::root_search(
                    &cb,
                    &self.evaluator,
                    d,
                    &self.best_moves,
                    &stop,
                );
            
                match result {
                    RootSearchResult::BestMove(best_move, score, nodes) => {
                        self.best_move = Some(best_move);
                        self.score = score;
                        self.nodes_searched = nodes as u64;
                        let pov_score = if cb.current_player().eq(&CurrentPlayer::Black) {
                            -score
                        } else {
                            score
                        };
                        println!(
                            "info depth {} score cp {} nodes {} pv {}",
                            d,
                            pov_score.round() as i32,
                            nodes,
                            GameController::move_to_uci(&best_move)
                        );
                    }
            
                    RootSearchResult::None => break,
                    RootSearchResult::ForcedMate(best_move, n, nodes) => {
                        self.best_move = Some(best_move);
                        self.score = ENG_INF;
                        self.nodes_searched = nodes as u64;

                        println!(
                            "info depth {} score mate {} nodes {} pv {}",
                            d,
                            n,
                            nodes,
                            GameController::move_to_uci(&best_move)
                        );
                    }
                }
        }
        if let Some(mov) = self.best_move{
            let uci_mov = GameController::move_to_uci(&mov);
            println!("bestmove {}", uci_mov);
        } else {
            println!("bestmove 0000");
        }
        stop.store(false, Ordering::Relaxed);
        let _ = done_tx.send(());
        
        if let Some(timer) = timer {
            let _ = timer.join();
        }
    }

    
    
    fn evaluate(&mut self, cb: &Chessboard<C>) -> f32 {
        self.evaluator.init(cb);
        self.evaluator.evaluate(cb)
    }
    
    fn get_next_move(&self) -> Option<C::MoveType>
    where
        E: Clone + Send + Sync,
        C: BoardConfig + Clone + Debug + Send + Sync,
        C::MoveType: Send + Sync,
    {
        self.best_move
    }

    fn get_score(&self) -> f32 {
        self.score
    }

    fn get_nodes_searched(&self) -> u64 {
        self.nodes_searched
    }
}

pub enum RootSearchResult<C: BoardConfig> 
where
    C: BoardConfig + PartialEq + Eq + Send + Sync,

    C::MoveType: Send + Sync,
    
    [(); C::AREA]: Sized,
{
    None,
    ForcedMate(C::MoveType, i32, u32),
    BestMove(C::MoveType, f32, u32)
}



impl<C, E> Engine<C, E>
where
    C: BoardConfig + Clone + Debug + PartialEq + Eq,
    E: Evaluator<C> + Send + Sync,
    [(); C::AREA]: Sized,
{
    pub fn new(evaluator: E) -> Self {
        Self {
            evaluator,
            nodes_searched: 0,
            score: 0.0,
            best_moves: Arc::new(DashMap::new()),
            best_move: None,
            _marker: PhantomData,
        }
    }
    
    fn root_search(
        cb: &Chessboard<C>,
        eval: &E,
        depth: u32,
        tt_table: &Arc<DashMap<u64, TTEntry<C>>>,
        stop: &Arc<AtomicBool>
    ) -> RootSearchResult<C>
    where
        E: Clone + Send + Sync,
        C: BoardConfig + Clone + Debug + Send + Sync + PartialEq + Eq,
        C::MoveType: Send + Sync,
    {
        if depth == 0 {
            return RootSearchResult::None;
        }
    
        let mut root = cb.clone();
    
        let mut moves = match root.generate_moves() {
            Ok(moves) => moves,
            Err(_) => return RootSearchResult::None,
        };
    
        let player = root.current_player();
    
        let key = root.zobrist_hash();
    
        if let Some(tt_move) = tt_table.get(&key).map(|entry| entry.best_move) {
            if let Some(pos) = moves.iter().position(|m| *m == tt_move) {
                moves.swap(0, pos);
            }
        }
    
        let mut root_eval = eval.clone();
        root_eval.init(&root);
    
        let weighted: Option<Vec<_>> = moves
            .into_par_iter()
            .map_init(
                || {
                    let board = root.clone();
                    let evaluator = root_eval.clone();
    
                    (board, evaluator)
                },
                |(board, evaluator), mov| {
                    let mut nodes = 0u64;
    
                    board.apply_move(mov).unwrap();
                    evaluator.on_make_move(board, &mov);
    
                    let score = match player {
                        CurrentPlayer::White => Self::alpha_beta_min(
                            evaluator,
                            board,
                            -ENG_INF,
                            ENG_INF,
                            depth as i32 - 1,
                            &mut nodes,
                            tt_table,
                            stop
                        ),
    
                        CurrentPlayer::Black => Self::alpha_beta_max(
                            evaluator,
                            board,
                            -ENG_INF,
                            ENG_INF,
                            depth as i32 - 1,
                            &mut nodes,
                            tt_table,
                            stop
                        ),
                    };

                    
                    board.undo_move();
                    evaluator.on_undo_move(board, &mov);
    
                    Some((score?, mov, nodes))
                },
            )
            .collect();
        
        let weighted = if let Some(w) = weighted {
            w 
        } else {
            return RootSearchResult::None;
        };
        
        let nodes: u64 = weighted.iter().map(|(_, _, nodes)| *nodes).sum();
    
        let best = match player {
            CurrentPlayer::White => weighted
                .into_iter()
                .max_by(|a, b| a.0.total_cmp(&b.0)),
    
            CurrentPlayer::Black => weighted
                .into_iter()
                .min_by(|a, b| a.0.total_cmp(&b.0)),
        };


         
        match best {
            Some((score, best_move, _)) => {        
                if let Some(mate) = Self::score_to_mate(score, depth, player) {
                    RootSearchResult::ForcedMate(best_move, mate, nodes as u32)
                } else {
                    RootSearchResult::BestMove(best_move, score, nodes as u32)
                }
            }
            None => RootSearchResult::None,
        }
    }
    
    fn score_to_mate(
        score: f32,
        root_depth: u32,
        player: CurrentPlayer,
    ) -> Option<i32> {
        if score.abs() < MATE_SCORE {
            return None;
        }
    
        let remaining = (score.abs() - MATE_SCORE).round() as i32;
    
        let plies = root_depth as i32 - remaining;
    
        let mate_in = (plies + 1) / 2;
    
        let pov_score = match player {
            CurrentPlayer::White => score,
            CurrentPlayer::Black => -score,
        };
    
        Some(if pov_score > 0.0 {
            mate_in
        } else {
            -mate_in
        })
    }
    
    fn alpha_beta_max(
        eval: &mut E,
        cb: &mut Chessboard<C>,
        alpha: f32,
        beta: f32,
        depth_left: i32,
        nodes_searched: &mut u64,
        tt_table: &Arc<DashMap<u64, TTEntry<C>>>,
        stop: &Arc<AtomicBool>
    ) -> Option<f32> {
        if (*nodes_searched & 1023) == 0 && stop.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }
        let mut alpha = alpha;
        if depth_left == 0 {
            return Some(eval.evaluate(cb));
        }

        let key = cb.zobrist_hash();
        
        let tt_entry = tt_table
                    .get(&key);
        
        if let Some(ref entry) = tt_entry {
            if entry.depth >= depth_left {
                return Some(entry.score);
            }
        }

        let tt_move = tt_entry.map(|entry| entry.best_move);
                
        let mut best_val = -ENG_INF;
        let mut best_mov = None;
        
        let mut moves = match cb.generate_moves() {
            Ok(m) => m,
            Err(err) => {
                return match err {
                    GameStatus::Won(color) => match color {
                        CurrentPlayer::White => Some(MATE_SCORE + depth_left as f32),
                        CurrentPlayer::Black => Some(-MATE_SCORE - depth_left as f32),
                    },
                    _ => Some(0.0),
                }
            }
        };
        if let Some(tt_move) = tt_move {
            if let Some(pos) = moves.iter().position(|m| *m == tt_move) {
                moves.swap(0, pos);
            }
        }
        for mov in moves {
            *nodes_searched = *nodes_searched + 1;
            _ = cb.apply_move(mov);
            eval.on_make_move(cb, &mov);
            let score = Self::alpha_beta_min(eval, cb, alpha, beta, depth_left - 1, nodes_searched, &tt_table, stop)?;
            cb.undo_move();
            eval.on_undo_move(cb, &mov);
            if score > best_val {
                best_val = score;
                best_mov = Some(mov);
                if score > alpha {
                    alpha = score;
                }
            }
            if score >= beta {
                tt_table.insert(key, TTEntry { depth: depth_left, best_move: mov, score});
                return Some(score);
            }
        }
        
        if let Some(best_move) = best_mov {
            tt_table.insert(key, TTEntry {
                depth: depth_left,
                best_move,
                score: best_val
            });
        }
        
        Some(best_val)
    }

    fn alpha_beta_min(
        eval: &mut E,
        cb: &mut Chessboard<C>,
        alpha: f32,
        beta: f32,
        depth_left: i32,
        nodes_searched: &mut u64,
        tt_table: &Arc<DashMap<u64, TTEntry<C>>>,
        stop: &Arc<AtomicBool>
    ) -> Option<f32> {
        if (*nodes_searched & 1023) == 0 && stop.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }
        let mut beta = beta;
    
        if depth_left == 0 {
            return Some(eval.evaluate(cb));
        }
    
        let key = cb.zobrist_hash();
    
        let tt_entry = tt_table
            .get(&key);

        if let Some(ref entry) = tt_entry {
            if entry.depth >= depth_left {
                return Some(entry.score);
            }
        }
        
        let tt_move = tt_entry.map(|entry| entry.best_move);
        
        let mut best_val = ENG_INF;
        let mut best_mov = None;
    
        let mut moves = match cb.generate_moves() {
            Ok(m) => m,
            Err(err) => {
                return match err {
                    GameStatus::Won(color) => match color {
                        CurrentPlayer::White => Some(MATE_SCORE + depth_left as f32),
                        CurrentPlayer::Black => Some(-MATE_SCORE - depth_left as f32),
                    },
                    _ => Some(0.0),
                };
            }
        };
    
        if let Some(tt_move) = tt_move {
            if let Some(pos) = moves.iter().position(|m| *m == tt_move) {
                moves.swap(0, pos);
            }
        }
    
        for mov in moves {
            *nodes_searched += 1;
    
            _ = cb.apply_move(mov);
            eval.on_make_move(cb, &mov);
    
            let score = Self::alpha_beta_max(
                eval,
                cb,
                alpha,
                beta,
                depth_left - 1,
                nodes_searched,
                tt_table,
                &stop
            )?;
    
            cb.undo_move();
            eval.on_undo_move(cb, &mov);
    
            if score < best_val {
                best_val = score;
                best_mov = Some(mov);
    
                if score < beta {
                    beta = score;
                }
            }
    
            if score <= alpha {
                tt_table.insert(
                    key,
                    TTEntry {
                        depth: depth_left,
                        best_move: mov,
                        score
                    },
                );
    
                return Some(score);
            }
        }
    
        if let Some(best_move) = best_mov {
            tt_table.insert(
                key,
                TTEntry {
                    depth: depth_left,
                    best_move,
                    score: best_val
                },
            );
        }
    
        Some(best_val)
    }
}

impl SearchLimits {
    pub fn time_budget(&self, player: CurrentPlayer) -> Option<Duration> {
        if self.infinite || self.ponder {
            return None;
        }

        if let Some(ms) = self.move_time {
            return Some(Duration::from_millis(ms));
        }

        let (remaining, increment) = match player {
            CurrentPlayer::White => (self.wtime, self.winc),
            CurrentPlayer::Black => (self.btime, self.binc),
        };

        let remaining = remaining?;
        let increment = increment.unwrap_or(0);
        let moves = u64::from(self.moves_to_go.unwrap_or(30).max(1));

        let reserve = (remaining / 10).min(50);
        let available = remaining.saturating_sub(reserve);

        let budget = (remaining / moves)
            .saturating_add(increment.saturating_mul(4) / 5)
            .min(available);

        Some(Duration::from_millis(budget))
    }
}
