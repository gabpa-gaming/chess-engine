use crate::board_config::BoardConfig;
use crate::chess_board::{Chessboard, CurrentPlayer};
use crate::chess_move::{MoveFlag, MoveTrait};
use crate::piece::Piece;
use serde::{Deserialize, Serialize};

const STATE_FEATURES: usize = 23;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinearModel {
    weights: Vec<f32>,
    bias: f32,
}

impl LinearModel {
    pub fn new(input_size: usize) -> Self {
        Self {
            weights: vec![0.0; input_size],
            bias: 0.0,
        }
    }

    pub fn bias(&self) -> f32 {
        self.bias
    }
    
    pub fn forward(&self, input: &[f32]) -> f32 {
        assert_eq!(input.len(), self.weights.len());

        self.weights
            .iter()
            .zip(input.iter())
            .map(|(w, x)| w * x)
            .sum::<f32>()
            + self.bias
    }

    pub fn input_size(&self) -> usize {
        self.weights.len()
    }

    pub fn get_weight(&self, index: usize) -> Option<&f32> {
        self.weights.get(index)
    }

    pub fn get_bias(&self) -> f32 {
        self.bias
    }

    pub fn load_model(path: &str) -> anyhow::Result<LinearModel, anyhow::Error> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json).map_err(|_| anyhow::anyhow!("Couldn't deserialize json model."))
    }

    pub fn board_to_input<C>(cb: &Chessboard<C>) -> [f32; 6 * C::AREA + 1 + STATE_FEATURES]
    where
        C: BoardConfig + PartialEq + Eq,
        [(); 6 * C::AREA + 1 + STATE_FEATURES]: Sized,
        [(); C::AREA]: Sized,
    {
        let mut input = [0.0; 6 * C::AREA + 1 + STATE_FEATURES];

        for (square, piece) in cb.get_pieces().iter().enumerate() {
            match piece {
                crate::piece::Piece::White(p) => {
                    let kind = (p.to_index() - 1) as usize;
                    let idx = kind * C::AREA + square;
                    input[idx] += 1.0;
                }

                crate::piece::Piece::Black(p) => {
                    let kind = (p.to_index() - 1) as usize;
                    let rank = square / C::WIDTH;
                    let file = square % C::WIDTH;
                    let mirrored_square = (C::HEIGHT - 1 - rank) * C::WIDTH + file;
                    let idx = kind * C::AREA + mirrored_square;
                    input[idx] -= 1.0;
                }

                _ => {}
            }
        }
        input[6 * C::AREA] = if cb.current_player() == CurrentPlayer::White {
            1.0
        } else {
            -1.0
        };
        let state_start = 6 * C::AREA + 1;
        input[state_start..].copy_from_slice(&Self::state_features(cb));

        input
    }

    fn state_features<C>(cb: &Chessboard<C>) -> [f32; STATE_FEATURES]
    where
        C: BoardConfig + PartialEq + Eq,
        [(); C::AREA]: Sized,
    {
        let mut input = [0.0; STATE_FEATURES];
        let rights = cb.castling_rights();
        input[0] = ((rights & ((1_u128 << 60) | (1_u128 << 63))) != 0) as u8 as f32;
        input[1] = ((rights & ((1_u128 << 60) | (1_u128 << 56))) != 0) as u8 as f32;
        input[2] = ((rights & ((1_u128 << 4) | (1_u128 << 7))) != 0) as u8 as f32;
        input[3] = ((rights & ((1_u128 << 4) | (1_u128 << 0))) != 0) as u8 as f32;

        if C::WIDTH == 8 && C::HEIGHT == 8 {
            if let Some(square) = cb.en_passant_square() {
                let rank = square as usize / C::WIDTH;
                let file = square as usize % C::WIDTH;
                let index = match rank {
                    2 => Some(file),
                    5 => Some(8 + file),
                    _ => None,
                };
                if let Some(index) = index {
                    input[4 + index] = 1.0;
                }
            }
        }

        input[20] = cb.repetition_count() as f32;
        input[21] = if C::WIDTH == 8 && C::HEIGHT == 8 {
            cb.is_king_checked(cb.current_player()) as u8 as f32
        } else {
            0.0
        };
        input[22] = cb.halfmove_clock() as f32;
        input
    }
}

#[derive(Clone, Debug)]
pub struct LinearStepper {
    model: LinearModel,
    inputs: Vec<f32>,
    current_score: f32,
    changed_squares: Vec<Vec<usize>>,
}

impl LinearStepper {
    pub fn new(model: LinearModel) -> Self {
        LinearStepper {
            current_score: model.clone().get_bias(),
            inputs: vec![0.0; model.input_size()],
            model,
            changed_squares: Vec::new(),
        }
    }

    pub fn set_input(&mut self, index: usize, val: f32) -> anyhow::Result<(), anyhow::Error> {
        let old_val = *self
            .inputs
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("Input doesn't exist at: {index}"))?;
        let weight = *self
            .model
            .get_weight(index)
            .ok_or_else(|| anyhow::anyhow!("Weight doesn't exist at: {index}"))?;
        *self
            .inputs
            .get_mut(index)
            .ok_or_else(|| anyhow::anyhow!("Input doesn't exist at: {index}"))? = val;
        self.current_score += (val - old_val) * weight;
        Ok(())
    }

    pub fn bias(&self) -> f32{
        self.model.bias
    }
    
    pub fn update_square<C>(&mut self, cb: &Chessboard<C>, square: usize)
    where
        C: BoardConfig + PartialEq + Eq,
        [(); 6 * C::AREA + 1 + STATE_FEATURES]: Sized,
        [(); C::AREA]: Sized,
    {
        assert!(square < C::AREA, "Square index is outside the board");
        let rank = square / C::WIDTH;
        let file = square % C::WIDTH;
        let mirrored_square = (C::HEIGHT - 1 - rank) * C::WIDTH + file;

        let piece = cb.piece_at(square);
        let mirrored_piece = cb.piece_at(mirrored_square);
        for piece_kind in 0..6 {
            let white_index = piece_kind * C::AREA + square;
            let black_index = piece_kind * C::AREA + mirrored_square;
            let white_value = match &piece {
                Piece::White(piece) if piece.to_index() as usize == piece_kind + 1 => 1.0,
                _ => 0.0,
            } + match &mirrored_piece {
                Piece::Black(piece) if piece.to_index() as usize == piece_kind + 1 => -1.0,
                _ => 0.0,
            };
            let black_value = match &mirrored_piece {
                Piece::White(piece) if piece.to_index() as usize == piece_kind + 1 => 1.0,
                _ => 0.0,
            } + match &piece {
                Piece::Black(piece) if piece.to_index() as usize == piece_kind + 1 => -1.0,
                _ => 0.0,
            };
            self.set_input(white_index, white_value).unwrap();
            self.set_input(black_index, black_value).unwrap();
        }
    }

    pub fn init<C>(&mut self, cb: &Chessboard<C>)
    where
        C: BoardConfig + PartialEq + Eq,
        [(); 6 * C::AREA + 1 + STATE_FEATURES]: Sized,
        [(); C::AREA]: Sized,
    {
        self.inputs = LinearModel::board_to_input(cb).to_vec();
        self.current_score = self.model.forward(self.inputs.as_slice());
        self.changed_squares.clear();
    }

    pub fn undo_step<C>(&mut self, cb: &Chessboard<C>)
    where
        C: BoardConfig + PartialEq + Eq,
        [(); 6 * C::AREA + 1 + STATE_FEATURES]: Sized,
        [(); C::AREA]: Sized,
    {
        if let Some(squares) = self.changed_squares.pop() {
            for square in squares {
                self.update_square(cb, square);
            }
            self.update_state_features(cb);
        } else {
            self.sync_to_board(cb);
        }
    }

    pub fn step<C>(&mut self, cb: &Chessboard<C>)
    where
        C: BoardConfig + PartialEq + Eq,
        [(); 6 * C::AREA + 1 + STATE_FEATURES]: Sized,
        [(); C::AREA]: Sized,
    {
        let move_ = cb
            .last_move()
            .expect("Stepper requires a board with an applied move")
            .moved();
        let mut squares = vec![move_.from() as usize, move_.to() as usize];
        match move_.flag() {
            MoveFlag::EnPassant => squares.push(if cb.current_player() == CurrentPlayer::Black {
                move_.to() as usize + C::WIDTH
            } else {
                move_.to() as usize - C::WIDTH
            }),
            MoveFlag::Castling => {
                if move_.to() > move_.from() {
                    squares.extend([(move_.from() + 3) as usize, (move_.from() + 1) as usize]);
                } else {
                    squares.extend([(move_.from() - 4) as usize, (move_.from() - 1) as usize]);
                }
            }
            _ => {}
        }
        squares.sort_unstable();
        squares.dedup();
        for &square in &squares {
            self.update_square(cb, square);
        }
        self.update_state_features(cb);
        self.changed_squares.push(squares);
    }

    fn sync_to_board<C>(&mut self, cb: &Chessboard<C>)
    where
        C: BoardConfig + PartialEq + Eq,
        [(); 6 * C::AREA + 1 + STATE_FEATURES]: Sized,
        [(); C::AREA]: Sized,
    {
        assert_eq!(
            self.inputs.len(),
            6 * C::AREA + 1 + STATE_FEATURES,
            "Model input size does not match board size"
        );

        for square in 0..C::AREA {
            self.update_square(cb, square);
        }
        self.update_state_features(cb);
    }

    fn update_state_features<C>(&mut self, cb: &Chessboard<C>)
    where
        C: BoardConfig + PartialEq + Eq,
        [(); C::AREA]: Sized,
    {
        let mut state = [0.0; 1 + STATE_FEATURES];
        state[0] = if cb.current_player() == CurrentPlayer::White {
            1.0
        } else {
            -1.0
        };
        state[1..].copy_from_slice(&LinearModel::state_features(cb));
        let state_start = 6 * C::AREA;
        for (offset, value) in state.into_iter().enumerate() {
            self.set_input(state_start + offset, value).unwrap();
        }
    }

    pub fn get_output(&self) -> f32 {
        self.current_score
    }
}

#[derive(Serialize, Deserialize)]
pub struct SimpleLayeredNetworkShape {
    pub main: LinearModel,
    #[serde(default)]
    pub hidden1: Option<Vec<LinearModel>>,

    #[serde(default)]
    pub hidden2: Option<Vec<LinearModel>>,

    #[serde(default)]
    pub output: Option<LinearModel>,
}
