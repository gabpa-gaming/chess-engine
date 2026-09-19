use std::error::Error;
use std::fmt;

use crate::board_config::{BoardConfig, RegularVariant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Uci,
    Debug(bool),
    IsReady,
    SetOption { name: String, value: Option<String> },
    UciNewGame,
    Position(Position),
    Go(SearchLimits),
    Stop,
    PonderHit,
    Quit,
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub board: PositionBoard,
    pub moves: Vec<UciMove>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PositionBoard {
    StartPos,
    Fen(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UciMove {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<PromotionPiece>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Square(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromotionPiece {
    Queen,
    Rook,
    Bishop,
    Knight,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchLimits {
    pub search_moves: Vec<UciMove>,
    pub ponder: bool,
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
    pub moves_to_go: Option<u32>,
    pub depth: Option<u32>,
    pub nodes: Option<u64>,
    pub mate: Option<u32>,
    pub move_time: Option<u64>,
    pub infinite: bool,
    pub go_variant: GoVariant,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum GoVariant {
    #[default]
    Regular,
    Perft,
    Weights,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    MissingArgument(&'static str),
    InvalidArgument {
        argument: String,
        context: &'static str,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingArgument(context) => write!(f, "missing argument for {context}"),
            Self::InvalidArgument { argument, context } => {
                write!(f, "invalid argument '{argument}' for {context}")
            }
        }
    }
}

impl Error for ParseError {}

pub fn parse(input: &str) -> Result<Option<Command>, ParseError> {
    parse_for::<RegularVariant>(input)
}

pub fn parse_for<C: BoardConfig>(input: &str) -> Result<Option<Command>, ParseError> {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let Some((&command, arguments)) = tokens.split_first() else {
        return Ok(None);
    };

    match command {
        "uci" => Ok(Some(Command::Uci)),
        "debug" => parse_debug(arguments).map(Command::Debug).map(Some),
        "isready" => Ok(Some(Command::IsReady)),
        "setoption" => parse_setoption(arguments).map(Some),
        "ucinewgame" => Ok(Some(Command::UciNewGame)),
        "position" => parse_position::<C>(arguments)
            .map(Command::Position)
            .map(Some),
        "go" => parse_go::<C>(arguments).map(Command::Go).map(Some),
        "stop" => Ok(Some(Command::Stop)),
        "ponderhit" => Ok(Some(Command::PonderHit)),
        "quit" => Ok(Some(Command::Quit)),
        _ => Ok(Some(Command::Unknown(command.to_owned()))),
    }
}

fn parse_debug(arguments: &[&str]) -> Result<bool, ParseError> {
    match arguments.first() {
        Some(&"on") => Ok(true),
        Some(&"off") => Ok(false),
        Some(argument) => invalid(argument, "debug"),
        None => Err(ParseError::MissingArgument("debug")),
    }
}

fn parse_setoption(arguments: &[&str]) -> Result<Command, ParseError> {
    if arguments.first() != Some(&"name") {
        return Err(ParseError::MissingArgument("setoption name"));
    }

    let value_index = arguments.iter().position(|token| *token == "value");
    let name_end = value_index.unwrap_or(arguments.len());
    let name = arguments[1..name_end].join(" ");
    if name.is_empty() {
        return Err(ParseError::MissingArgument("setoption name"));
    }

    let value = value_index.map(|index| arguments[index + 1..].join(" "));
    Ok(Command::SetOption { name, value })
}

fn parse_position<C: BoardConfig>(arguments: &[&str]) -> Result<Position, ParseError> {
    let Some((&kind, remaining)) = arguments.split_first() else {
        return Err(ParseError::MissingArgument("position"));
    };

    let (board, remaining) = match kind {
        "startpos" => (PositionBoard::StartPos, remaining),
        "fen" => {
            if remaining.len() < 6 {
                return Err(ParseError::MissingArgument("position fen"));
            }
            (
                PositionBoard::Fen(remaining[..6].join(" ")),
                &remaining[6..],
            )
        }
        argument => return invalid(argument, "position"),
    };

    let move_tokens = match remaining.split_first() {
        None => &[][..],
        Some((&"moves", moves)) => moves,
        Some((argument, _)) => return invalid(argument, "position"),
    };

    let moves = move_tokens
        .iter()
        .map(|move_| UciMove::parse_for::<C>(move_))
        .collect::<Result<_, _>>()?;
    Ok(Position { board, moves })
}

fn parse_go<C: BoardConfig>(arguments: &[&str]) -> Result<SearchLimits, ParseError> {
    let mut limits = SearchLimits::default();
    let mut index = 0;

    while let Some(&keyword) = arguments.get(index) {
        index += 1;
        match keyword {
            "searchmoves" => {
                let start = index;
                while let Some(&move_) = arguments.get(index) {
                    if is_go_keyword(move_) {
                        break;
                    }
                    limits.search_moves.push(UciMove::parse_for::<C>(move_)?);
                    index += 1;
                }
                if index == start {
                    return Err(ParseError::MissingArgument("go searchmoves"));
                }
            }
            "ponder" => limits.ponder = true,
            "infinite" => limits.infinite = true,
            "wtime" => limits.wtime = Some(parse_number(arguments.get(index), "go wtime")?),
            "btime" => limits.btime = Some(parse_number(arguments.get(index), "go btime")?),
            "winc" => limits.winc = Some(parse_number(arguments.get(index), "go winc")?),
            "binc" => limits.binc = Some(parse_number(arguments.get(index), "go binc")?),
            "movestogo" => {
                limits.moves_to_go = Some(parse_number(arguments.get(index), "go movestogo")?)
            }
            "depth" => limits.depth = Some(parse_number(arguments.get(index), "go depth")?),
            "nodes" => limits.nodes = Some(parse_number(arguments.get(index), "go nodes")?),
            "mate" => limits.mate = Some(parse_number(arguments.get(index), "go mate")?),
            "movetime" => {
                limits.move_time = Some(parse_number(arguments.get(index), "go movetime")?)
            }
            "weights" => limits.go_variant = GoVariant::Weights,
            argument => return invalid(argument, "go"),
        }

        if requires_value(keyword) {
            index += 1;
        }
    }
    Ok(limits)
}

impl UciMove {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Self::parse_for::<RegularVariant>(input)
    }

    pub fn parse_for<C: BoardConfig>(input: &str) -> Result<Self, ParseError> {
        let bytes = input.as_bytes();
        if !input.is_ascii() {
            return invalid(input, "UCI move");
        }

        let (from, from_end) = Square::parse_prefix::<C>(bytes, input)?;
        let (to, to_end) = Square::parse_prefix::<C>(&bytes[from_end..], input)?;
        let promotion = match bytes.get(from_end + to_end) {
            Some(b'q') => Some(PromotionPiece::Queen),
            Some(b'r') => Some(PromotionPiece::Rook),
            Some(b'b') => Some(PromotionPiece::Bishop),
            Some(b'n') => Some(PromotionPiece::Knight),
            Some(_) => return invalid(input, "UCI move"),
            None => None,
        };

        if from_end + to_end + usize::from(promotion.is_some()) != bytes.len() {
            return invalid(input, "UCI move");
        }

        Ok(Self {
            from,
            to,
            promotion,
        })
    }
}

impl Square {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Self::parse_for::<RegularVariant>(input)
    }

    pub fn parse_for<C: BoardConfig>(input: &str) -> Result<Self, ParseError> {
        let bytes = input.as_bytes();
        let (square, consumed) = Self::parse_prefix::<C>(bytes, input)?;
        if consumed != bytes.len() {
            return invalid(input, "square");
        }
        Ok(square)
    }

    fn parse_prefix<C: BoardConfig>(
        bytes: &[u8],
        input: &str,
    ) -> Result<(Self, usize), ParseError> {
        let Some((&file, rank_bytes)) = bytes.split_first() else {
            return invalid(input, "square");
        };
        if !(b'a'..=b'z').contains(&file) || C::WIDTH > 26 || C::AREA > u8::MAX as usize + 1 {
            return invalid(input, "square");
        }

        let rank_end = rank_bytes
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if rank_end == 0 {
            return invalid(input, "square");
        }

        let rank = std::str::from_utf8(&rank_bytes[..rank_end])
            .ok()
            .and_then(|rank| rank.parse::<usize>().ok());
        let Some(rank) = rank else {
            return invalid(input, "square");
        };
        let file = (file - b'a') as usize;
        if file >= C::WIDTH || rank == 0 || rank > C::HEIGHT {
            return invalid(input, "square");
        }

        Ok((
            Self(((C::HEIGHT - rank) * C::WIDTH + file) as u8),
            rank_end + 1,
        ))
    }
}

fn parse_number<T: std::str::FromStr>(
    argument: Option<&&str>,
    context: &'static str,
) -> Result<T, ParseError> {
    let Some(argument) = argument else {
        return Err(ParseError::MissingArgument(context));
    };
    argument.parse().map_err(|_| ParseError::InvalidArgument {
        argument: (*argument).to_owned(),
        context,
    })
}

fn invalid<T>(argument: &str, context: &'static str) -> Result<T, ParseError> {
    Err(ParseError::InvalidArgument {
        argument: argument.to_owned(),
        context,
    })
}

fn is_go_keyword(token: &str) -> bool {
    matches!(
        token,
        "searchmoves"
            | "ponder"
            | "wtime"
            | "btime"
            | "winc"
            | "binc"
            | "movestogo"
            | "depth"
            | "nodes"
            | "mate"
            | "movetime"
            | "infinite"
    )
}

fn requires_value(keyword: &str) -> bool {
    matches!(
        keyword,
        "wtime" | "btime" | "winc" | "binc" | "movestogo" | "depth" | "nodes" | "mate" | "movetime"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board_config::BigVariant;

    #[test]
    fn parses_position_with_moves() {
        let command = parse("position startpos moves e2e4 e7e5 g1f3").unwrap();
        assert_eq!(
            command,
            Some(Command::Position(Position {
                board: PositionBoard::StartPos,
                moves: vec![
                    UciMove {
                        from: Square(52),
                        to: Square(36),
                        promotion: None
                    },
                    UciMove {
                        from: Square(12),
                        to: Square(28),
                        promotion: None
                    },
                    UciMove {
                        from: Square(62),
                        to: Square(45),
                        promotion: None
                    },
                ],
            }))
        );
    }

    #[test]
    fn parses_fen_and_search_limits() {
        let position = parse(
            "position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 moves e2e4",
        )
        .unwrap();
        assert_eq!(
            position,
            Some(Command::Position(Position {
                board: PositionBoard::Fen(
                    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_owned()
                ),
                moves: vec![UciMove {
                    from: Square(52),
                    to: Square(36),
                    promotion: None
                }],
            }))
        );

        let go = parse("go searchmoves e2e4 d2d4 wtime 30000 btime 25000 depth 12").unwrap();
        assert_eq!(
            go,
            Some(Command::Go(SearchLimits {
                search_moves: vec![
                    UciMove {
                        from: Square(52),
                        to: Square(36),
                        promotion: None
                    },
                    UciMove {
                        from: Square(51),
                        to: Square(35),
                        promotion: None
                    },
                ],
                wtime: Some(30_000),
                btime: Some(25_000),
                depth: Some(12),
                ..SearchLimits::default()
            }))
        );
    }

    #[test]
    fn rejects_invalid_commands() {
        assert!(parse("position fen 8/8/8/8/8/8/8/8 w - - 0").is_err());
        assert!(parse("position startpos moves e2e9").is_err());
        assert!(parse("go depth many").is_err());
        assert!(parse("go searchmoves").is_err());
    }

    #[test]
    fn validates_coordinates_against_the_selected_variant() {
        let move_ = UciMove::parse_for::<BigVariant>("a10j12").unwrap();
        assert_eq!(
            move_,
            UciMove {
                from: Square(20),
                to: Square(9),
                promotion: None
            }
        );
        assert!(UciMove::parse("a10j12").is_err());
    }
}
