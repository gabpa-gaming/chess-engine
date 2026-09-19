use crate::bishop::Bishop;
use crate::board_config::{BoardConfig, RegularVariant};
use crate::chess_move::MoveFlag::Promotion;
use crate::chess_move::{MoveFlag, MoveTrait};
use crate::knight::Knight;
use crate::pawn;
use crate::piece::PieceType::Pawn;
use crate::piece::{Piece, PieceType};
use crate::queen::Queen;
use crate::rook::Rook;
use std::fmt::Debug;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum CurrentPlayer {
    White,
    Black,
}

impl CurrentPlayer {
    pub fn other(&self) -> CurrentPlayer {
        match self {
            CurrentPlayer::White => CurrentPlayer::Black,
            CurrentPlayer::Black => CurrentPlayer::White,
        }
    }
}

pub enum GameStatus {
    DrawRepetition,
    DrawStalemate,
    DrawInsufficientMaterial,
    Won(CurrentPlayer),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MoveHistoryData<C: BoardConfig + Clone + Debug>
where
    [(); C::AREA]: Sized,
{
    moved: C::MoveType,
    last_castling_data: u128,
    captured: PieceType<C>,
    en_passant: Option<u8>,
    halfmove_clock: u8,
    zobrist: u64,
}

impl<C: BoardConfig + Clone + Debug> MoveHistoryData<C>
where
    [(); C::AREA]: Sized,
{
    pub fn moved(&self) -> C::MoveType {
        self.moved
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Chessboard<C: BoardConfig + Clone + Debug>
where
    [(); C::AREA]: Sized,
{
    pieces: [PieceType<C>; C::AREA],
    occupancy_board: u128,
    black_pieces_bitboard: u128,
    white_pieces_bitboard: u128,
    castling_rights: u128,
    en_passant_square: Option<u8>,
    halfmove_clock: u8,
    turn: CurrentPlayer,
    zobrist: u64,
    move_history: Vec<MoveHistoryData<C>>,
}

impl<C: BoardConfig + Clone + Debug + PartialEq + Eq> Chessboard<C>
where
    [(); C::AREA]: Sized,
{
    pub const fn height() -> usize {
        C::HEIGHT
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum MoveLegality {
    Legal,
    SemiLegal,
    IllegalPieceTypePinnedToKing,
    IllegalKingStillInCheck,
    IllegalOther,
}

impl<C> Chessboard<C>
where
    C: BoardConfig + PartialEq + Eq,
    [(); C::AREA]: Sized,
{
    fn castling_rank(player: CurrentPlayer) -> usize {
        match player {
            CurrentPlayer::White => C::HEIGHT - 1,
            CurrentPlayer::Black => 0,
        }
    }

    fn castling_right_bits(player: CurrentPlayer, kingside: bool) -> u128 {
        let rank_start = Self::castling_rank(player) * C::WIDTH;
        let king = rank_start + 4;
        let rook = if kingside { rank_start + C::WIDTH - 1 } else { rank_start };
        (1_u128 << king) | (1_u128 << rook)
    }

    pub fn new() -> Chessboard<C> {
        let mut pieces_vec: Vec<PieceType<C>> = Vec::with_capacity(C::AREA);
        for _ in 0..C::AREA {
            pieces_vec.push(PieceType::None);
        }
        let mut pieces = std::mem::ManuallyDrop::new(pieces_vec);
        let pieces_ptr = pieces.as_mut_ptr();
        let pieces = unsafe { std::ptr::read(pieces_ptr as *const [PieceType<C>; C::AREA]) };

        Self {
            pieces,
            turn: CurrentPlayer::White,
            occupancy_board: 0,
            black_pieces_bitboard: 0,
            white_pieces_bitboard: 0,
            castling_rights: 0,
            en_passant_square: None,
            halfmove_clock: 0,
            zobrist: 0,
            move_history: Vec::new(),
        }
    }
    pub fn regular_board() -> Chessboard<C> {
        let fen = if C::WIDTH == 8 && C::HEIGHT == 8 {
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        } else if C::WIDTH == 10 && C::HEIGHT == 12 {
            "rnbqkbnbnr/pppppppppp/10/10/10/10/10/10/10/10/PPPPPPPPPP/RNBQKBNBNR w KQkq - 0 1"
        } else {
            panic!("No starting position defined for this board size")
        };
        Chessboard::from_fen(fen).unwrap()
    }

    pub fn is_move_semilegal(&self, move_: C::MoveType) -> MoveLegality
    where
            {
        match self.pieces[move_.from() as usize].clone() {
            PieceType::<C>::None => MoveLegality::IllegalOther,
            piece => {
                //room for optimization
                let enemy_board = match self.current_player() {
                    CurrentPlayer::Black => self.white_pieces_bitboard,
                    CurrentPlayer::White => self.black_pieces_bitboard,
                };
                let move_bitboard = (piece.get_move_bitboard(
                    move_.from(),
                    self.occupancy_board,
                    self.current_player(),
                ) & (!self.occupancy_board))
                    | (piece.get_attack_bitboard(
                        move_.from(),
                        self.occupancy_board,
                        self.current_player(),
                    ) & enemy_board);

                if (move_bitboard & (1 << move_.to())) > 0 {
                    return MoveLegality::SemiLegal;
                }
                MoveLegality::IllegalOther
            }
        }
    }

    pub fn is_move_legal(&mut self, move_: C::MoveType) -> MoveLegality {
        let moves: Vec<C::MoveType> = self.generate_moves().unwrap_or(Vec::new());
        if moves
            .iter()
            .find(|m| m.from() == move_.from() && m.to() == move_.to() && m.flag() == move_.flag())
            .is_none()
        {
            MoveLegality::IllegalOther
        } else {
            MoveLegality::Legal
        }
    }

    pub fn is_king_checked(&self, current_player: CurrentPlayer) -> bool {
        let king_pos = self.get_king_position(current_player);
        let enemy_attacks = self.get_attack_bitboard(current_player.other());
        (enemy_attacks >> king_pos) & 1 > 0
    }

    pub fn from_fen(fen: &str) -> Result<Self, String>
    where
        [(); C::AREA]: Sized,
    {
        let mut board = Self::new();
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 {
            return Err("Invalid FEN".to_string());
        }

        let ranks: Vec<&str> = parts[0].split('/').collect();
        if ranks.len() != C::HEIGHT {
            return Err("Incorrect number of ranks".to_string());
        }
        for (rank, row) in ranks.iter().enumerate() {
            let mut file = 0;
            let mut chars = row.chars().peekable();
            while let Some(ch) = chars.next() {
                if ch.is_ascii_digit() {
                    let mut count = ch.to_digit(10).unwrap() as usize;
                    while let Some(next) = chars.peek().copied() {
                        if !next.is_ascii_digit() {
                            break;
                        }
                        count = count * 10 + chars.next().unwrap().to_digit(10).unwrap() as usize;
                    }
                    file += count;
                    continue;
                }
                if file >= C::WIDTH {
                    return Err("Rank is too wide".to_string());
                }
                let is_white = ch.is_uppercase();
                let piece = match ch.to_ascii_lowercase() {
                    'p' => PieceType::Pawn(unsafe { std::mem::zeroed() }),
                    'r' => PieceType::Rook(unsafe { std::mem::zeroed() }),
                    'n' => PieceType::Knight(unsafe { std::mem::zeroed() }),
                    'b' => PieceType::Bishop(unsafe { std::mem::zeroed() }),
                    'q' => PieceType::Queen(unsafe { std::mem::zeroed() }),
                    'k' => PieceType::King(unsafe { std::mem::zeroed() }),
                    _ => return Err("Invalid char".to_string()),
                };

                let index = rank * C::WIDTH + file;
                board.pieces[index] = piece;
                let bit = 1_u128 << index;
                board.occupancy_board |= bit;

                if is_white {
                    board.white_pieces_bitboard |= bit;
                } else {
                    board.black_pieces_bitboard |= bit;
                }

                file += 1;
            }
            if file != C::WIDTH {
                return Err("Rank has incorrect width".to_string());
            }
        }

        board.turn = match parts[1] {
            "w" => CurrentPlayer::White,
            "b" => CurrentPlayer::Black,
            _ => return Err("Invalid color".to_string()),
        };

        board.castling_rights = 0;
        if parts[2] != "-" {
            for ch in parts[2].chars() {
                match ch {
                    'K' => board.castling_rights |= Self::castling_right_bits(CurrentPlayer::White, true),
                    'Q' => board.castling_rights |= Self::castling_right_bits(CurrentPlayer::White, false),
                    'k' => board.castling_rights |= Self::castling_right_bits(CurrentPlayer::Black, true),
                    'q' => board.castling_rights |= Self::castling_right_bits(CurrentPlayer::Black, false),
                    _ => {}
                }
            }
        }

        if parts[3] != "-" {
            let bytes = parts[3].as_bytes();
            if let Some((&file, rank)) = bytes.split_first() {
                let file = (file - b'a') as usize;
                let rank = std::str::from_utf8(rank).ok().and_then(|rank| rank.parse::<usize>().ok());
                if file < C::WIDTH && rank.is_some_and(|rank| (1..=C::HEIGHT).contains(&rank)) {
                    board.en_passant_square = Some(((C::HEIGHT - rank.unwrap()) * C::WIDTH + file) as u8);
                }
            }
        }
        if let Some(halfmove_clock) = parts.get(4) {
            board.halfmove_clock = halfmove_clock
                .parse()
                .map_err(|_| "Invalid halfmove clock".to_string())?;
        }
        board.zobrist = board.calculate_zobrist_hash();
        Ok(board)
    }

    pub fn get_king_position(&self, player: CurrentPlayer) -> u8 {
        let mut king_pos = 0;
        loop {
            match self.pieces[king_pos].clone() {
                PieceType::King(_) => {
                    if player == CurrentPlayer::White
                        && (self.white_pieces_bitboard & (1 << king_pos)) != 0
                    {
                        break;
                    } else if player == CurrentPlayer::Black
                        && (self.black_pieces_bitboard & (1 << king_pos)) != 0
                    {
                        break;
                    }
                }
                _ => (),
            }
            king_pos += 1;
            if king_pos >= C::AREA {
                eprintln!("This sequence turned into an invalid board position:");
                for i in self.move_history.iter() {
                    eprintln!(
                        "from {} to {}",
                        self.to_chess_notation(i.moved.from()),
                        self.to_chess_notation(i.moved.to())
                    );
                    println!("{:?}", i.moved.flag());
                    println!("Captured: {:?}", i.captured);
                }
                panic!("No king of color on the board")
            }
        }
        king_pos as u8
    }

    pub fn find_pins(&self, player: CurrentPlayer) -> u128 {
        let mut pinned_pieces = 0;

        let attack_vectors = crate::piece::get_all_possible_attack_vectors();
        let king_pos = self.get_king_position(player.clone());
        let current_player_bitboard = if player == CurrentPlayer::White {
            self.white_pieces_bitboard
        } else {
            self.black_pieces_bitboard
        };
        let opponent_player_bitboard = if player == CurrentPlayer::White {
            self.black_pieces_bitboard
        } else {
            self.white_pieces_bitboard
        };
        for attack in attack_vectors {
            let res = self.raycast(*attack, king_pos, C::AREA as u8);
            if res.is_none_or(|res| current_player_bitboard & (1 << res) == 0) {
                continue;
            }
            let res = res.unwrap();
            let pinner = self.raycast(*attack, res, C::AREA as u8);
            if pinner.is_none_or(|pinner| {
                opponent_player_bitboard & (1 << pinner) == 0
                    || !self.pieces[pinner as usize].has_opposite_vector(*attack)
            }) {
                continue;
            }
            pinned_pieces |= (1 << res) | (1 << pinner.unwrap());
        }
        pinned_pieces
    }

    pub fn get_pin_ray_mask(&self, king_sq: u8, pinned_sq: u8) -> u128 {
        let mut mask = 0;
        let k_file = (king_sq % C::WIDTH as u8) as i16;
        let k_rank = (king_sq / C::WIDTH as u8) as i16;
        let p_file = (pinned_sq % C::WIDTH as u8) as i16;
        let p_rank = (pinned_sq / C::WIDTH as u8) as i16;

        let d_file = (p_file - k_file).signum();
        let d_rank = (p_rank - k_rank).signum();
        let vector = d_rank * (C::WIDTH as i16) + d_file;

        let mut current = king_sq as i16;
        loop {
            current += vector;
            if current < 0 || current >= C::AREA as i16 {
                break;
            }
            mask |= 1_u128 << current;

            if (self.occupancy_board & (1_u128 << current)) != 0 && current != pinned_sq as i16 {
                break;
            }
        }
        mask
    }

    pub fn get_attack_bitboard_king_phase_through(&self, player: CurrentPlayer) -> u128 {
        let mut attack_bitboard: u128 = 0;

        let king_pos = self.get_king_position(self.current_player());
        let pieces_bitboard = if player == CurrentPlayer::White {
            self.white_pieces_bitboard ^ (1_u128 << king_pos)
        } else {
            self.black_pieces_bitboard ^ (1_u128 << king_pos)
        };

        for i in 0..C::AREA {
            if pieces_bitboard & (1 << i) != 0 {
                let piece = &self.pieces[i];
                let move_bitboard =
                    piece.get_attack_bitboard(i as u8, self.occupancy_board, player.clone());
                attack_bitboard |= move_bitboard;
            }
        }
        attack_bitboard
    }

    pub fn get_attack_bitboard(&self, player: CurrentPlayer) -> u128 {
        let mut attack_bitboard: u128 = 0;
        let pieces_bitboard = if player == CurrentPlayer::White {
            self.white_pieces_bitboard
        } else {
            self.black_pieces_bitboard
        };

        for i in 0..C::AREA {
            if pieces_bitboard & (1 << i) != 0 {
                let piece = &self.pieces[i];
                let move_bitboard =
                    piece.get_attack_bitboard(i as u8, self.occupancy_board, player.clone());
                attack_bitboard |= move_bitboard;
            }
        }
        attack_bitboard
    }

    pub fn raycast(&self, vector: i8, start: u8, max_len: u8) -> Option<u8> {
        let area = C::AREA as i16;
        let mut current = start as i16;
        let v = vector as i16;

        for _ in 0..max_len {
            let next = current + v;

            if next < 0 || next >= area {
                return None;
            }

            let current_file = current % C::WIDTH as i16;
            let next_file = next % C::WIDTH as i16;
            if (current_file - next_file).abs() > 1 {
                return None;
            }

            current = next;

            if (self.occupancy_board & (1_u128 << current as u8)) != 0 {
                return Some(current as u8);
            }
        }

        None
    }

    pub fn generate_moves(&mut self) -> Result<Vec<C::MoveType>, GameStatus> {
        let moves = if self.is_king_checked(self.turn) {
            self.generate_moves_out_of_check()
        } else {
            self.generate_moves_no_check(false)
        };
        if moves.is_empty() {
            if self.is_king_checked(self.turn) {
                Err(GameStatus::Won(self.current_player().other()))
            } else {
                Err(GameStatus::DrawStalemate)
            }
        } else if self.halfmove_clock >= 100 {
            Err(GameStatus::DrawStalemate)
        } else if !self.is_enough_material() {
            Err(GameStatus::DrawInsufficientMaterial)
        } else if self.is_threefold_repetition() {
            Err(GameStatus::DrawRepetition)
        } else {
            Ok(moves)
        }
    }

    fn is_threefold_repetition(&self) -> bool {
        self.move_history
            .iter()
            .filter(|a| a.zobrist == self.zobrist)
            .count()
            >= 2
    }

    fn is_enough_material(&self) -> bool {
        true //todo: implement this
    }

    fn generate_moves_no_check(&mut self, is_checked: bool) -> Vec<C::MoveType>
    where
        [(); C::AREA]: Sized,
    {
        let mut moves: Vec<C::MoveType> = Vec::new();
        let pieces_bitboard = if self.current_player() == CurrentPlayer::White {
            self.white_pieces_bitboard
        } else {
            self.black_pieces_bitboard
        };
        let enemy_pieces = self.occupancy_board & !pieces_bitboard;
        let pinned_pieces = self.find_pins(self.current_player());
        for i in 0..C::AREA {
            let piece_mask = pieces_bitboard & (1 << i);
            let is_pinned = (pinned_pieces & piece_mask) != 0;
            if piece_mask != 0 {
                let piece = &self.pieces[i];
                let mut move_bitboard =
                    (piece.get_move_bitboard(i as u8, self.occupancy_board, self.current_player())
                        | (piece.get_attack_bitboard(
                            i as u8,
                            self.occupancy_board,
                            self.current_player(),
                        ) & enemy_pieces))
                        & !pieces_bitboard;
                if is_pinned {
                    move_bitboard &= self
                        .get_pin_ray_mask(self.get_king_position(self.current_player()), i as u8);
                }
                //todo: this rule could be applied better
                if let PieceType::King(_) = piece {
                    let enemy_attacks = self.get_attack_bitboard(self.current_player().other());
                    move_bitboard &= !enemy_attacks;
                }

                for to in 0..C::AREA {
                    if move_bitboard & (1 << to) != 0 {
                        let is_capture = (enemy_pieces & (1_u128 << to)) != 0;
                        let flag = if is_capture {
                            MoveFlag::Capture
                        } else if matches!(piece, PieceType::Pawn(_))
                            && i16::abs(to as i16 - i as i16) == 2 * C::WIDTH as i16
                        {
                            MoveFlag::DoublePawnPush
                        } else {
                            MoveFlag::Quiet
                        };
                        let mut is_promotion = false;
                        if let PieceType::Pawn(_) = piece.clone() {
                            if (self.current_player() == CurrentPlayer::Black
                                && to / C::WIDTH == C::HEIGHT - 1)
                                || (self.current_player() == CurrentPlayer::White && to < C::WIDTH)
                            {
                                is_promotion = true;
                            }
                        }

                        if is_promotion {
                            moves.push(C::MoveType::new(
                                i as u8,
                                to as u8,
                                Promotion,
                                PieceType::Queen(Queen::default()),
                            ));
                            moves.push(C::MoveType::new(
                                i as u8,
                                to as u8,
                                Promotion,
                                PieceType::Rook(Rook::default()),
                            ));
                            moves.push(C::MoveType::new(
                                i as u8,
                                to as u8,
                                Promotion,
                                PieceType::Bishop(Bishop::default()),
                            ));
                            moves.push(C::MoveType::new(
                                i as u8,
                                to as u8,
                                Promotion,
                                PieceType::Knight(Knight::default()),
                            ));
                        } else {
                            moves.push(C::MoveType::new(i as u8, to as u8, flag, PieceType::None));
                        }
                    }
                }

                if let PieceType::Pawn(_) = piece {
                    if let Some(ep_sq) = self.en_passant_square {
                        let mut pawn_attacks = piece.get_attack_bitboard(
                            i as u8,
                            self.occupancy_board,
                            self.current_player(),
                        );
                        if (pawn_attacks & (1_u128 << ep_sq)) != 0 {
                            let mov = C::MoveType::new(
                                i as u8,
                                ep_sq,
                                MoveFlag::EnPassant,
                                PieceType::None,
                            );

                            self.apply_move(mov);
                            if !self.is_king_checked(self.turn.other()) {
                                moves.push(mov);
                            }
                            self.undo_move();
                        }
                    }
                }
            }
        }
        if (is_checked) {
            return moves;
        }
        match self.current_player() {
            CurrentPlayer::Black => {
                self.generate_castling_for_rank(&mut moves, 0, self.current_player())
            }
            CurrentPlayer::White => {
                self.generate_castling_for_rank(&mut moves, (C::HEIGHT - 1) * C::WIDTH, self.current_player())
            }
        }
        moves
    }

    fn is_square_attacked(&self, square: u8, attacker_color: CurrentPlayer) -> bool {
        let sq_bit = 1_u128 << square;
        (self.get_attack_bitboard(attacker_color) & sq_bit) > 0
    }
    fn generate_castling_for_rank(
        &self,
        moves: &mut Vec<C::MoveType>,
        offset: usize,
        player: CurrentPlayer,
    ) {
        let king = offset + 4;
        let kingside_rook = offset + C::WIDTH - 1;
        let queenside_rook = offset;
        let king_to_kingside = king + 2;
        let king_to_queenside = king - 2;
        let mut ks_empty = 0;
        for square in king + 1..kingside_rook {
            ks_empty |= 1_u128 << square;
        }
        let ks_rights = (1_u128 << king) | (1_u128 << kingside_rook);

        let has_rights = (self.castling_rights & ks_rights) == ks_rights;
        let is_empty = (self.occupancy_board & ks_empty) == 0;
        let e_safe = !self.is_square_attacked(king as u8, player.other());
        let f_safe = !self.is_square_attacked((king + 1) as u8, player.other());
        let g_safe = !self.is_square_attacked(king_to_kingside as u8, player.other());

        if has_rights && is_empty && e_safe && f_safe && g_safe {
            moves.push(C::MoveType::new(
                king as u8,
                king_to_kingside as u8,
                MoveFlag::Castling,
                PieceType::None,
            ));
        }

        let mut qs_empty = 0;
        for square in queenside_rook + 1..king {
            qs_empty |= 1_u128 << square;
        }
        let qs_rights = (1_u128 << king) | (1_u128 << queenside_rook);

        if (self.castling_rights & qs_rights) == qs_rights
            && (self.occupancy_board & qs_empty) == 0
            && !self.is_square_attacked(king as u8, player.other())
            && !self.is_square_attacked((king - 1) as u8, player.other())
            && !self.is_square_attacked(king_to_queenside as u8, player.other())
        {
            moves.push(C::MoveType::new(
                king as u8,
                king_to_queenside as u8,
                MoveFlag::Castling,
                PieceType::None,
            ));
        }
    }

    fn generate_moves_out_of_check(&mut self) -> Vec<C::MoveType> {
        //IMPROVE: this should only run for king when knight is checking
        // (or double checks)
        // *or just move to an algorithmic solution
        let semilegal = self.generate_moves_no_check(true);
        let mut legal = Vec::new();
        let current_player = self.turn;
        for m in semilegal {
            self.apply_move(m);
            let king = self.get_king_position(current_player);
            let attacks = self.get_attack_bitboard(current_player.other());
            if (attacks >> king) & 1 == 0 {
                legal.push(m);
            }
            self.undo_move();
        }
        legal
    }

    pub fn get_check_block_squares(&self, player: CurrentPlayer) -> u128 {
        //very wrong i think
        let mut block_bitboard = 0;
        let attack_vectors = crate::piece::get_all_possible_attack_vectors();
        let king_pos = self.get_king_position(self.current_player());
        let current_player_bitboard = if player == CurrentPlayer::White {
            self.white_pieces_bitboard
        } else {
            self.black_pieces_bitboard
        };
        let opponent_player_bitboard = if player == CurrentPlayer::White {
            self.black_pieces_bitboard
        } else {
            self.white_pieces_bitboard
        };
        for vector in attack_vectors {
            let mut maybe_block_bits = 0;
            let mut pos = Some(king_pos);
            loop {
                if (pos.unwrap() as i16 + *vector as i16) < 0
                    || (pos.unwrap() as i16 + *vector as i16) as usize >= C::AREA
                {
                    break;
                }
                pos = (pos.unwrap() as i16 + *vector as i16).try_into().ok();
                if (self.occupancy_board & (1 << pos.unwrap())) != 0 {
                    if self.pieces[pos.unwrap() as usize].has_opposite_vector(*vector) {
                        block_bitboard |= maybe_block_bits;
                    }
                } else {
                    maybe_block_bits |= 1 << pos.unwrap();
                }
            }
        }
        block_bitboard
    }

    pub fn apply_move_checked(&mut self, move_: C::MoveType) -> MoveLegality {
        match self.is_move_legal(move_) {
            MoveLegality::Legal => {
                self.apply_move(move_);
                MoveLegality::Legal
            }
            legality => legality,
        }
    }

    pub fn apply_move(&mut self, move_: C::MoveType) -> Result<(), ()> {
        let last_castling_rights = self.castling_rights;
        let last_halfmove_clock = self.halfmove_clock;
        let last_en_passant_square = self.en_passant_square;
        let last_zobrist = self.zobrist;
        let moved = self.pieces[move_.from() as usize].clone();
        let board = &mut self.pieces;
        if !matches!(
            move_.flag(),
            MoveFlag::Quiet | MoveFlag::Capture | MoveFlag::DoublePawnPush | MoveFlag::Promotion
        ) {
            let captured = self.handle_special_move(move_)?;
            self.update_zobrist_after_move(
                move_,
                &moved,
                &captured,
                last_castling_rights,
                last_en_passant_square,
            );
            self.move_history.push(MoveHistoryData {
                moved: move_,
                last_castling_data: last_castling_rights,
                captured,
                en_passant: last_en_passant_square,
                halfmove_clock: last_halfmove_clock,
                zobrist: last_zobrist,
            });
            self.verify_board_state("After apply_move");
            return Ok(());
        }
        let captured = board[move_.to() as usize].clone();
        if matches!(board[move_.from() as usize], (PieceType::Pawn(_)))
            && i16::abs(move_.to() as i16 - move_.from() as i16) == 2 * C::WIDTH as i16
        {
            self.en_passant_square = if self.turn == CurrentPlayer::White {
                Some(move_.from() - C::WIDTH as u8)
            } else {
                Some(move_.from() + C::WIDTH as u8)
            };
        } else {
            self.en_passant_square = None;
        }

        board[move_.to() as usize] = board[move_.from() as usize].clone();
        board[move_.from() as usize] = PieceType::None;

        let moved_bit = (1 << move_.from());
        let changed_bit = (1 << move_.to());

        self.occupancy_board = self.occupancy_board ^ moved_bit;
        self.occupancy_board = self.occupancy_board | changed_bit;

        self.castling_rights &= !(1_u128 << move_.from());
        self.castling_rights &= !(1_u128 << move_.to());

        if self.turn == CurrentPlayer::White {
            self.white_pieces_bitboard = self.white_pieces_bitboard & !moved_bit;
            self.white_pieces_bitboard = self.white_pieces_bitboard | changed_bit;
            self.black_pieces_bitboard = self.black_pieces_bitboard & !changed_bit;
            self.turn = CurrentPlayer::Black;
        } else {
            self.black_pieces_bitboard = self.black_pieces_bitboard & !moved_bit;
            self.black_pieces_bitboard = self.black_pieces_bitboard | changed_bit;
            self.white_pieces_bitboard = self.white_pieces_bitboard & !changed_bit;
            self.turn = CurrentPlayer::White;
        }
        if captured != PieceType::None || matches!(board[move_.to() as usize], PieceType::Pawn(_)) {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock = self.halfmove_clock + 1;
        }
        if move_.flag() == MoveFlag::Promotion {
            board[move_.to() as usize] = move_.promotion();
        }

        self.update_zobrist_after_move(
            move_,
            &moved,
            &captured,
            last_castling_rights,
            last_en_passant_square,
        );
        self.move_history.push(MoveHistoryData {
            moved: move_,
            last_castling_data: last_castling_rights,
            captured,
            en_passant: last_en_passant_square,
            halfmove_clock: last_halfmove_clock,
            zobrist: last_zobrist,
        });
        self.verify_board_state("After apply_move");
        Ok(())
    }
    pub fn handle_special_move(&mut self, move_: C::MoveType) -> Result<PieceType<C>, ()> {
        match move_.flag() {
            MoveFlag::EnPassant => {
                let captured_sq = if self.turn == CurrentPlayer::White {
                    move_.to() + C::WIDTH as u8
                } else {
                    move_.to() - C::WIDTH as u8
                };

                let captured_piece = self.pieces[captured_sq as usize].clone();

                self.pieces[move_.to() as usize] = self.pieces[move_.from() as usize].clone();
                self.pieces[move_.from() as usize] = PieceType::None;
                self.pieces[captured_sq as usize] = PieceType::None;

                let moved_bit = 1_u128 << move_.from();
                let changed_bit = 1_u128 << move_.to();
                let captured_bit = 1_u128 << captured_sq;

                self.occupancy_board &= !moved_bit;
                self.occupancy_board |= changed_bit;
                self.occupancy_board &= !captured_bit;

                if self.turn == CurrentPlayer::White {
                    self.white_pieces_bitboard &= !moved_bit;
                    self.white_pieces_bitboard |= changed_bit;
                    self.black_pieces_bitboard &= !captured_bit;
                    self.turn = CurrentPlayer::Black;
                } else {
                    self.black_pieces_bitboard &= !moved_bit;
                    self.black_pieces_bitboard |= changed_bit;
                    self.white_pieces_bitboard &= !captured_bit;
                    self.turn = CurrentPlayer::White;
                }

                self.castling_rights &= !moved_bit;
                self.castling_rights &= !changed_bit;
                self.halfmove_clock = 0;
                self.en_passant_square = None;

                Ok(captured_piece)
            }

            MoveFlag::Castling => {
                self.pieces[move_.to() as usize] = self.pieces[move_.from() as usize].clone();
                self.pieces[move_.from() as usize] = PieceType::None;

                let is_kingside = move_.to() > move_.from();
                let rank_start = (move_.from() as usize / C::WIDTH) * C::WIDTH;
                let rook_from = if is_kingside {
                    rank_start + C::WIDTH - 1
                } else {
                    rank_start
                } as u8;
                let rook_to = if is_kingside {
                    move_.from() + 1
                } else {
                    move_.from() - 1
                };

                self.pieces[rook_to as usize] = self.pieces[rook_from as usize].clone();
                self.pieces[rook_from as usize] = PieceType::None;

                let k_from_bit = 1_u128 << move_.from();
                let k_to_bit = 1_u128 << move_.to();
                let r_from_bit = 1_u128 << rook_from;
                let r_to_bit = 1_u128 << rook_to;

                let clear_mask = !(k_from_bit | r_from_bit);
                let set_mask = k_to_bit | r_to_bit;

                self.occupancy_board &= clear_mask;
                self.occupancy_board |= set_mask;

                if self.turn == CurrentPlayer::White {
                    self.white_pieces_bitboard &= clear_mask;
                    self.white_pieces_bitboard |= set_mask;
                    self.turn = CurrentPlayer::Black;
                } else {
                    self.black_pieces_bitboard &= clear_mask;
                    self.black_pieces_bitboard |= set_mask;
                    self.turn = CurrentPlayer::White;
                }

                let player = self.turn.other();
                let rights_to_clear = Self::castling_right_bits(player, true)
                    | Self::castling_right_bits(player, false);
                self.castling_rights &= !rights_to_clear;

                self.halfmove_clock += 1;
                self.en_passant_square = None;

                Ok(PieceType::None)
            }
            _ => Err(()),
        }
    }

    pub fn last_move(&self) -> Option<&MoveHistoryData<C>> {
        self.move_history.last()
    }

    pub fn undo_move(&mut self) -> C::MoveType {
        let last_move = self
            .move_history
            .pop()
            .expect("No move history left!");
        if !matches!(
            last_move.moved.flag(),
            MoveFlag::Capture | MoveFlag::Quiet | MoveFlag::DoublePawnPush | MoveFlag::Promotion
        ) {
            let _ = self.undo_special_move(&last_move);
            self.castling_rights = last_move.last_castling_data;
            self.halfmove_clock = last_move.halfmove_clock;
            self.en_passant_square = last_move.en_passant;
            self.zobrist = last_move.zobrist;
            self.verify_board_state("After undo_move");
            return last_move.moved.clone();
        }
        let from_bit = 1 << last_move.moved.from();
        let to_bit = 1 << last_move.moved.to();

        self.pieces[last_move.moved.from() as usize] =
            self.pieces[last_move.moved.to() as usize].clone();
        self.pieces[last_move.moved.to() as usize] = last_move.captured.clone();
        if self.turn == CurrentPlayer::Black {
            self.white_pieces_bitboard |= from_bit;

            self.white_pieces_bitboard &= !to_bit;
            self.black_pieces_bitboard &= !to_bit;
            if last_move.captured != PieceType::None {
                self.black_pieces_bitboard |= to_bit;
            }

            self.turn = CurrentPlayer::White;
        } else {
            self.black_pieces_bitboard |= from_bit;
            self.black_pieces_bitboard &= !to_bit;
            self.white_pieces_bitboard &= !to_bit;

            if last_move.captured != PieceType::None {
                self.white_pieces_bitboard |= to_bit;
            }

            self.turn = CurrentPlayer::Black;
        }

        self.occupancy_board |= from_bit;
        if last_move.captured == PieceType::None {
            self.occupancy_board &= !to_bit;
        } else {
            self.occupancy_board |= to_bit;
        }
        if last_move.moved.flag() == MoveFlag::Promotion {
            self.pieces[last_move.moved.from() as usize] = Pawn(pawn::Pawn::default());
        }
        self.castling_rights = last_move.last_castling_data;
        self.halfmove_clock = last_move.halfmove_clock;
        self.en_passant_square = last_move.en_passant;
        self.zobrist = last_move.zobrist;
        self.verify_board_state("After undo_move");
        last_move.moved
    }

    pub fn undo_special_move(&mut self, last_move: &MoveHistoryData<C>) -> Result<(), ()> {
        let from = last_move.moved.from();
        let to = last_move.moved.to();

        let from_bit = 1_u128 << from;
        let to_bit = 1_u128 << to;

        match last_move.moved.flag() {
            MoveFlag::EnPassant => {
                let actual_captured_sq = if self.turn == CurrentPlayer::White {
                    to - C::WIDTH as u8
                } else {
                    to + C::WIDTH as u8
                };
                let captured_bit = 1_u128 << actual_captured_sq;

                self.pieces[from as usize] = self.pieces[to as usize].clone();
                self.pieces[to as usize] = PieceType::None;
                self.pieces[actual_captured_sq as usize] = last_move.captured.clone();

                if self.turn == CurrentPlayer::Black {
                    self.white_pieces_bitboard |= from_bit;
                    self.white_pieces_bitboard &= !to_bit;
                    self.black_pieces_bitboard |= captured_bit;
                    self.turn = CurrentPlayer::White;
                } else {
                    self.black_pieces_bitboard |= from_bit;
                    self.black_pieces_bitboard &= !to_bit;
                    self.white_pieces_bitboard |= captured_bit;
                    self.turn = CurrentPlayer::Black;
                }

                self.occupancy_board |= from_bit;
                self.occupancy_board &= !to_bit;
                self.occupancy_board |= captured_bit;

                Ok(())
            }
            MoveFlag::Castling => {
                self.pieces[from as usize] = self.pieces[to as usize].clone();
                self.pieces[to as usize] = PieceType::None;

                let is_kingside = to > from;
                let rank_start = (from as usize / C::WIDTH) * C::WIDTH;
                let rook_from = if is_kingside {
                    rank_start + C::WIDTH - 1
                } else {
                    rank_start
                } as u8;
                let rook_to = if is_kingside { from + 1 } else { from - 1 };

                self.pieces[rook_from as usize] = self.pieces[rook_to as usize].clone();
                self.pieces[rook_to as usize] = PieceType::None;

                let r_from_bit = 1_u128 << rook_from;
                let r_to_bit = 1_u128 << rook_to;

                let clear_mask = to_bit | r_to_bit;
                let set_mask = from_bit | r_from_bit;

                self.occupancy_board &= !clear_mask;
                self.occupancy_board |= set_mask;

                if self.turn == CurrentPlayer::Black {
                    self.white_pieces_bitboard &= !clear_mask;
                    self.white_pieces_bitboard |= set_mask;
                    self.turn = CurrentPlayer::White;
                } else {
                    self.black_pieces_bitboard &= !clear_mask;
                    self.black_pieces_bitboard |= set_mask;
                    self.turn = CurrentPlayer::Black;
                }

                Ok(())
            }
            _ => Err(()),
        }
    }

    pub fn get_pieces(&self) -> [Piece<C>; C::AREA] {
        self.pieces
            .iter()
            .enumerate()
            .map(|(iter, ptype)| {
                if self.white_pieces_bitboard & (1 << iter) != 0 {
                    Piece::White(ptype.clone())
                } else if self.black_pieces_bitboard & (1 << iter) != 0 {
                    Piece::Black(ptype.clone())
                } else {
                    Piece::None
                }
            })
            .collect::<Vec<Piece<C>>>()
            .try_into()
            .unwrap()
    }

    pub fn to_chess_notation(&self, pos: u8) -> String {
        let file = ((pos % C::WIDTH as u8) + b'a') as char;
        let rank = (pos / C::WIDTH as u8) + 1;
        format!("{}{}", file, rank)
    }

    pub fn debug_print_data(&self) {
        println!("Occupancy Board: {:064b}", self.occupancy_board);
        println!("White Pieces Bitboard: {:064b}", self.white_pieces_bitboard);
        println!("Black Pieces Bitboard: {:064b}", self.black_pieces_bitboard);
        println!("White Pins: {:064b}", self.find_pins(CurrentPlayer::White));
        println!("Black Pins: {:064b}", self.find_pins(CurrentPlayer::Black));
        println!("Castling Data: {:?}", self.castling_rights);
    }

    pub fn current_player(&self) -> CurrentPlayer {
        self.turn.clone()
    }

    pub fn castling_rights(&self) -> u128 {
        self.castling_rights
    }

    pub fn en_passant_square(&self) -> Option<u8> {
        self.en_passant_square
    }

    pub fn halfmove_clock(&self) -> u8 {
        self.halfmove_clock
    }

    pub fn repetition_count(&self) -> usize {
        1 + self
            .move_history
            .iter()
            .filter(|entry| entry.zobrist == self.zobrist)
            .count()
    }

    pub fn piece_at(&self, square: usize) -> Piece<C> {
        assert!(square < C::AREA, "Square index is outside the board");
        let piece = self.pieces[square].clone();
        if self.white_pieces_bitboard & (1_u128 << square) != 0 {
            Piece::White(piece)
        } else if self.black_pieces_bitboard & (1_u128 << square) != 0 {
            Piece::Black(piece)
        } else {
            Piece::None
        }
    }

    pub fn zobrist_hash(&self) -> u64 {
        self.zobrist
    }

    fn update_zobrist_after_move(
        &mut self,
        move_: C::MoveType,
        moved: &PieceType<C>,
        captured: &PieceType<C>,
        old_castling_rights: u128,
        old_en_passant_square: Option<u8>,
    ) {
        let mover = self.turn.other();
        self.zobrist ^= Self::zobrist_key(0xBB67_AE85_84CA_A73B);
        self.zobrist ^= Self::piece_zobrist_key(moved, mover, move_.from());
        self.zobrist ^=
            Self::piece_zobrist_key(&self.pieces[move_.to() as usize], mover, move_.to());

        if let Some(captured_square) = match move_.flag() {
            MoveFlag::EnPassant => Some(if mover == CurrentPlayer::White {
                move_.to() + C::WIDTH as u8
            } else {
                move_.to() - C::WIDTH as u8
            }),
            _ if !matches!(captured, PieceType::None) => Some(move_.to()),
            _ => None,
        } {
            self.zobrist ^= Self::piece_zobrist_key(captured, mover.other(), captured_square);
        }

        if move_.flag() == MoveFlag::Castling {
            let is_kingside = move_.to() > move_.from();
            let rank_start = (move_.from() as usize / C::WIDTH) * C::WIDTH;
            let rook_from = if is_kingside {
                rank_start + C::WIDTH - 1
            } else {
                rank_start
            } as u8;
            let rook_to = if is_kingside { move_.from() + 1 } else { move_.from() - 1 };
            let rook = &self.pieces[rook_to as usize];
            self.zobrist ^= Self::piece_zobrist_key(rook, mover, rook_from);
            self.zobrist ^= Self::piece_zobrist_key(rook, mover, rook_to);
        }

        let mut changed_castling_rights = old_castling_rights ^ self.castling_rights;
        while changed_castling_rights != 0 {
            let square = changed_castling_rights.trailing_zeros() as u8;
            self.zobrist ^= Self::zobrist_key(0x3C6E_F372_FE94_F82B ^ square as u64);
            changed_castling_rights &= changed_castling_rights - 1;
        }
        if let Some(square) = old_en_passant_square {
            self.zobrist ^= Self::zobrist_key(0xA54F_F53A_5F1D_36F1 ^ square as u64);
        }
        if let Some(square) = self.en_passant_square {
            self.zobrist ^= Self::zobrist_key(0xA54F_F53A_5F1D_36F1 ^ square as u64);
        }
    }

    fn calculate_zobrist_hash(&self) -> u64 {
        const SIDE_SEED: u64 = 0xBB67_AE85_84CA_A73B;
        const CASTLING_SEED: u64 = 0x3C6E_F372_FE94_F82B;
        const EN_PASSANT_SEED: u64 = 0xA54F_F53A_5F1D_36F1;

        let mut hash = 0;
        for square in 0..C::AREA {
            let color = if self.white_pieces_bitboard & (1_u128 << square) != 0 {
                Some(CurrentPlayer::White)
            } else if self.black_pieces_bitboard & (1_u128 << square) != 0 {
                Some(CurrentPlayer::Black)
            } else {
                None
            };

            if let Some(color) = color {
                hash ^= Self::piece_zobrist_key(&self.pieces[square], color, square as u8);
            }
        }

        if self.turn == CurrentPlayer::Black {
            hash ^= Self::zobrist_key(SIDE_SEED);
        }
        for square in 0..C::AREA {
            if self.castling_rights & (1_u128 << square) != 0 {
                hash ^= Self::zobrist_key(CASTLING_SEED ^ square as u64);
            }
        }
        if let Some(square) = self.en_passant_square {
            hash ^= Self::zobrist_key(EN_PASSANT_SEED ^ square as u64);
        }
        hash
    }

    fn piece_index(piece: &PieceType<C>) -> Option<u64> {
        match piece {
            PieceType::Pawn(_) => Some(0),
            PieceType::Rook(_) => Some(1),
            PieceType::Knight(_) => Some(2),
            PieceType::Bishop(_) => Some(3),
            PieceType::Queen(_) => Some(4),
            PieceType::King(_) => Some(5),
            PieceType::None => None,
        }
    }

    fn piece_zobrist_key(piece: &PieceType<C>, color: CurrentPlayer, square: u8) -> u64 {
        let Some(piece) = Self::piece_index(piece) else {
            return 0;
        };
        let color = match color {
            CurrentPlayer::White => 0,
            CurrentPlayer::Black => 1,
        };
        Self::zobrist_key(0x6A09_E667_F3BC_C909 ^ ((color * 6 + piece) * 128 + square as u64))
    }

    fn zobrist_key(mut value: u64) -> u64 {
        value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    pub fn verify_board_state(&self, context: &str) {
        if !cfg!(debug_assertions) {
            return;
        }
        for i in 0..64 {
            let has_piece = !matches!(self.pieces[i], PieceType::None);

            let in_white_bb = (self.white_pieces_bitboard & (1_u128 << i)) != 0;
            let in_black_bb = (self.black_pieces_bitboard & (1_u128 << i)) != 0;
            let in_occ = (self.occupancy_board & (1_u128 << i)) != 0;

            let bitboard_has_piece = in_white_bb || in_black_bb;

            if has_piece != bitboard_has_piece
                || has_piece != in_occ
                || (in_white_bb && in_black_bb)
            {
                println!("\n========================================");
                println!("CRITICAL STATE DESYNC DETECTED!");
                println!("Context: {}", context);
                println!("Square Index: {} ({})", i, self.to_chess_notation(i as u8));
                println!("Has physical piece in array: {}", has_piece);
                if has_piece {
                    println!("Piece type: {:?}", self.pieces[i]);
                }
                println!("Present in White Bitboard: {}", in_white_bb);
                println!("Present in Black Bitboard: {}", in_black_bb);
                println!("Present in Occupancy BB: {}", in_occ);
                println!("========================================\n");

                Self::display_board(&self.get_pieces(), C::WIDTH as usize);
                panic!("Engine state corrupted!");
            }
        }
        assert_eq!(
            self.zobrist,
            self.calculate_zobrist_hash(),
            "Zobrist hash desync detected: {context}"
        );
    }

    pub fn display_board(pieces: &[Piece<C>], width: usize) {
        let height = pieces.len() / width;

        println!("  +{}+", "-".repeat(width * 2));

        for row in 0..height {
            print!("{} |", height - row);

            for col in 0..width {
                let index = row * width + col;
                let piece = pieces[index].clone();
                print!("{}", Chessboard::<C>::format_piece(piece));
            }

            println!("|");
        }

        println!("  +{}+", "-".repeat(width * 2));

        print!("   ");
        for col in 0..width {
            print!("{} ", (('a' as u8 + col as u8) as char));
        }
        println!();
    }

    fn format_piece(piece: Piece<C>) -> String {
        match piece {
            Piece::White(ptype) => {
                let c = ptype.to_notation().to_uppercase().next().unwrap();
                format!("{}{}", c, c)
            }
            Piece::Black(ptype) => {
                let c = ptype.to_notation();
                format!("{}{}", c, c)
            }
            Piece::None => "  ".to_string(),
        }
    }
}
