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
// Hence, board[0][0] is the bottom right of the board, and is white's leftmost square.
impl Board {
    /// Create a chessboard with no pieces on it.
    pub fn blank() -> Board {
        Board {
            board: [[None; 8]; 8],
        }
    }

    /// Create a standard Chess board.
    pub fn default() -> Board {
        #[rustfmt::skip]
        let setup = [
            "RNBQKBNR",
            "PPPPPPPP",
            "........",
            "........",
            "........",
            "........",
            "PPPPPPPP",
            "RNBQKBNR",
        ];
        Board::from_string_array(setup)
    }

    /// Create a board from a string array. The array must have 8 strings,
    /// each of which must contain exactly 8 characters. Valid characters are
    /// P - Pawn
    /// N - Bishop
    /// R - Rook
    /// Q - Queen
    /// K - King
    /// All other characters are assumed to be "empty square"
    /// White is at the bottom, black is at the top.
    /// Note that the last bottom two rows are assumed to be white's and the
    /// top two rows are assumed to be black's.
    pub fn from_string_array(array: [&str; 8]) -> Board {
        let mut board = Board::blank();
        for (i, row) in array.iter().enumerate() {
            for (j, piece) in (*row).chars().enumerate() {
                use PieceType::*;
                let piece = match piece {
                    'P' => Some(Pawn),
                    'N' => Some(Knight),
                    'B' => Some(Bishop),
                    'R' => Some(Rook),
                    'Q' => Some(Queen),
                    'K' => Some(King),
                    _ => None,
                };
                let color = if i == 0 || i == 1 {
                    Color::Black
                } else {
                    Color::White
                };
                board.board[i][j] = piece.map(|piece| Piece { piece, color });
            }
        }
        board
    }

    /// Attempt to move the piece located at `start` to
    /// `end`. This function returns `Ok(())` if the move is
    /// valid (and updates the board correspondingly) and `Err(&str)` if the move
    /// fails (and does not alter the board).
    pub fn move_piece(
        &mut self,
        player: Color,
        start: (i8, i8),
        end: (i8, i8),
    ) -> Result<(), &'static str> {
        self.check_move(player, start, end)?;
        let moved_piece: Option<Piece> = self.get(start);
        self.set(end, moved_piece);
        self.set(start, None);
        Ok(())
    }

    /// Check if the piece located at `start` can be moved to
    /// `end`. This function returns `Ok(())` if the move is
    /// valid and `Err(&str)` if the move is invalid.
    fn check_move(
        &self,
        player: Color,
        start: (i8, i8),
        end: (i8, i8),
    ) -> Result<(), &'static str> {
        let start_piece = self.get(start);
        let end_piece = self.get(end);
        use Color::*;
        use PieceType::*;

        if start_piece == None {
            return Err("You aren't Roxy");
        }

        let start_piece = start_piece.unwrap();
        // You can't move a piece that isn't yours
        if start_piece.color != player {
            return Err("You aren't Caliborn");
        }

        // Pieces can move in two main ways
        // First, pieces may move onto Empty squares as long as all of the
        // intermediate squares are empty (except for the knight, which does not
        // require this)
        // Alternatively, pieces may move onto another piece of othe opposite color
        // so long as it captures the piece.
        // Pawns can move either forwards, or diagonally forwards
        match start_piece.piece {
            Pawn => {
                // TODO: first time pawn movement
                // a pawn can move directly forward if moving into empty space
                let valid_move_spot = (start.0, start.1 + player.direction());
                let valid_capture_spot1 = (start.0 + 1, start.1 + player.direction());
                let valid_capture_spot2 = (start.0 - 1, start.1 + player.direction());
                if end == valid_move_spot {
                    if end_piece == None {
                        return Ok(());
                    } else {
                        return Err("Can't move pawn into occupied space this way!");
                    }
                }
                // a pawn can move diagonally if capturing
                else if end == valid_capture_spot1 || end == valid_capture_spot2 {
                    if let Some(end_piece) = end_piece {
                        if end_piece.color != player {
                            return Ok(());
                        } else {
                            return Err("Can't move diagonally unless it's a capture");
                        }
                    } else {
                        return Err("Can't move diagonally unless it's a capture");
                    }
                } else {
                    println!(
                        "Valid pawn moves are: {:?} {:?} {:?}",
                        valid_move_spot, valid_capture_spot1, valid_capture_spot2
                    );
                    return Err("That's not a valid pawn move");
                }
            }
            Knight => unimplemented!(),
            Bishop => unimplemented!(),
            Rook => unimplemented!(),
            Queen => unimplemented!(),
            King => unimplemented!(),
        }
    }

    /// Gets the piece located at the coordinates
    fn get(&self, coord: (i8, i8)) -> Option<Piece> {
        // i promise very very hard that this i8 is, in fact, in the range 0-7
        self.board[(7 - coord.1) as usize][coord.0 as usize]
    }

    /// Sets the piece located at the coordinates
    fn set(&mut self, coord: (i8, i8), piece: Option<Piece>) {
        // i promise very very hard that this i8 is, in fact, in the range 0-7
        self.board[(7 - coord.1) as usize][coord.0 as usize] = piece;
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
                }
            }
            writeln!(f, "")?;
        }
        writeln!(f, "         WHITE")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    color: Color,
    piece: PieceType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    fn direction(self) -> i8 {
        match self {
            Color::White => 1,
            Color::Black => -1,
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Color::Black => write!(f, "B"),
            Color::White => write!(f, "W"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

/// Return a list of valid movement deltas (offsets from the piece) given a
/// PieceType. Move deltas take into account the piece's color.
/// This function returns Err on Rook, Bishop, and Queen
fn get_move_deltas(piece: Piece) -> Result<Vec<(i8, i8)>, &'static str> {
    use PieceType::*;
    match piece.piece {
        Pawn => Ok(vec![(0, 1)]),
        Knight => Ok(vec![
            (1, 2),
            (2, 1),
            (-1, 2),
            (-2, 1),
            (1, -2),
            (2, -1),
            (-1, -2),
            (-2, -1),
        ]),
        King => Ok(vec![
            (0, 1),
            (1, 1),
            (1, 0),
            (1, -1),
            (0, -1),
            (-1, -1),
            (-1, 0),
            (-1, 1),
        ]),
        Rook | Bishop | Queen => Err("Can't get move deltas for Rooks/Bishops/Kings"),
    }
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
