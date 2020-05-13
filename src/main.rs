use std::fmt;

#[derive(Debug)]
pub struct GameState {
    board: Board,
    dead_black: Vec<Piece>,
    dead_white: Vec<Piece>,
    black_has_castle: bool,
    white_has_castle: bool,
}

impl GameState {
    pub fn new() -> GameState {
        GameState {
            board: Board::blank(),
            dead_black: Vec::new(),
            dead_white: Vec::new(),
            black_has_castle: true,
            white_has_castle: true,
        }
    }
}

#[derive(Debug)]
pub struct Board {
    board: [[Option<Piece>; 8]; 8],
}

// Boards are arranged internally so that white is on the bottom and black is on the top.
// Hence, board[0][0] is the upper right of the board, and is black's leftmost square.
impl Board {
    pub fn blank() -> Board {
        Board {
            board: [[None; 8]; 8],
        }
    }

    pub fn default() -> Board {
        let setup = [
            "RNBQKBNR", // dont format this rustfmt thx
            "PPPPPPPP", //
            "........", //
            "........", //
            "........", //
            "........", //
            "PPPPPPPP", //
            "RNBQKBNR", //
        ];
        Board::from_string_array(setup)
    }

    pub fn from_string_array(array: [&str; 8]) -> Board {
        let mut board = [[None; 8]; 8];
        for (i, row) in array.iter().enumerate() {
            for (j, piece) in (*row).chars().enumerate() {
                use Color::*;
                use PieceType::*;
                let piece_type = match piece {
                    'P' => Some(Pawn),
                    'N' => Some(Knight),
                    'B' => Some(Bishop),
                    'R' => Some(Rook),
                    'Q' => Some(Queen),
                    'K' => Some(King),
                    _ => None,
                };
                let color = if i == 0 || i == 1 { Black } else { White };
                if let Some(piece) = piece_type {
                    board[i][j] = Some(Piece { piece, color });
                } else {
                    board[i][j] = None;
                }
            }
        }
        Board { board }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "         BLACK")?;
        for row in &self.board {
            for piece in row {
                match piece {
                    Some(piece) => write!(f, "{}{} ", piece.color, piece.piece)?,
                    None => write!(f, ".. ")?,
                };
            }
            writeln!(f, "")?;
        }
        writeln!(f, "         WHITE")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Piece {
    color: Color,
    piece: PieceType,
}

#[derive(Debug, Clone, Copy)]
pub enum Color {
    Black,
    White,
}
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Color::*;
        match self {
            Black => write!(f, "B"),
            White => write!(f, "W"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl fmt::Display for PieceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use PieceType::*;
        match self {
            Pawn => write!(f, "P"),
            Knight => write!(f, "N"),
            Bishop => write!(f, "B"),
            Rook => write!(f, "R"),
            Queen => write!(f, "Q"),
            King => write!(f, "K"),
        }
    }
}

fn main() {
    println!("{}", Board::default());
}
