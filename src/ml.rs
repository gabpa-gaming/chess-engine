use crate::board_config::BoardConfig;
use crate::chess_board::{Chessboard, CurrentPlayer};
use crate::chess_move::{MoveFlag, MoveTrait};
use crate::piece::Piece;
use serde::{Deserialize, Serialize};

const STATE_FEATURES: usize = 23;


#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FeatureMapping {
    #[default]
    ColorPerMapping,
    TwoColorsPerMapping,
}

impl FeatureMapping {
    pub fn input_size<C: BoardConfig>(self) -> usize {
        self.piece_planes() * C::AREA + 1 + STATE_FEATURES
    }

    fn piece_planes(self) -> usize {
        match self {
            Self::ColorPerMapping => 6,
            Self::TwoColorsPerMapping => 12,
        }
    }

    pub fn board_to_input<C>(self, cb: &Chessboard<C>) -> Vec<f32>
    where
        C: BoardConfig + PartialEq + Eq,
        [(); C::AREA]: Sized,
    {
        let mut input = vec![0.0; self.input_size::<C>()];
        for square in 0..C::AREA {
            self.write_square(cb, square, &mut input);
        }
        self.write_state(cb, &mut input);
        input
    }

    fn write_square<C>(self, cb: &Chessboard<C>, square: usize, input: &mut [f32])
    where
        C: BoardConfig + PartialEq + Eq,
        [(); C::AREA]: Sized,
    {
        self.for_each_square_update(cb, square, |index, value| input[index] = value);
    }

    fn for_each_square_update<C>(
        self,
        cb: &Chessboard<C>,
        square: usize,
        mut update: impl FnMut(usize, f32),
    )
    where
        C: BoardConfig + PartialEq + Eq,
        [(); C::AREA]: Sized,
    {
        debug_assert!(square < C::AREA);
        let rank = square / C::WIDTH;
        let file = square % C::WIDTH;
        let mirrored_square = (C::HEIGHT - 1 - rank) * C::WIDTH + file;

        match self {
            Self::ColorPerMapping => {
                for (target, white_square, black_square) in [
                    (square, square, mirrored_square),
                    (mirrored_square, mirrored_square, square),
                ] {
                    for kind in 0..6 {
                        update(kind * C::AREA + target, matches!(cb.piece_at(white_square), Piece::White(piece) if piece.to_index() as usize == kind + 1)
                            as u8 as f32
                            - matches!(cb.piece_at(black_square), Piece::Black(piece) if piece.to_index() as usize == kind + 1)
                                as u8 as f32);
                    }
                }
            }
            Self::TwoColorsPerMapping => {
                for kind in 0..6 {
                    update(kind * C::AREA + square, matches!(cb.piece_at(square), Piece::White(piece) if piece.to_index() as usize == kind + 1) as u8 as f32);
                    update((6 + kind) * C::AREA + square, matches!(cb.piece_at(square), Piece::Black(piece) if piece.to_index() as usize == kind + 1) as u8 as f32);
                }
            }
        }
    }

    fn write_state<C>(self, cb: &Chessboard<C>, input: &mut [f32])
    where
        C: BoardConfig + PartialEq + Eq,
        [(); C::AREA]: Sized,
    {
        let state_start = self.piece_planes() * C::AREA;
        input[state_start] = if cb.current_player() == CurrentPlayer::White {
            1.0
        } else {
            -1.0
        };
        input[state_start + 1..].copy_from_slice(&LinearModel::state_features(cb));
    }
}

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
        let white_start = (C::HEIGHT - 1) * C::WIDTH;
        let black_start = 0;
        input[0] = ((rights & ((1_u128 << (white_start + 4)) | (1_u128 << (white_start + C::WIDTH - 1)))) != 0) as u8 as f32;
        input[1] = ((rights & ((1_u128 << (white_start + 4)) | (1_u128 << white_start))) != 0) as u8 as f32;
        input[2] = ((rights & ((1_u128 << (black_start + 4)) | (1_u128 << (black_start + C::WIDTH - 1)))) != 0) as u8 as f32;
        input[3] = ((rights & ((1_u128 << (black_start + 4)) | (1_u128 << black_start))) != 0) as u8 as f32;

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
        let current_king_exists = cb.get_pieces().iter().any(|piece| {
            matches!(piece, Piece::White(crate::piece::PieceType::King(_)) | Piece::Black(crate::piece::PieceType::King(_)))
        });
        input[21] = (current_king_exists && cb.is_king_checked(cb.current_player())) as u8 as f32;
        input[22] = cb.halfmove_clock() as f32;
        input
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MLModel {
    #[serde(flatten)]
    linear: LinearModel,
    #[serde(default)]
    mapping: FeatureMapping,
}

impl MLModel {
    pub fn new(linear: LinearModel, mapping: FeatureMapping) -> Self {
        Self { linear, mapping }
    }

    pub fn load_model(path: &str) -> anyhow::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json).map_err(|_| anyhow::anyhow!("Couldn't deserialize json model."))
    }

    pub fn mapping(&self) -> FeatureMapping {
        self.mapping
    }

    fn into_parts(self) -> (LinearModel, FeatureMapping) {
        (self.linear, self.mapping)
    }
}

#[derive(Clone, Debug)]
pub struct LinearStepper {
    model: LinearModel,
    mapping: FeatureMapping,
    inputs: Vec<f32>,
    current_score: f32,
    changed_squares: Vec<Vec<usize>>,
}

impl LinearStepper {
    pub fn new(model: LinearModel) -> Self {
        Self::with_mapping(model, FeatureMapping::ColorPerMapping)
    }

    pub fn with_mapping(model: LinearModel, mapping: FeatureMapping) -> Self {
        LinearStepper {
            current_score: model.clone().get_bias(),
            inputs: vec![0.0; model.input_size()],
            model,
            mapping,
            changed_squares: Vec::new(),
        }
    }

    pub fn from_ml_model(model: MLModel) -> Self {
        let (linear, mapping) = model.into_parts();
        Self::with_mapping(linear, mapping)
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

    pub fn bias(&self) -> f32 {
        self.model.bias
    }

    pub fn update_square<C>(&mut self, cb: &Chessboard<C>, square: usize)
    where
        C: BoardConfig + PartialEq + Eq,
        [(); 6 * C::AREA + 1 + STATE_FEATURES]: Sized,
        [(); C::AREA]: Sized,
    {
        assert!(square < C::AREA, "Square index is outside the board");
        let mapping = self.mapping;
        mapping.for_each_square_update(cb, square, |index, value| {
            if self.inputs[index] != value {
                self.set_input(index, value).unwrap();
            }
        });
    }

    pub fn init<C>(&mut self, cb: &Chessboard<C>)
    where
        C: BoardConfig + PartialEq + Eq,
        [(); 6 * C::AREA + 1 + STATE_FEATURES]: Sized,
        [(); C::AREA]: Sized,
    {
        self.inputs = self.mapping.board_to_input(cb);
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
            self.mapping.input_size::<C>(),
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
        let state_start = self.mapping.piece_planes() * C::AREA;
        let turn = if cb.current_player() == CurrentPlayer::White {
            1.0
        } else {
            -1.0
        };
        self.set_input(state_start, turn).unwrap();
        for (offset, value) in LinearModel::state_features(cb).into_iter().enumerate() {
            self.set_input(state_start + 1 + offset, value).unwrap();
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
