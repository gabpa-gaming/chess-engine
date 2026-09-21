#![feature(generic_const_exprs)]
use chess_engine::board_config::*;
use chess_engine::game_controller::*;
fn main() {
    GameController::<RegularVariant>::uci_loop();
}
