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

#[derive(Debug, PartialEq, Eq)]
pub struct Board {
    board: [[Tile; 8]; 8],
}

// Boards are arranged internally so that white is on the bottom and black is on the top.
// Hence, board[0][0] is the bottom right of the board, and is white's leftmost square.
impl Board {
    /// Create a chessboard with no pieces on it.
    pub fn blank() -> Board {
        Board {
            board: [[Tile(None); 8]; 8],
        }
    }

    /// Create a standard Chess board.
    pub fn default() -> Board {
        #[rustfmt::skip]
        let setup = [
            "BR BN BB BQ BK BB BN BR",
            "BP BP BP BP BP BP BP BP",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            "WP WP WP WP WP WP WP WP",
            "WR WN WB WQ WK WB WN WR",
        ];
        Board::from_string_array(setup)
    }

    /// Create a board from a string array.
    pub fn from_string_array(array: [&str; 8]) -> Board {
        let mut board = Board::blank();
        for (i, row) in array.iter().enumerate() {
            for (j, piece) in (*row).split_whitespace().enumerate() {
                let color = match piece.chars().nth(0).unwrap() {
                    'B' => Color::Black,
                    _ => Color::White,
                };
                use PieceType::*;
                let piece = match piece.chars().nth(1).unwrap() {
                    'P' => Some(Pawn),
                    'N' => Some(Knight),
                    'B' => Some(Bishop),
                    'R' => Some(Rook),
                    'Q' => Some(Queen),
                    'K' => Some(King),
                    _ => None,
                };
                board.board[i][j] = Tile(piece.map(|piece| Piece { piece, color }));
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
        let moved_piece = self.get(start);
        self.set(end, moved_piece);
        self.set(start, Tile(None));
        Ok(())
    }

    /// Check if the piece located at `start` can be moved to
    /// `end`. This function returns `Ok(())` if the move is
    /// valid and `Err(&str)` if the move is invalid.
    pub fn check_move(
        &self,
        player: Color,
        start: (i8, i8),
        end: (i8, i8),
    ) -> Result<(), &'static str> {
        let start_piece = self.get(start);
        // let end_piece = self.get(end);
        use PieceType::*;

        if start_piece.0.is_none() {
            return Err("You aren't Roxy");
        }

        let start_piece = start_piece.0.unwrap();
        // You can't move a piece that isn't yours
        if start_piece.color != player {
            return Err("You aren't Caliborn");
        }
        let valid_end_spots: Vec<(i8, i8)> = match start_piece.piece {
            Pawn => check_pawn(self, start, player),
            Knight | King => check_jump_piece(
                self,
                start,
                player,
                get_move_deltas(start_piece.piece).unwrap(),
            ),
            Bishop | Rook | Queen => {
                check_line_of_sight_piece(self, start, player, get_los(start_piece.piece).unwrap())
            }
        };

        if valid_end_spots.contains(&end) {
            Ok(())
        } else {
            Err("Can't move a piece there")
        }
    }

    /// Gets the piece located at the coordinates
    pub fn get(&self, coord: (i8, i8)) -> Tile {
        // i promise very very hard that this i8 is, in fact, in the range 0-7
        self.board[(7 - coord.1) as usize][coord.0 as usize]
    }

    /// Sets the piece located at the coordinates
    fn set(&mut self, coord: (i8, i8), piece: Tile) {
        // i promise very very hard that this i8 is, in fact, in the range 0-7
        self.board[(7 - coord.1) as usize][coord.0 as usize] = piece;
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "         BLACK")?;
        for row in &self.board {
            for piece in row {
                write!(f, "{} ", piece)?;
            }
            writeln!(f, "")?;
        }
        writeln!(f, "         WHITE")?;
        Ok(())
    }
}

fn check_pawn(board: &Board, pos: (i8, i8), color: Color) -> Vec<(i8, i8)> {
    let mut valid_end_pos = Vec::new();
    let forwards = (pos.0, pos.1 + color.direction());
    if on_board(forwards) {
        if board.get(forwards).0.is_none() {
            valid_end_pos.push(forwards);
        }
    }

    for &diagonal in &[
        (pos.0 + 1, pos.1 + color.direction()),
        (pos.0 - 1, pos.1 + color.direction()),
    ] {
        if on_board(diagonal) {
            match board.get(diagonal).0 {
                Some(piece) if piece.color != color => valid_end_pos.push(diagonal),
                _ => {}
            }
        }
    }
    valid_end_pos
}

/// Get a list of valid locations the "jump" piece may move or capture to.
/// The `move_deltas` is a list of offsets that the piece may potenitally move
/// to. The piece may actually move there if the Tile is on the board and is
/// Either unoccupied (a move) or is a piece of the opposite color (a capture).
fn check_jump_piece(
    board: &Board,
    pos: (i8, i8),
    color: Color,
    move_deltas: Vec<(i8, i8)>,
) -> Vec<(i8, i8)> {
    let mut valid_end_pos = Vec::new();
    for delta in move_deltas {
        let end_pos = (pos.0 + delta.0, pos.1 + delta.1 * color.direction());
        if !on_board(end_pos) {
            continue;
        }
        // A piece may actually move to end_pos if the location is unoccupied
        // or contains a piece of the opposite color.
        match board.get(end_pos).0 {
            Some(piece) if piece.color != color => valid_end_pos.push(end_pos),
            None => valid_end_pos.push(end_pos),
            _ => {}
        }
    }
    valid_end_pos
}

/// Get a list of valid locations the "LoS" piece may move or capture to.
/// The `lines_of_sight` is consists of lines of sights. A line of sight is a
/// list of move deltas arranged in the order that the piece can "see" (ie: for)
/// Rooks, the line of sight starts closest to the Rook, and goes away from it
/// in an orthogonal direction. Lines of sight end on the first piece of the opposite
/// color or just before the first piece of the same color.
fn check_line_of_sight_piece(
    board: &Board,
    pos: (i8, i8),
    color: Color,
    line_of_sights: Vec<Vec<(i8, i8)>>,
) -> Vec<(i8, i8)> {
    let mut valid_end_pos = Vec::new();
    for los in line_of_sights {
        for delta in los {
            let end_pos = (pos.0 + delta.0, pos.1 + delta.1 * color.direction());

            if !on_board(end_pos) {
                break;
            }

            let end_piece = board.get(end_pos).0;
            if end_piece.is_none() {
                valid_end_pos.push(end_pos);
            } else if let Some(piece) = end_piece {
                if piece.color != color {
                    valid_end_pos.push(end_pos);
                }
                break;
            }
        }
    }

    valid_end_pos
}

fn get_los(piece: PieceType) -> Result<Vec<Vec<(i8, i8)>>, &'static str> {
    use PieceType::*;
    match piece {
        Rook => Ok(get_los_rook()),
        Bishop => Ok(get_los_bishop()),
        Queen => Ok([get_los_rook(), get_los_bishop()].concat()),
        _ => Err("Only Rooks, Bishops, and Queens are line of sight pieces"),
    }
}

fn get_los_rook() -> Vec<Vec<(i8, i8)>> {
    let mut los_right = Vec::new();
    let mut los_left = Vec::new();
    let mut los_up = Vec::new();
    let mut los_down = Vec::new();
    for i in 1..8 {
        los_right.push((i, 0));
        los_left.push((-i, 0));
        los_up.push((0, i));
        los_down.push((0, -i));
    }
    return vec![los_up, los_down, los_right, los_left];
}

fn get_los_bishop() -> Vec<Vec<(i8, i8)>> {
    let mut los_up_right = Vec::new();
    let mut los_up_left = Vec::new();
    let mut los_down_right = Vec::new();
    let mut los_down_left = Vec::new();
    for i in 1..8 {
        los_up_right.push((i, i));
        los_up_left.push((-i, i));
        los_down_right.push((i, -i));
        los_down_left.push((-i, -i));
    }

    return vec![los_up_right, los_up_left, los_down_right, los_down_left];
}

fn on_board(pos: (i8, i8)) -> bool {
    0 <= pos.0 && pos.0 < 8 && 0 <= pos.1 && pos.1 < 8
}

/// Newtype wrapper for `Option<Piece>`. `Some(piece)` indicates that a piece is
/// in the tile, and `None` indicates that the tile is empty. Used in `Board`.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Tile(Option<Piece>);

impl fmt::Display for Tile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Some(piece) => write!(f, "{}{}", piece.color, piece.piece),
            None => write!(f, ".."),
        }
    }
}

/// A chess piece which has a color and the type of piece it is.
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
/// PieceType. Move deltas DO NOT take into account the piece's color.
/// This function returns Err on Rook, Bishop, and Queen
fn get_move_deltas(piece: PieceType) -> Result<Vec<(i8, i8)>, &'static str> {
    use PieceType::*;
    match piece {
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
