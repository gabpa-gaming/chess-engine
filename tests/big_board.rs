#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use chess_engine::bitboard::Bitboard;
use chess_engine::board_config::{BigVariant, BoardConfig};
use chess_engine::chess_board::Chessboard;
use chess_engine::chess_move::{MoveFlag, MoveTrait};
use chess_engine::piece::Piece;

#[test]
fn big_board_start_position_generates_moves() {
    let mut board = Chessboard::<BigVariant>::regular_board();
    assert_eq!(board.get_pieces().iter().filter(|piece| matches!(piece, Piece::White(_))).count(), BigVariant::WIDTH * 2);
    assert!(matches!(board.generate_moves(), Ok(moves) if !moves.is_empty()));
}

#[test]
fn big_board_castling_moves_outer_rook_and_undoes() {
    let mut board = Chessboard::<BigVariant>::from_fen(
        "4k5/10/10/10/10/10/10/10/10/10/10/R3K4R w KQkq - 0 1",
    )
    .unwrap();
    let original = board.clone();
    let moves = match board.generate_moves() {
        Ok(moves) => moves,
        Err(_) => panic!("expected legal moves"),
    };
    let castle = moves
        .into_iter()
        .find(|move_| move_.flag() == MoveFlag::Castling && move_.to() > move_.from())
        .unwrap();

    board.apply_move(castle).unwrap();
    assert!(matches!(board.piece_at(116), Piece::White(_)));
    assert!(matches!(board.piece_at(115), Piece::White(_)));
    board.undo_move();
    assert_eq!(board, original);
}

#[test]
fn big_board_pawns_promote_on_the_first_and_last_ranks() {
    let mut board = Chessboard::<BigVariant>::from_fen(
        "4k5/P9/10/10/10/10/10/10/10/10/10/4K5 w - - 0 1",
    )
    .unwrap();
    let promotions = match board.generate_moves() {
        Ok(moves) => moves,
        Err(_) => panic!("expected legal moves"),
    }
    .into_iter()
    .filter(|move_| move_.flag() == MoveFlag::Promotion)
    .count();
    assert_eq!(promotions, 4);
}
