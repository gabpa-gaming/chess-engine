#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use chess_engine::board_config::{BoardConfig, RegularVariant};
use chess_engine::chess_board::Chessboard;
use chess_engine::ml::FeatureMapping;

#[test]
fn color_separated_mapping_uses_distinct_planes_per_color() {
    let board = Chessboard::<RegularVariant>::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
    let input = FeatureMapping::TwoColorsPerMapping.board_to_input(&board);
    let area = RegularVariant::AREA;

    assert_eq!(input.len(), 12 * area + 24);
    assert_eq!(input[5 * area + 60], 1.0);
    assert_eq!(input[11 * area + 4], 1.0);
    assert_eq!(input[5 * area + 4], 0.0);
    assert_eq!(input[11 * area + 60], 0.0);
}
