use anyhow::{anyhow, Error, Result};

use crate::board_config::{BoardConfig, RegularVariant};
use crate::chess_board::Chessboard;
use crate::ml::{LinearModel, LinearStepper, SimpleLayeredNetworkShape};
use std::fmt::Debug;
use std::marker::PhantomData;

pub trait Evaluator<C: BoardConfig>
where
    [(); C::AREA]: Sized,
{
    fn evaluate(&self, cb: &Chessboard<C>) -> i32;
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
    fn evaluate(&self, cb: &Chessboard<C>) -> i32 {
        cb.get_pieces()
            .iter()
            .map(|p| match p {
                crate::piece::Piece::White(a) => a.value(),
                crate::piece::Piece::Black(a) => -a.value(),
                _ => 0,
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
    pub fn new(model: LinearModel) -> Self {
        Self {
            stepper: LinearStepper::new(model),
            _phantom_data: PhantomData,
        }
    }
}

impl<C> Evaluator<C> for MLEvaluator<C>
where
    C: BoardConfig + PartialEq + Eq,
    [(); 6 * C::AREA + 1]: Sized,
    [(); C::AREA]: Sized,
{
    fn evaluate(&self, cb: &Chessboard<C>) -> i32 {
        let positional = self.stepper.get_output();

        positional.round() as i32
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
pub struct SimpleLayeredMLEvaluator<C> {
    main_model: LinearStepper,
    hidden_model: [LinearStepper; 16],
    _phantom_data: PhantomData<C>
}

impl<C> SimpleLayeredMLEvaluator<C> {
    pub fn new(model: SimpleLayeredNetworkShape) -> Result<Self, Error> {
        Ok(Self {
            main_model: LinearStepper::new(model.main),
            hidden_model: model.hidden
                .into_iter()
                .map(|h| LinearStepper::new(h))
                .collect::<Vec<LinearStepper>>()
                .try_into()
                .map_err(|v: Vec<LinearStepper>| anyhow!("Expected 16 elements, found {}", v.len()))?,
            _phantom_data: PhantomData,
        })
    }
}

impl<C> Evaluator<C> for SimpleLayeredMLEvaluator<C>
where
    C: BoardConfig + PartialEq + Eq,
    [(); 6 * C::AREA + 1]: Sized,
    [(); C::AREA]: Sized,
{
    fn evaluate(&self, cb: &Chessboard<C>) -> i32 {
        todo!();
    }

    fn init(&mut self, cb: &Chessboard<C>) {
        todo!();
    }

    fn on_make_move(&mut self, cb: &Chessboard<C>, _m: &C::MoveType) {
        todo!();
    }

    fn on_undo_move(&mut self, cb: &Chessboard<C>, _m: &C::MoveType) {
        todo!();
    }
}
