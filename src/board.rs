use std::fmt;

// use ggez::Context;

/// The overall board state, which keeps track of the various things each player
/// can do, such as if they can castle, or what pieces are currently dead.
#[derive(Debug, Clone)]
pub struct BoardState {
    /// The actual board containing all the pieces in play
    pub board: Board,
    /// The color of the player-to-move
    pub current_player: Color,
    dead_black: Vec<Piece>,
    dead_white: Vec<Piece>,
    black_has_castle: bool,
    white_has_castle: bool,
}

impl BoardState {
    /// Create a board state using the board given. The player-to-move will
    /// initially be white.
    pub fn new(board: Board) -> BoardState {
        BoardState {
            board,
            current_player: Color::White,
            dead_black: Vec::new(),
            dead_white: Vec::new(),
            black_has_castle: true,
            white_has_castle: true,
        }
    }

    /// Attempt to move the piece located at `start` to `end`. This function
    /// returns `Ok()` if the move was successful and `Err` if it was not.
    /// It also sets `current_player` to the opposite color.
    pub fn take_turn(&mut self, start: BoardCoord, end: BoardCoord) -> Result<(), &'static str> {
        use Color::*;
        self.board.check_move(self.current_player, start, end)?;
        self.board.move_piece(start, end);
        self.current_player = match self.current_player {
            White => Black,
            Black => White,
        };

        Ok(())
    }

    /// Return the list of valid places the piece at `coord` can move. This
    /// function takes into account `current_player`. Note that the returned
    /// vector is empty if any of the follow are true.
    /// - `coord` is off the board
    /// - `coord` refers to an empty tile
    /// - `coord` refers to a piece that is the opposite color of `current_player`
    /// - `coord` refers to a piece that has nowhere to move
    /// Also note that this function DOES check if the move would place the
    /// king into check.
    pub fn get_move_list(&self, coord: BoardCoord) -> Vec<BoardCoord> {
        if !on_board(coord) {
            return vec![];
        }

        // If not a piece
        let tile = self.board.get(coord).0;
        if tile.is_none() {
            return vec![];
        }

        // If not a piece of the player's color
        let piece = tile.unwrap();
        if piece.color != self.current_player {
            return vec![];
        }

        let list = get_move_list_ignore_check(&self.board, coord);

        filter_check_causing_moves(&self.board, self.current_player, coord, list).0
    }

    /// Returns if the current player is currently in checkmate
    pub fn is_checkmate(&self) -> CheckmateState {
        self.board.is_checkmate(self.current_player)
    }

    /// Try to get the `Tile` at `coord`. This function returns `None` if `coord`
    /// would be off the board.
    pub fn get(&self, coord: BoardCoord) -> Tile {
        self.board.get(coord)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckmateState {
    Normal,
    Check,
    Checkmate,
    Stalemate,
}

/// Wrapper struct around an 8x8 array of Tiles. This represents the state of
/// pieces on the board. Note that Boards are arranged internally so that white
/// is on the bottom and black is on the top. Hence, `board[0][0]` is the bottom
/// left of the board, and is white's leftmost square, while `board[7][7]` is
/// the top right of the board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    board: [[Tile; 8]; 8],
}

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
        let setup = vec![
            "BR BN BB BQ BK BB BN BR",
            "BP BP BP BP BP BP BP BP",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            "WP WP WP WP WP WP WP WP",
            "WR WN WB WQ WK WB WN WR",
        ];
        Board::from_string_vec(setup)
    }

    /// Create a board from a string array. The array assumes that each string
    /// can be split into exactly 8 two character substrings, each either being
    /// "B" or "W" in the first character and a P, N, B, R, Q, or K in the
    /// second character. Anything else is treated as a blank Tile.
    pub fn from_string_vec(str_board: Vec<&str>) -> Board {
        let mut board = Board::blank();
        for (i, row) in str_board.iter().enumerate() {
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
                let x = j;
                let y = i + (8 - str_board.len());
                board.board[y][x] = Tile(piece.map(|piece| Piece { piece, color }));
            }
        }
        board
    }

    /// Moves the piece located at `start` to `end`. This function always moves
    /// the piece, even if it would not be actually legal to do so in a real
    /// game, so you should check the move first with `check_move`
    pub fn move_piece(&mut self, start: BoardCoord, end: BoardCoord) {
        let moved_piece = self.get(start);
        self.set(end, moved_piece);
        self.set(start, Tile(None));
    }

    /// Check if the piece located at `start` can be legally be moved to
    /// `end`. This function assumes the player-to-move is whatever `player` is.
    /// This function returns `Ok(())` if the move is valid and `Err(&str)` if
    /// the move is invalid.
    pub fn check_move(
        &self,
        player: Color,
        start: BoardCoord,
        end: BoardCoord,
    ) -> Result<(), &'static str> {
        let start_piece = self.get(start);

        if start_piece.0.is_none() {
            return Err("You aren't Roxy");
        }

        let start_piece = start_piece.0.unwrap();
        // You can't move a piece that isn't yours
        if start_piece.color != player {
            return Err("You aren't Caliborn");
        }
        let valid_end_spots = get_move_list_ignore_check(self, start).0;

        if valid_end_spots.contains(&end) {
            Ok(())
        } else {
            Err("Can't move a piece there")
        }
    }

    /// Returns if the player is currently in checkmate
    fn is_checkmate(&self, player: Color) -> CheckmateState {
        use CheckmateState::*;
        match (self.has_legal_moves(player), self.is_in_check(player)) {
            (false, false) => Stalemate,
            (false, true) => Checkmate,
            (true, false) => Normal,
            (true, true) => Check,
        }
    }

    fn has_legal_moves(&self, player: Color) -> bool {
        // TODO: this is also HILARIOUSLY INEFFICENT
        for i in 0..8 {
            for j in 0..8 {
                let coord = BoardCoord::new((j, i)).unwrap();
                let tile = self.get(coord);
                if tile.is_color(player) {
                    if !get_move_list_with_check(self, player, coord).0.is_empty() {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn is_in_check(&self, player: Color) -> bool {
        !self.is_square_safe(player, &self.get_king(player).unwrap())
    }

    /// Gets the piece located at the coordinates
    pub fn get(&self, BoardCoord(x, y): BoardCoord) -> Tile {
        // i promise very very hard that this i8 is, in fact, in the range 0-7
        self.board[(7 - y) as usize][x as usize]
    }

    /// Sets the piece located at the coordinates
    fn set(&mut self, BoardCoord(x, y): BoardCoord, piece: Tile) {
        // i promise very very hard that this i8 is, in fact, in the range 0-7
        self.board[(7 - y) as usize][x as usize] = piece;
    }

    /// Returns true if no piece of the opposite color threatens the square.
    fn is_square_safe(&self, color: Color, check_coord: &BoardCoord) -> bool {
        // TODO: this is hilariously inefficient
        for i in 0..8 {
            for j in 0..8 {
                let coord = BoardCoord::new((j, i)).unwrap();
                let tile = self.get(coord);
                if tile.0.is_none() {
                    continue;
                }
                let piece = tile.0.unwrap();
                if piece.color != color {
                    let move_list = get_move_list_ignore_check(self, coord);
                    if move_list.0.contains(&check_coord) {
                        return false;
                    } else {
                        continue;
                    }
                }
            }
        }

        true
    }

    /// Attempts to return the coordinates the king of the specified color
    fn get_king(&self, color: Color) -> Option<BoardCoord> {
        for i in 0..8 {
            for j in 0..8 {
                let coord = BoardCoord::new((j, 7 - i)).unwrap();
                let tile = self.get(coord);
                if tile.is(color, PieceType::King) {
                    return Some(coord);
                }
            }
        }
        None
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
/// A board space coordinate. The origin is at the bottom left and (7, 7) is at
/// the top right. This is in line with how rank-file notation works~~, and also
/// is how graphics should work~~
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoardCoord(pub i8, pub i8);

impl BoardCoord {
    pub fn new((x, y): (i8, i8)) -> Result<BoardCoord, &'static str> {
        if on_board_i8((x, y)) {
            Ok(BoardCoord(x, y))
        } else {
            Err("Expected coordinates to be in range 0-7")
        }
    }
}

/// A list of spaces that a piece may move to.
pub struct MoveList(pub Vec<BoardCoord>);

/// Return a MoveList of the piece located at `coord`.
/// This function DOES check if a move made by the King would put the King into
/// check.
fn get_move_list_with_check(board: &Board, player: Color, coord: BoardCoord) -> MoveList {
    let list = get_move_list_ignore_check(&board, coord);
    filter_check_causing_moves(&board, player, coord, list)
}

/// This function attempts to move the piece at `coord` to each destination
/// in `move_list`, and removes the move if it would cause the King of `color`
/// to be in check.
fn filter_check_causing_moves(
    board: &Board,
    color: Color,
    coord: BoardCoord,
    move_list: MoveList,
) -> MoveList {
    // for each attempted move, see if moving there would
    // actually put the king in check. we need to actually move the
    // piece there because we want to avoid the chance that the
    // "old" king location causes a line of sight piece to be blocked
    // EX: we have a board like this
    // .. .. ..
    // BR WK ..
    // .. .. ..
    // and WK attempts to move right. From this current board layout,
    // BR can't attack the square to the right of WK (because WK
    // blocks LoS), but it would be attacked if WK actually did move
    // there.
    let mut filtered_list = vec![];
    for attempted_move in move_list.0 {
        let mut moved_board = board.clone();
        moved_board.move_piece(coord, attempted_move);
        let king_coord = moved_board.get_king(color).unwrap();
        if moved_board.is_square_safe(color, &king_coord) {
            filtered_list.push(attempted_move);
        }
    }
    MoveList(filtered_list)
}

/// Return a MoveList of the piece located at `coord`. This function assumes
/// that the player to move is whatever the color of the piece at `coord` is.
/// This function does NOT check if a move made by the King would put the King into
/// check.
fn get_move_list_ignore_check(board: &Board, coord: BoardCoord) -> MoveList {
    let piece = board.get(coord).0;
    use PieceType::*;
    match piece {
        None => MoveList(vec![]),
        Some(piece) => match piece.piece {
            Pawn => check_pawn(board, coord, piece.color),
            Knight | King => {
                check_jump_piece(board, coord, piece.color, get_move_deltas(piece.piece))
            }

            Bishop | Rook | Queen => {
                check_line_of_sight_piece(board, coord, piece.color, get_los(piece.piece))
            }
        },
    }
}

/// Get a list of the locations the pawn at `pos` can move to. The pawn's color
/// is assumed to be `color`. Note that this function doesn't actually check if
/// there is a pawn at `pos`.
fn check_pawn(board: &Board, pos: BoardCoord, color: Color) -> MoveList {
    let mut valid_end_pos = MoveList(Vec::new());

    // Check forward space if it can be moved into.
    let forwards = BoardCoord(pos.0, pos.1 + color.direction());
    if on_board(forwards) {
        if board.get(forwards).0.is_none() {
            valid_end_pos.0.push(forwards);
        }
    }

    // Check diagonal spaces if they can be attacked.
    for &diagonal in &[
        BoardCoord(pos.0 + 1, pos.1 + color.direction()),
        BoardCoord(pos.0 - 1, pos.1 + color.direction()),
    ] {
        if on_board(diagonal) {
            match board.get(diagonal).0 {
                Some(piece) if piece.color != color => valid_end_pos.0.push(diagonal),
                _ => {}
            }
        }
    }
    valid_end_pos
}

/// Get a list of valid locations the "jump" piece may move or capture to.
/// The `move_deltas` is a list of offsets that the piece may potenitally move
/// to. The piece's color is assumed to be `color`. The piece may actually move
/// there if the Tile is on the board and is either unoccupied (a move) or is a
/// piece of the opposite color (a capture). Note that this function doesn't
/// actually check the piece at `pos`.
fn check_jump_piece(
    board: &Board,
    pos: BoardCoord,
    color: Color,
    move_deltas: Vec<BoardCoord>,
) -> MoveList {
    let mut valid_end_pos = MoveList(Vec::new());
    for delta in move_deltas {
        let end_pos = BoardCoord(pos.0 + delta.0, pos.1 + delta.1 * color.direction());
        if !on_board(end_pos) {
            continue;
        }
        // A piece may actually move to end_pos if the location is unoccupied
        // or contains a piece of the opposite color.
        match board.get(end_pos).0 {
            Some(piece) if piece.color != color => valid_end_pos.0.push(end_pos),
            None => valid_end_pos.0.push(end_pos),
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
    pos: BoardCoord,
    color: Color,
    line_of_sights: Vec<Vec<BoardCoord>>,
) -> MoveList {
    let mut valid_end_pos = MoveList(Vec::new());
    for los in line_of_sights {
        for delta in los {
            let end_pos = BoardCoord(pos.0 + delta.0, pos.1 + delta.1 * color.direction());

            if !on_board(end_pos) {
                break;
            }

            let end_piece = board.get(end_pos).0;
            if end_piece.is_none() {
                valid_end_pos.0.push(end_pos);
            } else if let Some(piece) = end_piece {
                if piece.color != color {
                    valid_end_pos.0.push(end_pos);
                }
                break;
            }
        }
    }

    valid_end_pos
}

/// Returns LoS for Rooks, Bishops, and Queens. Panics on other PieceTypes.
fn get_los(piece: PieceType) -> Vec<Vec<BoardCoord>> {
    use PieceType::*;
    match piece {
        Rook => get_los_rook(),
        Bishop => get_los_bishop(),
        Queen => [get_los_rook(), get_los_bishop()].concat(),
        Pawn | Knight | King => panic!("Expected a Rook, Bishop, or Queen. Got {:?}", piece),
    }
}

fn get_los_rook() -> Vec<Vec<BoardCoord>> {
    let mut los_right = Vec::new();
    let mut los_left = Vec::new();
    let mut los_up = Vec::new();
    let mut los_down = Vec::new();
    for i in 1..8 {
        los_right.push(BoardCoord(i, 0));
        los_left.push(BoardCoord(-i, 0));
        los_up.push(BoardCoord(0, i));
        los_down.push(BoardCoord(0, -i));
    }
    return vec![los_up, los_down, los_right, los_left];
}

fn get_los_bishop() -> Vec<Vec<BoardCoord>> {
    let mut los_up_right = Vec::new();
    let mut los_up_left = Vec::new();
    let mut los_down_right = Vec::new();
    let mut los_down_left = Vec::new();
    for i in 1..8 {
        los_up_right.push(BoardCoord(i, i));
        los_up_left.push(BoardCoord(-i, i));
        los_down_right.push(BoardCoord(i, -i));
        los_down_left.push(BoardCoord(-i, -i));
    }

    return vec![los_up_right, los_up_left, los_down_right, los_down_left];
}

/// Return a list of valid movement deltas (offsets from the piece) given a
/// PieceType. Move deltas DO NOT take into account the piece's color.
/// This function only works on Knights and Kings and panics on everything else.
fn get_move_deltas(piece: PieceType) -> Vec<BoardCoord> {
    use PieceType::*;
    match piece {
        Knight => vec![
            BoardCoord(1, 2),
            BoardCoord(2, 1),
            BoardCoord(-1, 2),
            BoardCoord(-2, 1),
            BoardCoord(1, -2),
            BoardCoord(2, -1),
            BoardCoord(-1, -2),
            BoardCoord(-2, -1),
        ],
        King => vec![
            BoardCoord(0, 1),
            BoardCoord(1, 1),
            BoardCoord(1, 0),
            BoardCoord(1, -1),
            BoardCoord(0, -1),
            BoardCoord(-1, -1),
            BoardCoord(-1, 0),
            BoardCoord(-1, 1),
        ],
        Pawn | Rook | Bishop | Queen => panic!("Expected a Knight or a King, got {:?}", piece),
    }
}

/// Return true if `pos` would be actually on the board.
pub fn on_board(pos: BoardCoord) -> bool {
    0 <= pos.0 && pos.0 < 8 && 0 <= pos.1 && pos.1 < 8
}

/// Return true if `pos` would be actually on the board.
pub fn on_board_i8(pos: (i8, i8)) -> bool {
    0 <= pos.0 && pos.0 < 8 && 0 <= pos.1 && pos.1 < 8
}

impl fmt::Display for MoveList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for i in 0..8 {
            for j in 0..8 {
                let coord = BoardCoord(j, 7 - i);
                if self.0.contains(&coord) {
                    write!(f, "## ")?;
                } else {
                    write!(f, ".. ")?;
                }
            }
            writeln!(f, "")?;
        }
        Ok(())
    }
}
/// Newtype wrapper for `Option<Piece>`. `Some(piece)` indicates that a piece is
/// in the tile, and `None` indicates that the tile is empty. Used in `Board`.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Tile(pub Option<Piece>);

impl Tile {
    /// Returns true if the `Tile` actually has a piece and
    /// `color` and `piece_type` match.
    pub fn is(&self, color: Color, piece_type: PieceType) -> bool {
        match self.0 {
            None => false,
            Some(piece) => piece.color == color && piece.piece == piece_type,
        }
    }

    /// Returns true if the `Tile` actually has a piece and
    /// `color` matches the color of the piece.
    fn is_color(&self, color: Color) -> bool {
        match self.0 {
            None => false,
            Some(piece) => piece.color == color,
        }
    }

    /// Returns true if the `Tile` actually has a piece and
    /// `piece` matches the `PieceType` of the piece.
    fn is_type(&self, piece_type: PieceType) -> bool {
        match self.0 {
            None => false,
            Some(piece) => piece.piece == piece_type,
        }
    }

    /// Return a string representation of this Tile (currently uses unicode
    /// chess piece characters)
    pub fn as_str(&self) -> &'static str {
        use PieceType::*;
        match self.0 {
            None => "",
            Some(piece) => match piece.piece {
                Pawn => "♟",
                Knight => "♞",
                Bishop => "♝",
                Rook => "♜",
                Queen => "♛",
                King => "♚",
            },
        }
    }
}

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
    pub color: Color,
    piece: PieceType,
}

/// The available player colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    /// Returns 1 if White, -1 if Black. This is used to indicate the direction
    /// that pieces move in (particularly the Pawn)
    fn direction(self) -> i8 {
        match self {
            Color::White => 1,
            Color::Black => -1,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Color::White => "White",
            Color::Black => "Black",
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

/// An enum that describes the six possible pieces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_from_string_vec() {
        #[rustfmt::skip]
        let board = vec![
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. WP .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
        ];
        let board = Board::from_string_vec(board);
        let mut expected = Board::blank();
        expected.set(
            BoardCoord(3, 3),
            Tile(Some(Piece {
                piece: PieceType::Pawn,
                color: Color::White,
            })),
        );
        println!("real\n{}\nexpected\n{}", board, expected);
        assert_eq!(board, expected);
    }

    #[test]
    fn test_from_string_vec_small() {
        #[rustfmt::skip]
        let board = vec![
            ".. .. .. WP",
            ".. .. .. ..",
            ".. .. .. ..",
            ".. .. .. ..",
        ];
        let board = Board::from_string_vec(board);
        let mut expected = Board::blank();
        expected.set(
            BoardCoord(3, 3),
            Tile(Some(Piece {
                piece: PieceType::Pawn,
                color: Color::White,
            })),
        );
        println!("real\n{}\nexpected\n{}", board, expected);
        assert_eq!(board, expected);
    }

    #[test]
    fn test_from_string_vec_smallest() {
        let board = vec!["WP"];
        let board = Board::from_string_vec(board);
        let mut expected = Board::blank();
        expected.set(
            BoardCoord(0, 0),
            Tile(Some(Piece {
                piece: PieceType::Pawn,
                color: Color::White,
            })),
        );
        println!("real\n{}\nexpected\n{}", board, expected);
        assert_eq!(board, expected);
    }

    #[test]
    fn test_pawn_move() {
        #[rustfmt::skip]
        let board = vec![
            ".. .. ..",
            ".. .. ..",
            ".. WP ..",
            ".. .. ..",
        ];
        #[rustfmt::skip]
        let expected = vec![
            ".. .. ..",
            ".. ## ..",
            ".. WP ..",
            ".. .. ..",
        ];
        assert_valid_movement(board, (1, 1), expected);
    }

    #[test]
    fn test_pawn_move_black() {
        #[rustfmt::skip]
        let board = vec![
            ".. .. ..",
            ".. BP ..",
            ".. .. ..",
            ".. .. ..",
        ];
        #[rustfmt::skip]
        let expected = vec![
            ".. .. ..",
            ".. BP ..",
            ".. ## ..",
            ".. .. ..",
        ];
        assert_valid_movement(board, (1, 2), expected);
    }

    #[test]
    fn test_pawn_capture() {
        #[rustfmt::skip]
        let board = vec![
            ".. .. ..",
            "BN BP BQ",
            ".. WP ..",
            ".. .. ..",
        ];
        #[rustfmt::skip]
        let expected = vec![
            ".. .. ..",
            "## .. ##",
            ".. WP ..",
            ".. .. ..",
        ];
        assert_valid_movement(board, (1, 1), expected);
    }

    #[test]
    fn test_pawn_cant_move() {
        #[rustfmt::skip]
        let board = vec![
            ".. .. ..",
            "WN BK WQ",
            ".. WP ..",
            ".. .. ..",
        ];
        #[rustfmt::skip]
        let expected = vec![
            ".. .. ..",
            ".. .. ..",
            ".. WP ..",
            ".. .. ..",
        ];
        assert_valid_movement(board, (1, 1), expected);
    }

    #[test]
    fn test_king() {
        #[rustfmt::skip]
        let board = vec![
            ".. .. ..",
            ".. WK ..",
            ".. .. ..",
        ];
        #[rustfmt::skip]
        let expected = vec![
            "## ## ##",
            "## WK ##",
            "## ## ##",
        ];
        assert_valid_movement(board, (1, 1), expected);
    }

    #[test]
    fn test_king_capture() {
        #[rustfmt::skip]
        let board = vec![
            "WP BP BP",
            "BP WK WP",
            "WP WP BP",
        ];
        #[rustfmt::skip]
        let expected = vec![
            "WP ## ##",
            "## WK WP",
            "WP WP ##",
        ];
        assert_valid_movement(board, (1, 1), expected);
    }

    #[test]
    fn test_king_capture_black() {
        #[rustfmt::skip]
        let board = vec![
            "WP BP BP",
            "BP BK WP",
            "WP WP BP",
        ];
        #[rustfmt::skip]
        let expected = vec![
            "## BP BP",
            "BP WK ##",
            "## ## BP",
        ];
        assert_valid_movement(board, (1, 1), expected);
    }
    #[test]
    fn test_knight() {
        #[rustfmt::skip]
        let board = vec![
            ".. .. .. BP ..",
            ".. .. .. .. BP",
            ".. .. WN .. ..",
            "BP .. .. .. ..",
            ".. BP .. .. ..",
        ];
        #[rustfmt::skip]
        let expected = vec![
            ".. ## .. ## ..",
            "## .. .. .. ##",
            ".. .. WN .. ..",
            "## .. .. .. ##",
            ".. ## .. ## ..",
        ];
        assert_valid_movement(board, (2, 2), expected);
    }

    #[test]
    fn test_knight_jump() {
        #[rustfmt::skip]
        let board = vec![
            "WP WP WP .. WP",
            "WP WP WP WP ..",
            "WP WP WN WP WP",
            "BP WP WP WP WP",
            "WP BP WP WP WP",
        ];
        #[rustfmt::skip]
        let expected = vec![
            ".. .. .. ## ..",
            ".. .. .. .. ##",
            ".. .. WN .. ..",
            "## .. .. .. ..",
            ".. ## .. .. ..",
        ];
        assert_valid_movement(board, (2, 2), expected);
    }

    #[test]
    fn test_rook_los() {
        #[rustfmt::skip]
        let board = vec![
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. WR .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
        ];
        #[rustfmt::skip]
        let expected = vec![
            ".. .. .. ## .. .. .. ..",
            ".. .. .. ## .. .. .. ..",
            ".. .. .. ## .. .. .. ..",
            ".. .. .. ## .. .. .. ..",
            "## ## ## WR ## ## ## ##",
            ".. .. .. ## .. .. .. ..",
            ".. .. .. ## .. .. .. ..",
            ".. .. .. ## .. .. .. ..",
        ];
        assert_valid_movement(board, (3, 3), expected);
    }

    #[test]
    fn test_rook_los_blocked() {
        #[rustfmt::skip]
        let board = vec![
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. WP .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. BP .. WR BP .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. WP .. .. .. ..",
        ];
        #[rustfmt::skip]
        let expected = vec![
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. ## .. .. .. ..",
            ".. ## ## WR ## .. .. ..",
            ".. .. .. ## .. .. .. ..",
            ".. .. .. ## .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
        ];
        assert_valid_movement(board, (3, 3), expected);
    }

    fn assert_valid_movement(board: Vec<&str>, coord: (i8, i8), expected: Vec<&str>) {
        let coord = BoardCoord(coord.0, coord.1);
        let board = Board::from_string_vec(board);
        let expected = to_move_list(expected);
        let move_list = get_move_list_ignore_check(&board, coord);
        println!("Board\n{}", board);
        println!("Actual Move List");
        println!("{}", &move_list);
        println!("Expected Move List");
        println!("{}", &expected);

        let mut move_list_counts = HashMap::new();
        for ele in move_list.0 {
            let entry = move_list_counts.entry(ele).or_insert(0);
            *entry += 1;
        }

        let mut expected_counts = HashMap::new();
        for ele in expected.0 {
            let entry = expected_counts.entry(ele).or_insert(0);
            *entry += 1;
        }

        assert_eq!(move_list_counts, expected_counts);
    }

    /// Create list of valid moves given a string board
    fn to_move_list(array: Vec<&str>) -> MoveList {
        let mut move_list = MoveList(Vec::new());
        for (y, row) in array.iter().enumerate() {
            for (x, tile) in (*row).split_whitespace().enumerate() {
                if tile.starts_with("#") {
                    move_list
                        .0
                        .push(BoardCoord(x as i8, ((array.len() - 1) - y) as i8));
                };
            }
        }
        move_list
    }
}
