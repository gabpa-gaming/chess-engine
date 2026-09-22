use crate::bitboard::BitboardIndex;
use crate::board_config::{BoardConfig, RegularVariant};
use crate::chess_board::{Chessboard, CurrentPlayer, MoveLegality};
use crate::chess_engine::ChessEngine;
use crate::chess_engine::Engine;
use crate::chess_move::{MoveFlag, MoveTrait};
use crate::eval::{DeadSimpleEvaluator, LayeredMLEvaluator, MLEvaluator};
use crate::ml::MLModel;
use crate::piece::{Piece, PieceType};
use crate::uci::GoVariant;
use anyhow::anyhow;
use std::fmt::Debug;
use std::io;
use std::io::BufRead;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Instant;
use std::sync::atomic::Ordering;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct GameSettings {
    pub evaluator_name: String,
    pub search_max_depth: u32,
}

#[derive(Clone, Debug)]
pub struct GameController<C: BoardConfig + Clone + Debug>
where
    [(); C::AREA]: Sized,
{
    chessboard: Chessboard<C>,
}

impl Default for GameSettings {
    fn default() -> Self {
        GameSettings {
            evaluator_name: "simple".to_string(),
            search_max_depth: 5,
        }
    }
}

impl<C: BoardConfig + Clone + Debug + PartialEq + Eq> GameController<C>
where
    [(); C::AREA]: Sized,
{
    pub fn new() -> Self {
        GameController::<C> {
            chessboard: Chessboard::regular_board(),
        }
    }

    pub fn from_fen(fen: &str) -> Result<Self, String> {
        Ok(GameController::<C> {
            chessboard: Chessboard::from_fen(fen)?,
        })
    }

    pub fn with_chessboard(mut self, board: Chessboard<C>) -> Self {
        self.chessboard = board;
        self
    }

    pub const fn height() -> usize {
        C::HEIGHT
    }

    pub fn move_piece(&mut self, move_: C::MoveType) -> MoveLegality {
        self.chessboard.apply_move_checked(move_)
    }

    pub fn undo(&mut self) {
        self.chessboard.undo_move();
    }

    pub fn from_uci(&mut self, input: &str) -> Option<C::MoveType> {
        let uci_move = crate::uci::UciMove::parse_for::<C>(input).ok()?;
        self.move_from_uci(uci_move)
    }

    pub fn move_to_uci(move_: &C::MoveType) -> String {
        let from = Self::square_to_uci(move_.from());
        let to = Self::square_to_uci(move_.to());
        let promotion = match move_.promotion() {
            PieceType::Queen(_) => "q",
            PieceType::Rook(_) => "r",
            PieceType::Bishop(_) => "b",
            PieceType::Knight(_) => "n",
            _ => "",
        };
        format!("{from}{to}{promotion}")
    }

    fn move_from_uci(&mut self, uci_move: crate::uci::UciMove) -> Option<C::MoveType> {
        self.chessboard
            .generate_moves()
            .ok()?
            .into_iter()
            .find(|move_| {
                move_.from().as_usize() == uci_move.from.0 as usize
                    && move_.to().as_usize() == uci_move.to.0 as usize
                    && match move_.flag() {
                        MoveFlag::Promotion => matches!(
                            (uci_move.promotion, move_.promotion()),
                            (Some(crate::uci::PromotionPiece::Queen), PieceType::Queen(_))
                                | (Some(crate::uci::PromotionPiece::Rook), PieceType::Rook(_))
                                | (
                                    Some(crate::uci::PromotionPiece::Bishop),
                                    PieceType::Bishop(_)
                                )
                                | (
                                    Some(crate::uci::PromotionPiece::Knight),
                                    PieceType::Knight(_)
                                )
                        ),
                        _ => uci_move.promotion.is_none(),
                    }
            })
    }

    fn square_to_uci(square: C::Square) -> String {
        let file = square.as_usize() % C::WIDTH;
        let rank = C::HEIGHT - (square.as_usize() / C::WIDTH);
        format!("{}{}", (b'a' + file as u8) as char, rank)
    }

    pub fn parse_chess_notation(&mut self, input: &str) -> Result<C::MoveType, String> {
        let input = input.trim();

        if input == "O-O" || input == "O-O-O" {
            let offset = if self.chessboard.current_player() == CurrentPlayer::White {
                (C::HEIGHT - 1) * C::WIDTH
            } else {
                0
            };
            let (from_sq, to_sq) = if input == "O-O" {
                (4 + offset, 6 + offset) // e to g
            } else {
                (4 + offset, 2 + offset) // e to c
            };

            let castle_move = C::MoveType::new(
                C::Square::from_usize(from_sq),
                C::Square::from_usize(to_sq),
                MoveFlag::Castling,
                PieceType::None,
            );

            let generated = self.chessboard.generate_moves().unwrap_or(Vec::new());
            if generated
                .iter()
                .any(|m| m.from().as_usize() == from_sq && m.to().as_usize() == to_sq)
            {
                return Ok(castle_move);
            } else {
                return Err("Castling move is illegal in this position".to_string());
            }
        }
        let mut promotion_piece = PieceType::None;
        let mut working_input = input.to_string();

        if let Some(eq_idx) = working_input.find('=') {
            if eq_idx + 1 < working_input.len() {
                let promo_char = working_input.chars().nth(eq_idx + 1).unwrap();
                promotion_piece = match promo_char.to_ascii_uppercase() {
                    'Q' | 'q' => PieceType::Queen(unsafe { std::mem::zeroed() }),
                    'R' | 'r' => PieceType::Rook(unsafe { std::mem::zeroed() }),
                    'B' | 'b' => PieceType::Bishop(unsafe { std::mem::zeroed() }),
                    'N' | 'n' => PieceType::Knight(unsafe { std::mem::zeroed() }),
                    _ => return Err(format!("Invalid promotion piece: {}", promo_char)),
                };
                working_input.truncate(eq_idx);
            }
        } else if input.chars().next().map_or(false, |c| c.is_lowercase()) {
            let last_char = working_input.chars().last().unwrap();
            if ['Q', 'R', 'B', 'N'].contains(&last_char.to_ascii_uppercase()) {
                promotion_piece = match last_char.to_ascii_uppercase() {
                    'Q' | 'q' => PieceType::Queen(unsafe { std::mem::zeroed() }),
                    'R' | 'r' => PieceType::Rook(unsafe { std::mem::zeroed() }),
                    'B' | 'b' => PieceType::Bishop(unsafe { std::mem::zeroed() }),
                    'N' | 'n' => PieceType::Knight(unsafe { std::mem::zeroed() }),
                    _ => PieceType::None,
                };
                working_input.pop();
            }
        }

        let cleaned: String = working_input
            .chars()
            .filter(|c| *c != '+' && *c != '#' && *c != 'x')
            .collect();
        let chars: Vec<char> = cleaned.chars().collect();

        if chars.len() < 2 {
            return Err(format!("Invalid chess notation: too short ('{}')", input));
        }

        let rank_start = cleaned
            .char_indices()
            .rev()
            .take_while(|(_, character)| character.is_ascii_digit())
            .last()
            .map(|(index, _)| index)
            .ok_or_else(|| "Invalid chess notation: invalid destination rank".to_string())?;
        let dest_file = cleaned[..rank_start]
            .chars()
            .last()
            .ok_or_else(|| "Invalid chess notation: invalid destination file".to_string())?;
        let dest_rank_num = cleaned[rank_start..]
            .parse::<usize>()
            .map_err(|_| "Invalid chess notation: invalid destination rank".to_string())?;

        if !(('a'..='z').contains(&dest_file))
            || (dest_file as usize - 'a' as usize) >= C::WIDTH
            || !(1..=C::HEIGHT).contains(&dest_rank_num)
        {
            return Err("Invalid chess notation: invalid destination square".to_string());
        }

        let dest_file_num = (dest_file as usize) - ('a' as usize);
        let dest_idx = ((C::HEIGHT - dest_rank_num) * C::WIDTH) + dest_file_num;

        let piece_char = if chars[0].is_lowercase() {
            'P'
        } else {
            chars[0].to_ascii_uppercase()
        };

        let pieces = self.chessboard.get_pieces();
        let current_player = self.chessboard.current_player();
        let mut candidates = Vec::new();

        for (idx, piece) in pieces.iter().enumerate() {
            let is_correct_piece = match (piece, current_player) {
                (Piece::White(ptype), CurrentPlayer::White) => {
                    ptype.to_notation().to_uppercase().next().unwrap() == piece_char
                }
                (Piece::Black(ptype), CurrentPlayer::Black) => {
                    ptype.to_notation().to_uppercase().next().unwrap() == piece_char
                }
                _ => false,
            };

            if is_correct_piece {
                candidates.push(idx);
            }
        }

        if candidates.is_empty() {
            return Err(format!("No valid source piece found for move '{}'", input));
        }

        let generated_moves = self.chessboard.generate_moves().unwrap_or(Vec::new());
        let mut valid_moves = Vec::new();
        let mut is_ep = false;

        for candidate in &candidates {
            if let Some(valid_move) = generated_moves
                .iter()
                .find(|m| m.from().as_usize() == *candidate && m.to().as_usize() == dest_idx)
            {
                if valid_move.flag() == MoveFlag::EnPassant {
                    is_ep = true;
                }
                valid_moves.push(*candidate);
            }
        }

        if valid_moves.is_empty() {
            return Err(format!(
                "No piece of that type can move to the destination square in '{}'",
                input
            ));
        }

        let final_flag = if promotion_piece != PieceType::None {
            MoveFlag::Promotion
        } else if is_ep {
            MoveFlag::EnPassant
        } else if input.contains('x') {
            MoveFlag::Capture
        } else {
            MoveFlag::Quiet
        };

        let destination_start = rank_start - 1;
        let has_disambiguator = if piece_char == 'P' {
            destination_start > 0
        } else {
            destination_start > 1
        };

        if has_disambiguator {
            let disambiguator = if piece_char == 'P' {
                chars[0]
            } else {
                chars[1]
            };

            valid_moves.retain(|&candidate| {
                if disambiguator >= 'a' && disambiguator <= 'h' {
                    let candidate_file = candidate % C::WIDTH;
                    let target_file = (disambiguator as usize) - ('a' as usize);
                    candidate_file == target_file
                } else if disambiguator.is_ascii_digit() {
                    let candidate_rank = candidate / C::WIDTH;
                    let target_rank_num = disambiguator.to_digit(10).unwrap() as usize;
                    let target_rank = C::HEIGHT - target_rank_num;
                    candidate_rank == target_rank
                } else {
                    false
                }
            });

            if valid_moves.is_empty() {
                return Err(format!(
                    "No valid move found matching the disambiguator '{}'",
                    disambiguator
                ));
            }
        }

        if valid_moves.len() > 1 {
            return Err(format!(
                "Ambiguous move '{}'. Multiple pieces can move there.",
                input
            ));
        }

        Ok(C::MoveType::new(
            C::Square::from_usize(valid_moves[0]),
            C::Square::from_usize(dest_idx),
            final_flag,
            promotion_piece,
        ))
    }

    pub fn to_chess_notation(&self, pos: u16) -> String {
        let rank = pos / C::WIDTH as u16;
        let file = pos % C::WIDTH as u16;

        let file_char = (b'a' + file as u8) as char;
        format!("{}{}", file_char, C::HEIGHT - rank as usize)
    }

    pub fn chessboard(&self) -> &Chessboard<C> {
        &self.chessboard
    }

    pub fn chessboard_mut(&mut self) -> &mut Chessboard<C> {
        &mut self.chessboard
    }

    pub fn current_player(&self) -> &str {
        match self.chessboard.current_player() {
            crate::chess_board::CurrentPlayer::White => "White",
            crate::chess_board::CurrentPlayer::Black => "Black",
        }
    }

    pub fn perf_mode(&mut self, depth: usize, speed: bool) -> u64 {
        let moves = self.chessboard_mut().generate_moves().unwrap_or(Vec::new());
        if depth <= 1 {
            let mut count = 0;
            for m in &moves {
                if !speed {
                    {
                        let board = self.chessboard_mut();
                        board.apply_move(*m).unwrap();
                        Chessboard::display_board(&board.get_pieces(), C::WIDTH);
                        let mov_c = self
                            .chessboard_mut()
                            .generate_moves()
                            .unwrap_or(Vec::new())
                            .len();
                        println!("Generated {} responses", mov_c);
                    }
                    self.chessboard_mut().undo_move();
                    count += 1;
                } else {
                    count += 1;
                }
            }
            return count;
        }

        let mut count = 0;
        for m in &moves {
            {
                let board = self.chessboard_mut();
                board.apply_move(*m).unwrap();
            }
            if !speed {
                let board = self.chessboard_mut();
                Chessboard::display_board(&board.get_pieces(), C::WIDTH);
            }
            count += self.perf_mode(depth - 1, speed);
            self.chessboard_mut().undo_move();
        }
        count
    }
}

impl GameController<RegularVariant> {
    fn get_engine(name: &str) -> anyhow::Result<Box<dyn ChessEngine<RegularVariant> + Send>> {
        match name {
            "simple" => Ok(Box::new(Engine::new(DeadSimpleEvaluator {}))),
            e if e.starts_with("ml") => {
                if let Some(inner) = e.strip_prefix("ml(").and_then(|x| x.strip_suffix(')')) {
                    Ok(Box::new(Engine::new(MLEvaluator::from_model(
                        MLModel::load_model(inner)?,
                    ))))
                } else {
                    Err(anyhow!("Incorrect format. Try: ml(path)"))
                }
            }
            e if e.starts_with("layeredml") => {
                if let Some(inner) = e
                    .strip_prefix("layeredml(")
                    .and_then(|x| x.strip_suffix(')'))
                {
                    Ok(Box::new(Engine::new(
                        LayeredMLEvaluator::<RegularVariant>::load_model(inner)?,
                    )))
                } else {
                    Err(anyhow!("Incorrect format. Try: layeredml(path)"))
                }
            }
            _ => Err(anyhow!(
                "Engine name {} not found. Try: simple, ml(path/to/weights/file)",
                name
            )),
        }
    }

    pub fn uci_loop() {
        let stdin = io::stdin();
        let mut game = Self::new();
        let mut settings = GameSettings::default();
        let mut engine: Option<Box<dyn ChessEngine<RegularVariant> + Send>> =
            Some(Box::new(Engine::new(DeadSimpleEvaluator {})));
        let mut search_handler: Option<
            thread::JoinHandle<Box<dyn ChessEngine<RegularVariant> + Send>>,
        > = None;
        let stop = Arc::new(AtomicBool::new(false));
        for line in stdin.lock().lines() {
            let Ok(line) = line else {
                break;
            };
            let command = match crate::uci::parse(&line) {
                Ok(Some(command)) => command,
                Ok(None) => continue,
                Err(error) => {
                    eprintln!("UCI parse error: {error}");
                    continue;
                }
            };
            if search_handler
                .as_ref()
                .is_some_and(|handler| handler.is_finished())
            {
                engine = search_handler.take().unwrap().join().ok();
            }
            match command {
                crate::uci::Command::Uci => {
                    println!("id name omni stork v{}", env!("CARGO_PKG_VERSION"));
                    println!("id author gabpa");
                    Self::emit_options();
                    println!("uciok");
                }
                crate::uci::Command::IsReady => println!("readyok"),
                crate::uci::Command::UciNewGame => game = Self::new(),
                crate::uci::Command::Position(position) => {
                    let next_game = match position.board {
                        crate::uci::PositionBoard::StartPos => Ok(Self::new()),
                        crate::uci::PositionBoard::Fen(fen) => Self::from_fen(&fen),
                    };
                    let Ok(mut next_game) = next_game else {
                        eprintln!("UCI position error: invalid FEN");
                        continue;
                    };

                    let moves_are_legal = position.moves.into_iter().all(|uci_move| {
                        next_game
                            .move_from_uci(uci_move)
                            .is_some_and(|move_| next_game.move_piece(move_) == MoveLegality::Legal)
                    });
                    if moves_are_legal {
                        game = next_game;
                    } else {
                        eprintln!("UCI position error: illegal move");
                    }
                }
                crate::uci::Command::Go(mut limits) => {
                    if let GoVariant::Weights(mapping) = limits.go_variant {
                        let inputs = mapping.board_to_input(&game.chessboard());
                        println!(
                            "{}",
                            serde_json::to_string(inputs.as_slice()).expect("Serialization error.")
                        );
                        continue;
                    }
                    if limits.go_variant == GoVariant::Perft {
                        let Some(depth) = limits.depth else {
                            eprintln!("UCI perft error: provide a depth (go perft depth N)");
                            continue;
                        };
                        let started = Instant::now();
                        let nodes = game.perf_mode(depth as usize, true);
                        let elapsed = started.elapsed();
                        let time_ms = elapsed.as_millis();
                        let nps = if elapsed.is_zero() {
                            0
                        } else {
                            (nodes as f64 / elapsed.as_secs_f64()) as u64
                        };
                        println!("info depth {depth} nodes {nodes} time {time_ms} nps {nps}");
                        println!("bestmove 0000");
                        continue;
                    }
                    if let Some(mut eng) = engine.take() {
                        if limits.go_variant == GoVariant::Eval {
                            let score = eng.evaluate(game.chessboard());
                            println!("info score cp {}", score.round() as i32);
                            println!("bestmove 0000");
                            continue;
                        }
                        let depth = limits.depth.unwrap_or(settings.search_max_depth);
                        limits.depth = Some(depth);
                        let chessboard = game.chessboard().clone();

                        let stop = stop.clone();
                        search_handler = Some(thread::spawn(move || {
                            eng.search(&chessboard, limits, stop);
                            eng
                        }));
                    }
                }
                crate::uci::Command::Quit => break,
                crate::uci::Command::SetOption { name, value } => match name.as_str() {
                    "Depth" => {
                        if let Some(val) = value {
                            match val.parse::<u32>() {
                                Ok(depth @ 1..=40) => settings.search_max_depth = depth,
                                _ => eprintln!("UCI option error: Depth must be between 1 and 40"),
                            }
                        }
                    }
                    "Evaluator" => {
                        if let Some(mut handler) = search_handler.take() {
                            engine = handler.join().ok();
                        }
                        if let Some(value) = value {
                            match Self::get_engine(&value) {
                                Ok(next_engine) => {
                                    engine = Some(next_engine);
                                    settings.evaluator_name = value;
                                }
                                Err(error) => {
                                    eprintln!("UCI option error: {error}");
                                }
                            }
                        }
                    }
                    _ => {}
                },
                crate::uci::Command::Stop => {
                    if let Some(mut handler) = search_handler.take() {
                        stop.store(true, Ordering::Relaxed);
                        engine = handler.join().ok();
                        stop.store(false, Ordering::Relaxed)
                    }
                }
                crate::uci::Command::Debug(_)
                | crate::uci::Command::PonderHit
                | crate::uci::Command::Unknown(_) => {}
            }
        }
    }

    fn emit_options() {
        println!("option name Depth type spin default 5 min 1 max 40");
        println!("option name Evaluator type string default simple")
    }
}
