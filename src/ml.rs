use crate::board_config::BoardConfig;
use crate::chess_board::{Chessboard, CurrentPlayer};
use serde::{Serialize, Deserialize};
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

    pub fn board_to_input<C>(cb: &Chessboard<C>) -> [f32; 6 * C::AREA + 1]
    where
        C: BoardConfig + PartialEq + Eq,
        [(); 6 * C::AREA + 1]: Sized,
        [(); C::AREA]: Sized,
    {
        let mut input = [0.0; 6 * C::AREA + 1];

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

        input
    }
}

#[derive(Clone, Debug)]
pub struct LinearStepper {
    model: LinearModel,
    inputs: Vec<f32>,
    current_score: f32,
}

impl LinearStepper {
    pub fn new(model: LinearModel) -> Self {
        LinearStepper {
            current_score: model.clone().get_bias(),
            inputs: vec![0.0; model.input_size()],
            model,
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

    pub fn update_square<C>(&mut self, cb: &Chessboard<C>, square: usize)
    where
        C: BoardConfig + PartialEq + Eq,
        [(); 6 * C::AREA + 1]: Sized,
        [(); C::AREA]: Sized,
    {
        assert!(square < C::AREA, "Square index is outside the board");
        let input = LinearModel::board_to_input(cb);
        let rank = square / C::WIDTH;
        let file = square % C::WIDTH;
        let mirrored_square = (C::HEIGHT - 1 - rank) * C::WIDTH + file;

        for piece_kind in 0..6 {
            for square in [square, mirrored_square] {
                let index = piece_kind * C::AREA + square;
                if self.inputs[index] != input[index] {
                    self.set_input(index, input[index]).unwrap();
                }
            }
        }
    }

    pub fn init<C>(&mut self, cb: &Chessboard<C>)
    where
        C: BoardConfig + PartialEq + Eq,
        [(); 6 * C::AREA + 1]: Sized,
        [(); C::AREA]: Sized,
    {
        self.inputs = LinearModel::board_to_input(cb).to_vec();
        self.current_score = self.model.forward(self.inputs.as_slice());
    }

    pub fn undo_step<C>(&mut self, cb: &Chessboard<C>)
    where
        C: BoardConfig + PartialEq + Eq,
        [(); 6 * C::AREA + 1]: Sized,
        [(); C::AREA]: Sized,
    {
        self.sync_to_board(cb);
    }

    pub fn step<C>(&mut self, cb: &Chessboard<C>)
    where
        C: BoardConfig + PartialEq + Eq,
        [(); 6 * C::AREA + 1]: Sized,
        [(); C::AREA]: Sized,
    {
        self.sync_to_board(cb);
    }

    fn sync_to_board<C>(&mut self, cb: &Chessboard<C>)
    where
        C: BoardConfig + PartialEq + Eq,
        [(); 6 * C::AREA + 1]: Sized,
        [(); C::AREA]: Sized,
    {
        let input = LinearModel::board_to_input(cb);
        assert_eq!(
            self.inputs.len(),
            input.len(),
            "Model input size does not match board size"
        );

        let mut visited = vec![false; C::AREA];
        for square in 0..C::AREA {
            if visited[square] {
                continue;
            }
            let rank = square / C::WIDTH;
            let file = square % C::WIDTH;
            let mirrored_square = (C::HEIGHT - 1 - rank) * C::WIDTH + file;
            let changed = (0..6).any(|piece_kind| {
                self.inputs[piece_kind * C::AREA + square] != input[piece_kind * C::AREA + square]
                    || self.inputs[piece_kind * C::AREA + mirrored_square]
                        != input[piece_kind * C::AREA + mirrored_square]
            });
            if changed {
                self.update_square(cb, square);
            }
            visited[square] = true;
            visited[mirrored_square] = true;
        }

        let turn_index = 6 * C::AREA;
        if self.inputs[turn_index] != input[turn_index] {
            self.set_input(turn_index, input[turn_index]).unwrap();
        }
    }

    pub fn get_output(&self) -> f32 {
        self.current_score
    }
}

#[derive(Serialize, Deserialize)]
pub struct SimpleLayeredNetworkShape {
    pub main: LinearModel,
    pub hidden: [LinearModel; 16],
}
