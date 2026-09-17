#![feature(generic_const_exprs)]
use chess_engine::game_controller::*;
use chess_engine::board_config::*;
fn main() {
    GameController::<RegularVariant>::uci_loop();
}