use anyhow::{Error, Result, anyhow};

use crate::board_config::{BoardConfig};
use crate::chess_board::Chessboard;
use crate::ml::{LinearModel, LinearStepper, MLModel, SimpleLayeredNetworkShape};
use std::fmt::Debug;
use std::marker::PhantomData;

pub trait Evaluator<C: BoardConfig>
where
    [(); C::AREA]: Sized,
{
    fn evaluate(&self, cb: &Chessboard<C>) -> f32;
    fn init(&mut self, cb: &Chessboard<C>) {}
    fn on_make_move(&mut self, cb: &Chessboard<C>, m: &C::MoveType) {}
    fn on_undo_move(&mut self, cb: &Chessboard<C>, m: &C::MoveType) {}
}

#[derive(Clone)]
pub struct DeadSimpleEvaluator {}

impl<C> Evaluator<C> for DeadSimpleEvaluator
where
    C: BoardConfig + PartialEq + Eq,
    [(); C::AREA]: Sized,
{
    fn evaluate(&self, cb: &Chessboard<C>) -> f32 {
        cb.get_pieces()
            .iter()
            .map(|p| match p {
                crate::piece::Piece::White(a) => a.value() as f32,
                crate::piece::Piece::Black(a) => -a.value() as f32,
                _ => 0.0,
            })
            .sum()
    }
}

#[derive(Clone, Debug)]
pub struct MLEvaluator<C> {
    stepper: LinearStepper,
    _phantom_data: PhantomData<C>,
}

impl<C> MLEvaluator<C> {
    pub fn from_model(model: MLModel) -> Self {
        Self {
            stepper: LinearStepper::from_ml_model(model),
            _phantom_data: PhantomData,
        }
    }
}

impl<C> Evaluator<C> for MLEvaluator<C>
where
    C: BoardConfig + PartialEq + Eq,
    [(); 6 * C::AREA + 1 + 23]: Sized,
    [(); C::AREA]: Sized,
{
    fn evaluate(&self, _cb: &Chessboard<C>) -> f32 {
        self.stepper.get_output()
    }

    fn init(&mut self, cb: &Chessboard<C>) {
        self.stepper.init(cb);
    }

    fn on_make_move(&mut self, cb: &Chessboard<C>, _m: &C::MoveType) {
        self.stepper.step(cb);
    }

    fn on_undo_move(&mut self, cb: &Chessboard<C>, _m: &C::MoveType) {
        self.stepper.undo_step(cb);
    }
}

#[derive(Clone, Debug)]
pub struct LayeredMLEvaluator<C> {
    main_model: LinearStepper,
    hidden_model1: [LinearStepper; 64],
    hidden_model2: [LinearModel; 32],
    output: LinearModel,
    cached_output: f32,
    _phantom_data: PhantomData<C>,
}

impl<C> LayeredMLEvaluator<C> {
    pub fn new(model: SimpleLayeredNetworkShape) -> Result<Self, Error> {
        Ok(Self {
            main_model: LinearStepper::new(model.main),
            hidden_model1: model
                .hidden1
                .unwrap_or_default()
                .into_iter()
                .map(|h| LinearStepper::new(h))
                .collect::<Vec<LinearStepper>>()
                .try_into()
                .map_err(|v: Vec<LinearStepper>| {
                    anyhow!("Expected 64 elements, found {}", v.len())
                })?,
            hidden_model2: model
                .hidden2
                .unwrap_or_default()
                .into_iter()
                .collect::<Vec<LinearModel>>()
                .try_into()
                .map_err(|v: Vec<LinearModel>| {
                    anyhow!("Expected 32 elements, found {}", v.len())
                })?,
            output: model.output.expect("Expected a model, found none"),
            cached_output: 0.0,
            _phantom_data: PhantomData,
        })
    }

    pub fn load_model(path: &str) -> anyhow::Result<LayeredMLEvaluator<C>, anyhow::Error> {
        let json = std::fs::read_to_string(path)?;
        let shape: SimpleLayeredNetworkShape = serde_json::from_str(&json)
            .map_err(|_| anyhow::anyhow!("Couldn't deserialize json model."))?;
        LayeredMLEvaluator::<C>::new(shape)
    }

    fn softsign(x: f32) -> f32 {
        x / (1.0 + x.abs())
    }

    pub fn evaluate_layers(&self) -> f32 {
        let mut layer1_outputs = [0.0; 64];
        for node in self.hidden_model1.iter().enumerate() {
            layer1_outputs[node.0] = Self::softsign(node.1.get_output());
        }
        let mut layer2_outputs = [0.0; 32];
        for node in self.hidden_model2.iter().enumerate() {
            layer2_outputs[node.0] = Self::softsign(node.1.forward(&layer1_outputs));
        }

        self.output.forward(&layer2_outputs)
    }
}

impl<C> Evaluator<C> for LayeredMLEvaluator<C>
where
    C: BoardConfig + PartialEq + Eq,
    [(); 6 * C::AREA + 1 + 23]: Sized,
    [(); C::AREA]: Sized,
{
    fn evaluate(&self, _cb: &Chessboard<C>) -> f32 {
        f32::round(self.cached_output)
    }

    fn init(&mut self, cb: &Chessboard<C>) {
        self.main_model.init(cb);
        for layer1 in &mut self.hidden_model1 {
            layer1.init(cb);
        }
        self.cached_output = self.evaluate_layers() * 100.0 + self.main_model.get_output();
    }

    fn on_make_move(&mut self, cb: &Chessboard<C>, _m: &C::MoveType) {
        self.main_model.step(cb);
        for layer1 in &mut self.hidden_model1 {
            layer1.step(cb);
        }
        self.cached_output = self.evaluate_layers() * 100.0 + self.main_model.get_output();
    }

    fn on_undo_move(&mut self, cb: &Chessboard<C>, _m: &C::MoveType) {
        self.main_model.undo_step(cb);
        for layer1 in &mut self.hidden_model1 {
            layer1.undo_step(cb);
        }
        self.cached_output = self.evaluate_layers() * 100.0 + self.main_model.get_output();
    }
}
