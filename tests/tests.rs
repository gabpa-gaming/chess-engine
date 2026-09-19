#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
#[cfg(test)]
mod tests {
    use chess_engine::board_config::{BigVariant, BoardConfig, RegularVariant};
    use chess_engine::chess_board::{Chessboard, GameStatus, MoveLegality};
    use chess_engine::game_controller::GameController;
    use chess_engine::ml::{LinearModel, LinearStepper};
    use chess_engine::chess_move::{MoveFlag, MoveTrait};
    use super::*; 

    #[test]
    fn test_standard_start_position() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        ).unwrap();

        assert_eq!(game.perf_mode(1, true), 20);
        assert_eq!(game.perf_mode(2, true), 400);
        assert_eq!(game.perf_mode(3, true), 8_902);
    }

    #[test]
    fn test_position_2_kiwipete() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"
        ).unwrap();

        assert_eq!(game.perf_mode(1, true), 48);
        assert_eq!(game.perf_mode(2, true), 2_039);
        assert_eq!(game.perf_mode(3, true), 97_862);
    }

    #[test]
    fn test_position_3_endgame_pin() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"
        ).unwrap();

        assert_eq!(game.perf_mode(1, true), 14);
        assert_eq!(game.perf_mode(2, true), 191);
        assert_eq!(game.perf_mode(3, true), 2_812);
        assert_eq!(game.perf_mode(4, true), 43_238);
    }

    #[test]
    fn test_position_4_nightmare_check() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1"
        ).unwrap();

        assert_eq!(game.perf_mode(1, true), 6);
        assert_eq!(game.perf_mode(2, true), 264);
        assert_eq!(game.perf_mode(3, true), 9_467);
    }

    #[test]
    fn test_position_5_underpromotions() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8"
        ).unwrap();

        assert_eq!(game.perf_mode(1, true), 44);
        assert_eq!(game.perf_mode(2, true), 1_486);
        assert_eq!(game.perf_mode(3, true), 62_379);
    }

    #[test]
    fn test_position_6_symmetrical_middlegame() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10"
        ).unwrap();

        assert_eq!(game.perf_mode(1, true), 46);
        assert_eq!(game.perf_mode(2, true), 2_079);
        assert_eq!(game.perf_mode(3, true), 89_890);
    }

    #[test]
    fn test_position_07_isolated_kingside_castling() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "5k2/8/8/8/8/8/8/4K2R w K - 0 1"
        ).unwrap();
        assert_eq!(game.perf_mode(6, true), 661_072);
    }

    #[test]
    fn test_position_08_isolated_queenside_castling() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "3k4/8/8/8/8/8/8/R3K3 w Q - 0 1"
        ).unwrap();
        assert_eq!(game.perf_mode(6, true), 803_711);
    }

    #[test]
    fn test_position_09_en_passant_with_check() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "8/8/1k6/2b5/2pP4/8/5K2/8 b - d3 0 1"
        ).unwrap();
        assert_eq!(game.perf_mode(6, true), 1_440_467);
    }

    #[test]
    fn test_position_10_promotion_with_check_and_capture() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "2K2r2/4P3/8/8/8/8/8/3k4 w - - 0 1"
        ).unwrap();
        assert_eq!(game.perf_mode(6, true), 3_821_001);
    }

    #[test]
    fn test_position_11_simple_promotion() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "4k3/1P6/8/8/8/8/K7/8 w - - 0 1"
        ).unwrap();
        assert_eq!(game.perf_mode(6, true), 217_342);
    }

    #[test]
    fn test_position_12_castling_through_tension() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "r3k2r/1b4bq/8/8/8/8/7B/R3K2R w KQkq - 0 1"
        ).unwrap();
        assert_eq!(game.perf_mode(4, true), 1_274_206);
    }

    #[test]
    fn test_position_13_heavy_pieces_castling() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "r3k2r/8/3Q4/8/8/5q2/8/R3K2R b KQkq - 0 1"
        ).unwrap();
        assert_eq!(game.perf_mode(4, true), 1_720_476);
    }

    #[test]
    fn test_position_14_bishop_pawn_tension() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "8/8/4k3/8/2p5/8/B2P2K1/8 w - - 0 1"
        ).unwrap();
        assert_eq!(game.perf_mode(6, true), 1_015_133);
    }

    #[test]
    fn test_position_15_rook_pawn_tension() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "3k4/3p4/8/K1P4r/8/8/8/8 b - - 0 1"
        ).unwrap();
        assert_eq!(game.perf_mode(6, true), 1_134_888);
    }

    #[test]
    fn test_position_16_queen_knight_check_evasion() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "8/8/1P2K3/8/2n5/1q6/8/5k2 b - - 0 1"
        ).unwrap();
        assert_eq!(game.perf_mode(5, true), 1_004_658);
    }

    fn assert_undo_clean(fen: &str) {
        let mut game = Chessboard::<RegularVariant>::from_fen(fen).unwrap();
        let original_state = game.clone();
        let original_hash = game.zobrist_hash();

        let moves = game.generate_moves().unwrap_or(Vec::new());

        for m in moves {
            game.apply_move(m);
            assert_ne!(game.zobrist_hash(), original_hash, "Hash did not change for move: {:?}", m);
            game.undo_move();
            assert_eq!(game.zobrist_hash(), original_hash, "Hash was not restored for move: {:?}", m);

            assert_eq!(
                game,
                original_state,
                "\nUndo desync detected. \nMove tried: {:?}\nFEN: {}\n",
                m, fen
            );
        }
    }

    #[test]
    fn zobrist_hash_includes_position_state() {
        let white_to_move = Chessboard::<RegularVariant>::from_fen(
            "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
        )
        .unwrap();
        let black_to_move = Chessboard::<RegularVariant>::from_fen(
            "4k3/8/8/8/8/8/8/4K3 b - - 0 1",
        )
        .unwrap();
        let castling_right = Chessboard::<RegularVariant>::from_fen(
            "4k3/8/8/8/8/8/8/4K2R w K - 0 1",
        )
        .unwrap();
        let no_castling_right = Chessboard::<RegularVariant>::from_fen(
            "4k3/8/8/8/8/8/8/4K2R w - - 0 1",
        )
        .unwrap();
        let en_passant = Chessboard::<RegularVariant>::from_fen(
            "4k3/8/8/8/3Pp3/8/8/4K3 w - d3 0 1",
        )
        .unwrap();
        let no_en_passant = Chessboard::<RegularVariant>::from_fen(
            "4k3/8/8/8/3Pp3/8/8/4K3 w - - 0 1",
        )
        .unwrap();

        assert_ne!(white_to_move.zobrist_hash(), black_to_move.zobrist_hash());
        assert_ne!(castling_right.zobrist_hash(), no_castling_right.zobrist_hash());
        assert_ne!(en_passant.zobrist_hash(), no_en_passant.zobrist_hash());
    }

    #[test]
    fn game_ends_after_threefold_repetition() {
        let mut game = GameController::<RegularVariant>::from_fen(
            "4k1n1/8/8/8/8/8/8/4K1N1 w - - 0 1",
        )
        .unwrap();

        for uci in ["g1f3", "g8f6", "f3g1", "f6g8", "g1f3", "g8f6", "f3g1", "f6g8"] {
            let move_ = game.from_uci(uci).expect("cycle move should be legal");
            assert_eq!(game.move_piece(move_), MoveLegality::Legal);
        }

        assert!(matches!(
            game.chessboard_mut().generate_moves(),
            Err(GameStatus::DrawRepetition)
        ));
    }


    #[test]
    fn undo_01_castling_physical_pieces() {
        assert_undo_clean("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    }

    #[test]
    fn undo_02_castling_rights_loss_king_move() {
        assert_undo_clean("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1");
    }

    #[test]
    fn undo_03_castling_rights_loss_rook_capture() {
        assert_undo_clean("r3k2r/p6p/8/8/8/8/P6P/R3K2R w KQkq - 0 1");
    }

    #[test]
    fn undo_04_en_passant_target_creation() {
        assert_undo_clean("k7/8/8/8/8/8/3P4/K7 w - - 0 1");
    }

    #[test]
    fn undo_05_en_passant_capture_restoration() {
        assert_undo_clean("k7/8/8/8/3Pp3/8/8/K7 b - d3 0 1");
    }

    #[test]
    fn undo_06_promotion_piece_demotion() {
        assert_undo_clean("k7/3P4/8/8/8/8/8/K7 w - - 0 1");
    }

    #[test]
    fn undo_07_promotion_capture_restoration() {
        assert_undo_clean("n1n4k/3P4/8/8/8/8/8/K7 w - - 0 1");
    }

    #[test]
    fn undo_08_halfmove_clock_reset_on_capture() {
        assert_undo_clean("k7/8/8/8/3p4/4P3/8/K7 w - - 49 1");
    }

    #[test]
    fn undo_09_halfmove_clock_reset_on_pawn_push() {
        assert_undo_clean("k7/8/8/8/8/8/3P4/K7 w - - 25 1");
    }

    #[test]
    fn undo_10_turn_and_history_pop() {
        assert_undo_clean("k7/8/8/8/8/8/8/K3k3 b - - 0 1");
    }

    fn model() -> LinearModel {
        let weights = (1..=408)
            .map(|weight| weight.to_string())
            .collect::<Vec<_>>()
            .join(",");
        serde_json::from_str(&format!(r#"{{"weights":[{weights}],"bias":-1.0}}"#)).unwrap()
    }

    fn assert_step_and_undo_match_init(fen: &str, flag: MoveFlag) {
        let mut board = Chessboard::<RegularVariant>::from_fen(fen).unwrap();
        let original_board = board.clone();
        let moves = match board.generate_moves() {
            Ok(moves) => moves,
            Err(_) => panic!("No legal moves available for {fen}"),
        };
        let move_ = moves
            .into_iter()
            .find(|move_| move_.flag() == flag)
            .unwrap_or_else(|| panic!("No {flag:?} move available for {fen}"));

        board.apply_move(move_).unwrap();
        let mut stepped = LinearStepper::new(model());
        stepped.init(&original_board);
        stepped.step(&board);

        let mut initialized = LinearStepper::new(model());
        initialized.init(&board);
        assert_eq!(stepped.get_output(), initialized.get_output());

        board.undo_move();
        stepped.undo_step(&board);

        initialized.init(&board);
        assert_eq!(stepped.get_output(), initialized.get_output());
    }

    #[test]
    fn stepper_quiet_move_matches_init() {
        assert_step_and_undo_match_init("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1", MoveFlag::Quiet);
    }

    #[test]
    fn stepper_capture_matches_init() {
        assert_step_and_undo_match_init("4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1", MoveFlag::Capture);
    }

    #[test]
    fn stepper_double_pawn_push_matches_init() {
        assert_step_and_undo_match_init(
            "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1",
            MoveFlag::DoublePawnPush,
        );
    }

    #[test]
    fn stepper_en_passant_matches_init() {
        assert_step_and_undo_match_init(
            "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
            MoveFlag::EnPassant,
        );
    }

    #[test]
    fn stepper_castling_matches_init() {
        assert_step_and_undo_match_init("4k3/8/8/8/8/8/8/4K2R w K - 0 1", MoveFlag::Castling);
    }

    #[test]
    fn stepper_promotion_matches_init() {
        assert_step_and_undo_match_init("4k3/P7/8/8/8/8/8/4K3 w - - 0 1", MoveFlag::Promotion);
    }

    #[test]
    fn board_input_scales_with_board_area() {
        let board = Chessboard::<BigVariant>::new();
        let input = LinearModel::board_to_input(&board);

        assert_eq!(input.len(), 6 * BigVariant::AREA + 24);
        assert_eq!(input[6 * BigVariant::AREA], 1.0);
    }

    #[test]
    fn board_input_includes_position_state_features() {
        let board = Chessboard::<RegularVariant>::from_fen(
            "4k3/8/8/8/8/8/4R3/R3K2R b KQkq e3 17 1",
        )
        .unwrap();
        let input = LinearModel::board_to_input(&board);
        let state_start = 6 * RegularVariant::AREA + 1;

        assert_eq!(&input[state_start..state_start + 4], &[1.0; 4]);
        assert_eq!(input[state_start + 4 + 12], 1.0);
        assert_eq!(input[state_start + 20], 1.0);
        assert_eq!(input[state_start + 21], 1.0);
        assert_eq!(input[state_start + 22], 17.0);
    }
}
