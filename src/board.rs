use std::fmt;

#[cfg(feature = "perf")]
use flame as fire;
#[cfg(feature = "perf")]
use flamer::flame;

const ROWS: std::ops::Range<i8> = 0..8;
const COLS: std::ops::Range<i8> = 0..8;
pub const PAWN_STR: &str = "♟";
pub const KNIGHT_STR: &str = "♞";
pub const BISHOP_STR: &str = "♝";
pub const ROOK_STR: &str = "♜";
pub const QUEEN_STR: &str = "♛";
pub const KING_STR: &str = "♚";

// use ggez::Context;
/// The overall board state, which keeps track of the various things each player
/// can do, such as if they can castle, or what pieces are currently dead.
#[derive(Debug, Clone)]
pub struct BoardState {
    /// The actual board containing all the pieces in play
    pub board: Board,
    /// The color of the player-to-move
    pub current_player: Color,
    pub dead_black: Vec<Piece>,
    pub dead_white: Vec<Piece>,
    pub checkmate: CheckmateState,
}

impl BoardState {
    /// Create a board state using the board given. The player-to-move will
    /// initially be white.
    pub fn new(board: Board) -> BoardState {
        let checkmate = board.checkmate_state(Color::White);
        BoardState {
            board,
            current_player: Color::White,
            dead_black: Vec::new(),
            dead_white: Vec::new(),
            checkmate,
        }
    }

    pub fn check_turn(&self, start: BoardCoord, end: BoardCoord) -> Result<(), &'static str> {
        use MoveType::*;

        if self.board.pawn_needs_promotion().is_some() {
            return Err("A pawn needs to be promoted");
        }

        match move_type(&self.board, start, end) {
            Castle(color, side) => self.board.can_castle(color, side),
            Normal | Lunge | Capture => self.board.check_move(self.current_player, start, end),
            EnPassant(side) => self.board.check_enpassant(self.current_player, start, side),
        }
    }

    /// Move the piece located at `start` to `end`. This function panics if the
    /// move would be illegal, so you should check the move first with `check_turn`
    /// It also sets `current_player` to the opposite color and handles the
    /// "just_lunged" pawn flags.
    /// A pawn needs promotion then this function always fails. You should
    /// call `promote` on the pawn.
    #[cfg_attr(feature = "perf", flame)]
    pub fn take_turn(&mut self, start: BoardCoord, end: BoardCoord) {
        use Color::*;
        use MoveType::*;

        debug_assert!(self.check_turn(start, end).is_ok());

        #[cfg(feature = "perf")]
        let guard = fire::start_guard("move check + apply");

        match move_type(&self.board, start, end) {
            Castle(color, side) => {
                // Clear the just lunged flags _after_ checking the move is valid
                // That way, invalid moves don't try to clear the flag.
                self.board.clear_just_lunged();
                self.board.castle(color, side);
            }
            Normal => {
                self.board.clear_just_lunged();
                self.board.move_piece(start, end)
            }
            Capture => {
                self.board.clear_just_lunged();
                let captured_piece = self
                    .get(end)
                    .0
                    .expect("Expected piece to be present at end on a capture!");
                match captured_piece.color {
                    Black => self.dead_black.push(captured_piece),
                    White => self.dead_white.push(captured_piece),
                }

                self.board.move_piece(start, end)
            }
            Lunge => {
                // Clear the old lunge flag before the new one
                self.board.clear_just_lunged();
                self.board.lunge(start);
            }
            EnPassant(side) => {
                use BoardSide::*;
                let captured_coord = match side {
                    Kingside => BoardCoord(start.0 + 1, start.1),
                    Queenside => BoardCoord(start.0 - 1, start.1),
                };

                // Add the captured pawn to the dead piece list
                let captured_piece = self
                    .get(captured_coord)
                    .0
                    .expect(&format!("Expected pawn at {:?}", captured_coord));
                match captured_piece.color {
                    Black => self.dead_black.push(captured_piece),
                    White => self.dead_white.push(captured_piece),
                }

                self.board.enpassant(start, end);
                // Don't clear the lunge flag until _after_ we check for enpassant
                // (otherwise we will never be able to :P)
                self.board.clear_just_lunged();
            }
        }

        #[cfg(feature = "perf")]
        drop(guard);

        if self.need_promote().is_none() {
            self.current_player = match self.current_player {
                White => Black,
                Black => White,
            };
        }

        #[cfg(feature = "perf")]
        flame::start("checkmate update");

        // Update the checkmate status
        self.checkmate = self.board.checkmate_state(self.current_player);

        #[cfg(feature = "perf")]
        flame::end("checkmate update");
    }

    pub fn need_promote(&self) -> Option<BoardCoord> {
        return self.board.pawn_needs_promotion();
    }

    /// Checks if the promotion is legal. This function returns Err if there is
    /// no piece to promote to or if the promotion would be illegal.
    pub fn check_promote(&self, coord: BoardCoord, piece: PieceType) -> Result<(), &'static str> {
        // TODO: remove this if?
        if self.need_promote().is_none() {
            return Err("No pawn needs to be promoted at this time");
        }

        self.board.check_promote(coord, piece)
    }

    /// Promotes the pawn. This function panics if the promotion is illegal. This
    /// function also handles updating the checkmate state and current player
    pub fn promote(&mut self, coord: BoardCoord, piece: PieceType) {
        debug_assert!(self.check_promote(coord, piece).is_ok());

        self.board.promote_pawn(coord, piece);

        use Color::*;
        self.current_player = match self.current_player {
            White => Black,
            Black => White,
        };

        // Update the checkmate status
        self.checkmate = self.board.checkmate_state(self.current_player);
    }

    /// Return the list of valid moves for current player at the coordinate
    pub fn get_move_list(&self, coord: BoardCoord) -> Vec<BoardCoord> {
        self.board.get_move_list(coord, self.current_player)
    }

    pub fn game_over(&self) -> bool {
        match self.board.checkmate_state(self.current_player) {
            CheckmateState::Normal | CheckmateState::Check => false,
            CheckmateState::Checkmate
            | CheckmateState::Stalemate
            | CheckmateState::InsuffientMaterial => true,
        }
    }

    /// Try to get the `Tile` at `coord`. This function returns `None` if `coord`
    /// would be off the board.
    pub fn get(&self, coord: BoardCoord) -> &Tile {
        self.board.get(coord)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckmateState {
    Normal,
    Check,
    Checkmate,
    Stalemate,
    InsuffientMaterial,
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
                let x = j;
                let y = i + (8 - str_board.len());
                let tile = match piece.chars().nth(1).unwrap() {
                    'P' => Tile::new(color, Pawn { just_lunged: false }),
                    'N' => Tile::new(color, Knight),
                    'B' => Tile::new(color, Bishop),
                    'R' => Tile::new(color, Rook),
                    'Q' => Tile::new(color, Queen),
                    'K' => Tile::new(color, King),
                    _ => Tile::blank(),
                };
                board.board[y][x] = tile;
            }
        }
        board
    }

    /// Lunges the pawn located at `coord` two spaces forward. This function
    /// always moves the pawn, even if would not actually be legal to do so in
    /// a real game, so you should check the move first.
    pub fn lunge(&mut self, coord: BoardCoord) {
        let pawn = self.get_mut(coord);
        pawn.set_just_lunged(true);
        let direction = pawn.0.expect("Expected a pawn").color.direction();
        let end = BoardCoord(coord.0, coord.1 + 2 * direction);
        self.move_piece(coord, end);
    }

    /// Moves the piece located at `start` to `end`. This function always moves
    /// the piece, even if it would not be actually legal to do so in a real
    /// game, so you should check the move first with `check_move`
    pub fn move_piece(&mut self, start: BoardCoord, end: BoardCoord) {
        let mut moved_piece = *self.get(start);
        moved_piece.set_moved(true);
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
            return Err("Can't move an empty tile");
        }

        let start_piece = start_piece.0.unwrap();
        if start_piece.color != player {
            return Err("Can't move a piece that isn't yours");
        }

        // TODO: use threatens/write a "can_move_to" function for this, which is
        // way better than doing a ton of allocations & redundant checks.
        let mut valid_end_spots = MoveList::reserved();
        get_move_list_full(self, player, start, &mut valid_end_spots);

        if valid_end_spots.0.contains(&end) {
            Ok(())
        } else {
            Err("Can't move a piece there")
        }
    }

    /// Return the list of valid places the piece at `coord` can move for the
    /// given `player`. Note that the returned vector is empty if any of the
    /// following are true.
    /// - `coord` is off the board
    /// - `coord` refers to an empty tile
    /// - `coord` refers to a piece that is the opposite color of `current_player`
    /// - `coord` refers to a piece that has nowhere to move
    /// Also note that this function DOES check if the move would place the
    /// king into check.
    /// This function also DOES check if the King can castle.
    fn get_move_list(&self, coord: BoardCoord, player: Color) -> Vec<BoardCoord> {
        if !on_board(coord) {
            return vec![];
        }

        if self.pawn_needs_promotion().is_some() {
            return vec![];
        }

        // If not a piece
        let tile = self.get(coord).0;
        if tile.is_none() {
            return vec![];
        }

        // If not a piece of the player's color
        let piece = tile.unwrap();
        if piece.color != player {
            return vec![];
        }

        let mut list = MoveList::reserved();
        get_move_list_full(&self, player, coord, &mut list);
        match piece.piece {
            PieceType::King => {
                list.0.append(&mut self.castle_locations(player));
            }
            PieceType::Pawn { .. } => list.0.append(&mut self.enpassant_locations(player, coord)),
            _ => {}
        }

        list.0
    }

    /// Return the list of all valid moves that can be made by a player
    #[cfg_attr(feature = "perf", flame)]
    pub fn get_all_moves(&self, player: Color) -> Vec<(BoardCoord, BoardCoord)> {
        // TODO: this is probably hilariously inefficent
        let mut moves = vec![];

        for i in ROWS {
            for j in COLS {
                let start = BoardCoord(i, j);
                let this_move_list = self.get_move_list(start, player);
                let mut move_pairs = this_move_list.into_iter().map(|end| (start, end)).collect();
                moves.append(&mut move_pairs);
            }
        }
        moves
    }

    /// Castle the King of `color`. This function does not check if
    /// doing so would actually be legal to do so in a real game, so you should
    /// check the castle first with `can_castle`
    fn castle(&mut self, color: Color, side: BoardSide) {
        use BoardSide::*;
        use Color::*;
        let first_rank = match color {
            White => 0,
            Black => 7,
        };

        let king_start = BoardCoord(4, first_rank);
        let king_end = match side {
            Queenside => BoardCoord(2, first_rank),
            Kingside => BoardCoord(6, first_rank),
        };

        let rook_start = match side {
            Queenside => BoardCoord(0, first_rank),
            Kingside => BoardCoord(7, first_rank),
        };

        let rook_end = match side {
            Queenside => BoardCoord(3, first_rank),
            Kingside => BoardCoord(5, first_rank),
        };

        self.move_piece(king_start, king_end);
        self.move_piece(rook_start, rook_end);
    }

    /// Return a list of locations that the king may castle to.
    pub fn castle_locations(&self, color: Color) -> Vec<BoardCoord> {
        let mut castle_locs = vec![];
        let first_rank = match color {
            Color::White => 0,
            Color::Black => 7,
        };

        if self.can_castle(color, BoardSide::Queenside).is_ok() {
            castle_locs.push(BoardCoord(2, first_rank));
        }

        if self.can_castle(color, BoardSide::Kingside).is_ok() {
            castle_locs.push(BoardCoord(6, first_rank));
        }

        castle_locs
    }

    /// Returns true if the king can castle to the rook indicated
    /// Note that a king may only castle if all the following are true
    /// - The king has not moved
    /// - The rook to castle with has not moved
    /// - There are no pieces in between the rook and king
    /// - The king is not in check
    /// - The king does not pass through any square attacked by an enemy piece
    /// - The kind does not end up in check
    /// Note that this means that, in the board below, the dots must both
    /// not be attacked and be empty tiles.
    ///   R . . . K . . R
    ///   0 1 2 3 4 5 6 7
    /// queenside kingside
    fn can_castle(&self, color: Color, side: BoardSide) -> Result<(), &'static str> {
        let first_rank = match color {
            Color::White => 0,
            Color::Black => 7,
        };

        // if the king hasn't moved, we expect it to be at (4, 0) or (4, 7)
        let king_coord = BoardCoord(4, first_rank);

        // King is actually a king and has not moved
        let king = self.get(king_coord);
        match king.0 {
            Some(Piece {
                piece: PieceType::King,
                color: c,
                has_moved: false,
            }) if c == color => (),
            _ => {
                return Err("Can't castle, king is not an unmoved king");
            }
        }

        let king_is_safe = self.is_square_safe(color, &king_coord);
        if !king_is_safe {
            return Err("Can't castle, king is in check");
        }

        let side = match side {
            BoardSide::Queenside => 0,
            BoardSide::Kingside => 7,
        };
        let rook_coord = BoardCoord(side, first_rank);

        // rook_coord is actually a rook that has not moved
        let rook = self.get(rook_coord);
        match rook.0 {
            Some(Piece {
                piece: PieceType::Rook,
                color: c,
                has_moved: false,
            }) if c == color => (),
            _ => {
                return Err("Can't castle, rook is not an unmoved rook");
            }
        }

        let king_passes_through = match rook_coord.0 {
            // Queenside castle
            0 => vec![BoardCoord(2, first_rank), BoardCoord(3, first_rank)],
            // Kingside castle
            7 => vec![BoardCoord(5, first_rank), BoardCoord(6, first_rank)],
            _ => unreachable!(),
        };

        // All interveening tiles that the king passes through are empty and not
        // under attack.
        for square in king_passes_through {
            let tile_safe = self.is_square_safe(color, &square);
            let tile_empty = self.get(square).0.is_none();
            if !tile_safe || !tile_empty {
                return Err("Can't castle, at least one square not empty or safe");
            }
        }

        // Additionally, if we are castling queenside, we need the tile just
        // to the left of the rook to be empty.
        let is_queenside = rook_coord == BoardCoord(0, first_rank);
        let queenside_space_empty = self.get(BoardCoord(1, first_rank)).0.is_none();
        if is_queenside && !queenside_space_empty {
            return Err("Can't castle, queenside rook space not empty");
        }
        Ok(())
    }

    /// Returns Ok if the move is a valid en passant. A player may en passant
    /// only if the following are true
    /// 1. The opposing player, in the previous turn, has just moved their pawn
    /// two spaces (aka: the pawn has `just_lunged` set to true)
    /// 2. The player has a capturing pawn on the fifth rank AND is in a file
    /// adjacent to the moved pawn
    /// 3. The player captures the opposing pawn on this turn.
    fn check_enpassant(
        &self,
        player: Color,
        capturing_pawn: BoardCoord,
        direction: BoardSide,
    ) -> Result<(), &'static str> {
        use BoardSide::*;
        // We expect that start and end are diagonal from each other
        // and that the captured pawn is "one rank behind" the the end location
        // where "behind" is relative to the player capturing.

        let captured_pawn_coord = match direction {
            Queenside => (capturing_pawn.0 - 1, capturing_pawn.1),
            Kingside => (capturing_pawn.0 + 1, capturing_pawn.1),
        };
        let captured_pawn_coord = BoardCoord::new(captured_pawn_coord)?;

        let capturing_pawn = self.get(capturing_pawn);
        let captured_pawn = self.get(captured_pawn_coord);

        match capturing_pawn.0 {
            Some(Piece {
                color: c,
                piece: PieceType::Pawn { .. },
                has_moved: _,
            }) if c == player => (),
            _ => return Err("Capturing piece must be a pawn of the player's color"),
        }

        match captured_pawn.0 {
            Some(Piece {
                color: c,
                piece: PieceType::Pawn { just_lunged: true },
                has_moved: _,
            }) if c != player => (),
            _ => {
                return Err("Captured piece must be a pawn that just lunged of the opposite color");
            }
        }

        Ok(())
    }

    /// Performs an en passant. This function does not check if this would be
    /// an actually valid en passant, so you should do that by calling
    /// `check_enpassant`. This function moves the piece located at `start`
    /// to `end` and kills whatever piece is just behind the tile at `end`
    /// where "behind" is defined relative to the piece being moved.
    fn enpassant(&mut self, start: BoardCoord, end: BoardCoord) {
        let direction = self.get(start).0.unwrap().color.direction();
        let captured_pawn = BoardCoord(end.0, end.1 - direction);

        self.move_piece(start, end);
        self.set(captured_pawn, Tile::blank());
    }

    /// Return a list of locations a pawn can move to using enpassant.
    /// (really this always just returns one element or no elements)
    fn enpassant_locations(&self, player: Color, capturing_pawn: BoardCoord) -> Vec<BoardCoord> {
        let mut locations = vec![];
        if self
            .check_enpassant(player, capturing_pawn, BoardSide::Queenside)
            .is_ok()
        {
            locations.push(BoardCoord(
                capturing_pawn.0 - 1,
                capturing_pawn.1 + player.direction(),
            ));
        }
        if self
            .check_enpassant(player, capturing_pawn, BoardSide::Kingside)
            .is_ok()
        {
            locations.push(BoardCoord(
                capturing_pawn.0 + 1,
                capturing_pawn.1 + player.direction(),
            ));
        }
        locations
    }

    fn clear_just_lunged(&mut self) {
        for i in ROWS {
            for j in COLS {
                self.get_mut(BoardCoord(j, i)).set_just_lunged(false);
            }
        }
    }

    #[cfg_attr(feature = "perf", flame)]
    /// Returns true if no piece of the opposite color threatens the square.
    fn is_square_safe(&self, color: Color, target: &BoardCoord) -> bool {
        // TODO: this is hilariously inefficient
        // Instead of checking for if a piece threatens the square, instead
        // check that the square has no pieces that could threaten it
        // AKA: fan out in a queen shape (+ and x) and check for {rook, bishop, queen}
        // then also check surroudning squares for knights, kings, pawns

        fn first_nonempty(board: &Board, iter: impl Iterator<Item = BoardCoord>) -> Option<Piece> {
            for coord in iter {
                if !on_board(coord) {
                    break;
                }

                if let Some(piece) = board.get(coord).0 {
                    return Some(piece);
                }
            }
            None
        }

        fn is_enemy_rook_or_queen(color: Color, piece: Option<Piece>) -> bool {
            if let Some(piece) = piece {
                return piece.color == color.opposite()
                    && (piece.piece == PieceType::Rook || piece.piece == PieceType::Queen);
            }
            false
        }

        fn is_enemy_bishop_or_queen(color: Color, piece: Option<Piece>) -> bool {
            if let Some(piece) = piece {
                return piece.color == color.opposite()
                    && (piece.piece == PieceType::Bishop || piece.piece == PieceType::Queen);
            }
            false
        }

        // Check for rooks, queens (+)
        let up_los = (1..8).map(|i| BoardCoord(target.0, target.1 + i));
        if is_enemy_rook_or_queen(color, first_nonempty(self, up_los)) {
            return false;
        }
        let down_los = (1..8).map(|i| BoardCoord(target.0, target.1 - i));
        if is_enemy_rook_or_queen(color, first_nonempty(self, down_los)) {
            return false;
        }
        let right_los = (1..8).map(|i| BoardCoord(target.0 + i, target.1));
        if is_enemy_rook_or_queen(color, first_nonempty(self, right_los)) {
            return false;
        }
        let left_los = (1..8).map(|i| BoardCoord(target.0 - i, target.1));
        if is_enemy_rook_or_queen(color, first_nonempty(self, left_los)) {
            return false;
        }

        // Check for bishops, queens (x)
        let up_right_los = (1..8).map(|i| BoardCoord(target.0 + i, target.1 + i));
        if is_enemy_bishop_or_queen(color, first_nonempty(self, up_right_los)) {
            return false;
        }
        let up_left_los = (1..8).map(|i| BoardCoord(target.0 - i, target.1 + i));
        if is_enemy_bishop_or_queen(color, first_nonempty(self, up_left_los)) {
            return false;
        }
        let down_right_los = (1..8).map(|i| BoardCoord(target.0 + i, target.1 - i));
        if is_enemy_bishop_or_queen(color, first_nonempty(self, down_right_los)) {
            return false;
        }
        let down_left_los = (1..8).map(|i| BoardCoord(target.0 - i, target.1 - i));
        if is_enemy_bishop_or_queen(color, first_nonempty(self, down_left_los)) {
            return false;
        }

        // Check for kings
        let delta_coords = [
            (1, 0),
            (-1, 0),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
            (0, 1),
            (0, -1),
        ];
        for delta in delta_coords.iter() {
            let (x, y) = (target.0 + delta.0, target.1 + delta.1);
            if on_board_i8((x, y)) {
                let check_coord = BoardCoord(x, y);
                if self.get(check_coord).is(color.opposite(), PieceType::King) {
                    return false;
                }
            }
        }

        // Check for knights
        let delta_coords = [
            (1, 2),
            (1, -2),
            (-1, 2),
            (-1, -2),
            (2, 1),
            (2, -1),
            (-2, 1),
            (-2, -1),
        ];
        for delta in delta_coords.iter() {
            let (x, y) = (target.0 + delta.0, target.1 + delta.1);
            if on_board_i8((x, y)) {
                let check_coord = BoardCoord(x, y);
                if self
                    .get(check_coord)
                    .is(color.opposite(), PieceType::Knight)
                {
                    return false;
                }
            }
        }

        // Check for pawn
        let delta_coords = match color {
            Color::White => [(1, 1), (-1, 1)],
            Color::Black => [(1, -1), (-1, -1)],
        };
        for delta in delta_coords.iter() {
            let (x, y) = (target.0 + delta.0, target.1 + delta.1);
            if on_board_i8((x, y)) {
                let check_coord = BoardCoord(x, y);
                if self
                    .get(check_coord)
                    .is(color.opposite(), PieceType::Pawn { just_lunged: false })
                {
                    return false;
                }
            }
        }

        true
    }

    /// Returns if the player is currently in checkmate
    #[cfg_attr(feature = "perf", flame)]
    fn checkmate_state(&self, player: Color) -> CheckmateState {
        use CheckmateState::*;
        match (
            self.has_legal_moves(player),
            self.is_in_check(player),
            self.insuffient_material(),
        ) {
            (false, false, false) => Stalemate,
            (false, true, false) => Checkmate,
            (true, false, false) => Normal,
            (true, true, false) => Check,
            (_, _, true) => InsuffientMaterial,
        }
    }

    fn insuffient_material(&self) -> bool {
        // TODO: you should implement the one harder cases where checkmate is impossible
        /*
        From https://en.wikipedia.org/wiki/Draw_(chess)
        Impossibility of checkmate – if a position arises in which neither
        player could possibly give checkmate by a series of legal moves, the
        game is a draw ("dead position"). This is usually because there is
        insufficient material left, but it is possible in other positions too.
        Combinations with insufficient material to checkmate are:
            king versus king
            king and bishop versus king
            king and knight versus king
            // TODO: this one i haven't implemented this yet beause it is hard
            king and bishop versus king and bishop with the bishops on the same color.
        */
        let mut num_knights = 0;
        let mut num_bishops = 0;
        // let mut black_has_black_square_bishops = 0;
        // let mut num_white_square_bishops

        use PieceType::*;
        for i in ROWS {
            for j in COLS {
                let coord = BoardCoord(i, j);
                let tile = self.get(coord).0;
                match tile {
                    None => continue,
                    Some(piece) => match piece.piece {
                        Pawn { .. } | Rook | Queen => return false,
                        Knight => num_bishops += 1,
                        Bishop => num_knights += 1,
                        // we will assume that the board always has two kings
                        // if it does not have two kings then there are probably
                        // bigger problems going on
                        King => continue,
                    },
                }
            }
        }

        match (num_knights, num_bishops) {
            (0, 0) | (0, 1) | (1, 0) => true,
            _ => false,
        }
    }

    fn is_in_check(&self, player: Color) -> bool {
        !self.is_square_safe(player, &self.get_king(player).unwrap())
    }

    #[cfg_attr(feature = "perf", flame)]
    fn has_legal_moves(&self, player: Color) -> bool {
        // TODO: this is also HILARIOUSLY INEFFICENT
        for i in ROWS {
            for j in COLS {
                let coord = BoardCoord(j, i);
                let tile = self.get(coord);

                if tile.is_color(player) && self.piece_has_legal_moves(player, coord) {
                    return true;
                }
            }
        }
        false
    }

    fn piece_has_legal_moves(&self, player: Color, coord: BoardCoord) -> bool {
        // TODO just directly check if a piece can move, instead of calculating all possible moves (overkill)
        let mut out = MoveList::reserved();
        get_move_list_full(self, player, coord, &mut out);
        !out.0.is_empty()
    }

    /// Returns Some(BoardCoord) if there is a pawn in the last rank that needs
    /// to be promoted. Otherwise, this functino returns None.
    pub fn pawn_needs_promotion(&self) -> Option<BoardCoord> {
        for i in ROWS {
            let black_coord = BoardCoord(i, 0);
            if self
                .get(black_coord)
                .is(Color::Black, PieceType::Pawn { just_lunged: false })
            {
                return Some(black_coord);
            }

            let white_coord = BoardCoord(i, 7);
            if self
                .get(white_coord)
                .is(Color::White, PieceType::Pawn { just_lunged: false })
            {
                return Some(white_coord);
            }
        }
        None
    }

    /// Checks if the Tile at coord can be promoted to piece.
    /// A pawn may be promoted if it is
    /// - a pawn
    /// - on the last rank of its side
    /// - being promoted to a piece that is not a pawn or a king
    pub fn check_promote(&self, coord: BoardCoord, piece: PieceType) -> Result<(), &'static str> {
        let pawn = self.get(coord);
        let color = match coord.1 {
            0 => Color::Black,
            7 => Color::White,
            _ => {
                return Err("Can only promote a pawn at the last rank of the board");
            }
        };

        if !pawn.is(color, PieceType::Pawn { just_lunged: false }) {
            return Err("Can only promote a pawn of opposite color at this position.");
        }

        match piece {
            PieceType::Pawn { .. } => Err("Can not promote to a pawn"),
            PieceType::King => Err("Can not promote to a king"),
            _ => Ok(()),
        }
    }

    // Promote the pawn located at coord to the piece of PieceType
    // Note that this function does not actually check if the promotion would be
    // valid.
    pub fn promote_pawn(&mut self, coord: BoardCoord, piece: PieceType) {
        let tile = self.get_mut(coord);
        *tile = Tile(Some(Piece {
            color: tile.0.unwrap().color,
            piece,
            has_moved: true,
        }));
    }

    /// Gets the piece located at the coordinates
    pub fn get(&self, BoardCoord(x, y): BoardCoord) -> &Tile {
        // i promise very very hard that this i8 is, in fact, in the range 0-7
        &self.board[(7 - y) as usize][x as usize]
    }

    /// Gets mutably the piece located at the coordinates
    pub fn get_mut(&mut self, BoardCoord(x, y): BoardCoord) -> &mut Tile {
        // i promise very very hard that this i8 is, in fact, in the range 0-7
        &mut self.board[(7 - y) as usize][x as usize]
    }

    /// Sets the piece located at the coordinates
    fn set(&mut self, BoardCoord(x, y): BoardCoord, piece: Tile) {
        // i promise very very hard that this i8 is, in fact, in the range 0-7
        self.board[(7 - y) as usize][x as usize] = piece;
    }

    /// Attempts to return the coordinates the king of the specified color
    /// TODO: MAKE MORE EFFICENT?
    pub fn get_king(&self, color: Color) -> Option<BoardCoord> {
        for i in ROWS {
            for j in COLS {
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

impl MoveList {
    fn reserved() -> MoveList {
        // A queen on an empty board has up to 28 possible moves it can make
        // (7 for each of its 4 lines of sight)
        MoveList(Vec::with_capacity(28))
    }
}

/// Return a MoveList of the piece located at `coord`.
/// This function DOES check if a move made by the King would put the King into
/// check and DOES NOT check if the King can castle. It also DOES NOT check if
/// a pawn needs to be promoted.
#[cfg_attr(feature = "perf", flame)]
fn get_move_list_full(board: &Board, player: Color, coord: BoardCoord, out: &mut MoveList) {
    get_move_list_ignore_check(&board, coord, out);
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
    let king_coord = board.get_king(player).unwrap();
    out.0.retain(|&attempted_move| {
        let mut moved_board = board.clone();
        moved_board.move_piece(coord, attempted_move);

        // update king coord if we just moved the king
        let king_coord = if coord == king_coord {
            attempted_move
        } else {
            king_coord
        };

        moved_board.is_square_safe(player, &king_coord)
    });
}

/// Return a MoveList of the piece located at `coord`. This function assumes
/// that the player to move is whatever the color of the piece at `coord` is.
/// This function does NOT check if a move made by the King would put the King into
/// check.
#[cfg_attr(feature = "perf", flame)]
fn get_move_list_ignore_check(board: &Board, coord: BoardCoord, out: &mut MoveList) {
    let piece = board.get(coord).0;
    use PieceType::*;
    match piece {
        None => (),
        Some(piece) => match piece.piece {
            Pawn { .. } => check_pawn(board, coord, piece.color, out),
            Knight | King => {
                check_jump_piece(board, coord, piece.color, get_move_deltas(piece.piece), out)
            }

            Bishop | Rook | Queen => {
                check_line_of_sight_piece(board, coord, piece.color, get_los(piece.piece), out)
            }
        },
    }
}

/// Get a list of the locations the pawn at `pos` can move to. The pawn's color
/// is assumed to be `color`. Note that this function doesn't actually check if
/// there is a pawn at `pos`.
fn check_pawn(board: &Board, pos: BoardCoord, color: Color, out: &mut MoveList) {
    // Check forward space if it can be moved into.
    let forwards = BoardCoord(pos.0, pos.1 + color.direction());
    let mut could_do_first_move = false;
    if on_board(forwards) {
        if board.get(forwards).0.is_none() {
            out.0.push(forwards);
            could_do_first_move = true;
        }
    }

    // if piece has not moved yet, check for double movement
    let double_move = BoardCoord(pos.0, pos.1 + color.direction() * 2);
    let pawn = board.get(pos).0.unwrap();
    if on_board(double_move) {
        if board.get(double_move).0.is_none() && !pawn.has_moved && could_do_first_move {
            out.0.push(double_move);
        }
    }

    // Check diagonal spaces if they can be attacked.
    for &diagonal in &[
        BoardCoord(pos.0 + 1, pos.1 + color.direction()),
        BoardCoord(pos.0 - 1, pos.1 + color.direction()),
    ] {
        if on_board(diagonal) {
            match board.get(diagonal).0 {
                Some(piece) if piece.color != color => out.0.push(diagonal),
                _ => {}
            }
        }
    }
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
    out: &mut MoveList,
) {
    for delta in move_deltas {
        let end_pos = BoardCoord(pos.0 + delta.0, pos.1 + delta.1 * color.direction());
        if !on_board(end_pos) {
            continue;
        }
        // A piece may actually move to end_pos if the location is unoccupied
        // or contains a piece of the opposite color.
        match board.get(end_pos).0 {
            Some(piece) if piece.color != color => out.0.push(end_pos),
            None => out.0.push(end_pos),
            _ => {}
        }
    }
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
    line_of_sights: impl Iterator<Item = LosIterator>,
    out: &mut MoveList,
) {
    for los in line_of_sights {
        for delta in los {
            let end_pos = BoardCoord(pos.0 + delta.0, pos.1 + delta.1 * color.direction());

            if !on_board(end_pos) {
                break;
            }

            let end_piece = board.get(end_pos).0;
            if end_piece.is_none() {
                out.0.push(end_pos);
            } else if let Some(piece) = end_piece {
                if piece.color != color {
                    out.0.push(end_pos);
                }
                break;
            }
        }
    }
}

type LosIterator = std::iter::Map<std::ops::Range<i8>, fn(i8) -> BoardCoord>;

/// Returns LoS for Rooks, Bishops, and Queens. Panics on other PieceTypes.
fn get_los(piece: PieceType) -> Box<dyn Iterator<Item = LosIterator>> {
    use PieceType::*;
    match piece {
        Rook => Box::new(get_los_rook()),
        Bishop => Box::new(get_los_bishop()),
        Queen => Box::new(get_los_rook().chain(get_los_bishop())),
        Pawn { .. } | Knight | King => panic!("Expected a Rook, Bishop, or Queen. Got {:?}", piece),
    }
}

// needed to force the iterators below to not be closures and instead be
// boring function types
fn boring<T, U>(f: fn(T) -> U) -> fn(T) -> U {
    f
}

fn get_los_rook() -> impl Iterator<Item = LosIterator> {
    let los_right = (1..8).map(boring(|i| BoardCoord(i, 0)));
    let los_left = (1..8).map(boring(|i: i8| BoardCoord(-i, 0)));
    let los_up = (1..8).map(boring(|i| BoardCoord(0, i)));
    let los_down = (1..8).map(boring(|i: i8| BoardCoord(0, -i)));
    use std::iter::once;
    once(los_right)
        .chain(once(los_left))
        .chain(once(los_up))
        .chain(once(los_down))
}

fn get_los_bishop() -> impl Iterator<Item = LosIterator> {
    let los_up_right = (1..8).map(boring(|i| BoardCoord(i, i)));
    let los_up_left = (1..8).map(boring(|i: i8| BoardCoord(-i, i)));
    let los_down_right = (1..8).map(boring(|i: i8| BoardCoord(i, -i)));
    let los_down_left = (1..8).map(boring(|i: i8| BoardCoord(-i, -i)));

    use std::iter::once;
    once(los_up_right)
        .chain(once(los_up_left))
        .chain(once(los_down_right))
        .chain(once(los_down_left))
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
        Pawn { .. } | Rook | Bishop | Queen => {
            panic!("Expected a Knight or a King, got {:?}", piece)
        }
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
        for i in ROWS {
            for j in COLS {
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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum BoardSide {
    Queenside,
    Kingside,
}

enum MoveType {
    Castle(Color, BoardSide),
    Normal,
    Capture,
    // A "queenside" enpassant is defined as the attacking pawn moving towards the
    // queen's side of the board (the x coordinate decreases), and vice versa for
    // "kingside" enpassant
    EnPassant(BoardSide),
    Lunge,
}

/// Returns what kind of move this is, either normal, a castle, or an en passant
/// Note that a castle is expected to be a move starting on the king, so these
/// are the only following castles
fn move_type(board: &Board, start: BoardCoord, end: BoardCoord) -> MoveType {
    use BoardSide::*;
    use MoveType::*;
    let tile = board.get(start);
    if tile.0.is_none() {
        return MoveType::Normal;
    }
    let piece = tile.0.unwrap();

    let delta = match piece.color {
        Color::White => (end.0 - start.0, end.1 - start.1),
        Color::Black => (end.0 - start.0, -(end.1 - start.1)),
    };

    let empty_end_pos = board.get(end).0.is_none();

    match (piece.piece, delta, empty_end_pos) {
        (PieceType::King, (-2, 0), _) => Castle(piece.color, Queenside),
        (PieceType::King, (2, 0), _) => Castle(piece.color, Kingside),
        (PieceType::Pawn { .. }, (0, 2), _) => Lunge,
        // An enpassant move will always attempt to move into an empty square
        // while a capture will move onto a nonempty square
        (PieceType::Pawn { .. }, (-1, 1), true) => EnPassant(Queenside),
        (PieceType::Pawn { .. }, (1, 1), true) => EnPassant(Kingside),
        (_, _, false) => Capture,
        _ => Normal,
    }
}

#[derive(Debug, Copy, Clone)]
pub enum BasicAction {
    Move { start: BoardCoord, end: BoardCoord },
    Remove { coord: BoardCoord },
    Change { coord: BoardCoord, new_piece: Piece },
}

pub fn basic_actions(board: &Board, start: BoardCoord, end: BoardCoord) -> Vec<BasicAction> {
    match move_type(board, start, end) {
        MoveType::Castle(color, board_side) => {
            let (king_start, king_end) = (start, end);
            let (rook_start, rook_end) = match (color, board_side) {
                (Color::White, BoardSide::Queenside) => (BoardCoord(0, 0), BoardCoord(3, 0)),
                (Color::White, BoardSide::Kingside) => (BoardCoord(7, 0), BoardCoord(5, 0)),
                (Color::Black, BoardSide::Queenside) => (BoardCoord(0, 7), BoardCoord(3, 7)),
                (Color::Black, BoardSide::Kingside) => (BoardCoord(7, 7), BoardCoord(5, 7)),
            };
            vec![
                BasicAction::Move {
                    start: king_start,
                    end: king_end,
                },
                BasicAction::Move {
                    start: rook_start,
                    end: rook_end,
                },
            ]
        }
        MoveType::Normal => vec![BasicAction::Move { start, end }],
        MoveType::Capture => vec![
            BasicAction::Remove { coord: end },
            BasicAction::Move { start, end },
        ],
        MoveType::EnPassant(side) => {
            let captured_pawn = match side {
                BoardSide::Queenside => BoardCoord(start.0 - 1, start.1),
                BoardSide::Kingside => BoardCoord(start.0 + 1, start.1),
            };
            vec![
                BasicAction::Remove {
                    coord: captured_pawn,
                },
                BasicAction::Move { start, end },
            ]
        }
        MoveType::Lunge => vec![BasicAction::Move { start, end }],
    }
}

/// Newtype wrapper for `Option<Piece>`. `Some(piece)` indicates that a piece is
/// in the tile, and `None` indicates that the tile is empty. Used in `Board`.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Tile(pub Option<Piece>);

impl Tile {
    pub fn new(color: Color, piece: PieceType) -> Tile {
        Tile(Some(Piece {
            color,
            piece,
            has_moved: false,
        }))
    }

    pub fn blank() -> Tile {
        Tile(None)
    }

    /// Set the `has_moved` field to `set`. If this `Tile` is `None`, nothing happens
    pub fn set_moved(&mut self, set: bool) {
        if let Some(piece) = &mut self.0 {
            piece.has_moved = set;
        } else {
            panic!(
                "Expected Tile to be Some piece, got None instead. set = {}",
                set
            );
        }
    }

    /// Set `just_lunged` flag on Pawn if the tile is a pawn. Else, do nothing.
    pub fn set_just_lunged(&mut self, set: bool) {
        if let Some(piece) = &mut self.0 {
            if let PieceType::Pawn { just_lunged } = &mut piece.piece {
                *just_lunged = set;
            }
        }
    }

    /// Returns true if the `Tile` actually has a piece and
    /// `color` and `piece_type` match. Note: the `just_lunged` flag on
    /// the Pawn piecetype is ignored.
    pub fn is(&self, color: Color, piece_type: PieceType) -> bool {
        match self.0 {
            None => false,
            Some(piece) => {
                // use std::mem:discriminant here because we don't care if PieceType::Pawn is just lunged or not
                piece.color == color
                    && std::mem::discriminant(&piece.piece) == std::mem::discriminant(&piece_type)
            }
        }
    }

    /// Returns true if the `Tile` actually has a piece and
    /// `color` matches the color of the piece.
    pub fn is_color(&self, color: Color) -> bool {
        match self.0 {
            None => false,
            Some(piece) => piece.color == color,
        }
    }

    /// Return a string representation of this Tile (currently uses unicode
    /// chess piece characters)
    pub fn as_str(&self) -> &'static str {
        match self.0 {
            None => "",
            Some(piece) => piece.as_str(),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece {
    pub color: Color,
    pub piece: PieceType,
    has_moved: bool,
}

impl Piece {
    pub fn as_str(&self) -> &'static str {
        self.piece.as_str()
    }
}

/// The available player colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    pub fn opposite(&self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
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
/// `Pawn` has a `bool` associated with that is true if the piece has just
/// lunged (moved two spaces) on the previous turn.S
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceType {
    Pawn { just_lunged: bool },
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceType {
    pub fn as_str(&self) -> &'static str {
        use PieceType::*;
        match self {
            Pawn { .. } => PAWN_STR,
            Knight => KNIGHT_STR,
            Bishop => BISHOP_STR,
            Rook => ROOK_STR,
            Queen => QUEEN_STR,
            King => KING_STR,
        }
    }
}

impl fmt::Display for PieceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use PieceType::*;
        match self {
            Pawn { .. } => write!(f, "P"),
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
    use super::*;

    /// STRING BOARDS

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
            Tile::new(Color::White, PieceType::Pawn { just_lunged: false }),
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
            Tile::new(Color::White, PieceType::Pawn { just_lunged: false }),
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
            Tile::new(Color::White, PieceType::Pawn { just_lunged: false }),
        );
        println!("real\n{}\nexpected\n{}", board, expected);
        assert_eq!(board, expected);
    }

    // MOVEMENT

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
            ".. ## ..",
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
            ".. ## ..",
        ];
        assert_valid_movement(board, (1, 2), expected);
    }

    #[test]
    fn test_pawn_single_move() {
        #[rustfmt::skip]
        let board = vec![
            ".. .. ..",
            ".. .. ..",
            ".. WP ..",
            ".. .. ..",
        ];
        let mut board = Board::from_string_vec(board);
        board.get_mut(BoardCoord(1, 1)).set_moved(true);
        #[rustfmt::skip]
        let expected = vec![
            ".. .. ..",
            ".. ## ..",
            ".. WP ..",
            ".. .. ..",
        ];
        assert_valid_movement_board(board, (1, 1), expected);
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

    // CASTLING TESTS

    const WHITE_QUEENSIDE_ROOK: BoardCoord = BoardCoord(0, 0);

    #[test]
    fn test_castle_simple() {
        let board = vec![
            "BR .. .. .. BK .. .. BR",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            "WR .. .. .. WK .. .. WR",
        ];
        let board = Board::from_string_vec(board);
        assert!(board.can_castle(Color::White, BoardSide::Queenside).is_ok());
        assert!(board.can_castle(Color::White, BoardSide::Kingside).is_ok());
        assert!(board.can_castle(Color::Black, BoardSide::Queenside).is_ok());
        assert!(board.can_castle(Color::Black, BoardSide::Kingside).is_ok());
    }

    #[test]
    fn test_castle_blocked() {
        let board = vec![
            "BR .. BP .. BK .. BP BR",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            "WR WP .. .. WK .. WP WR",
        ];
        let board = Board::from_string_vec(board);
        assert!(board
            .can_castle(Color::White, BoardSide::Queenside)
            .is_err());
        assert!(board.can_castle(Color::White, BoardSide::Kingside).is_err());
        assert!(board
            .can_castle(Color::Black, BoardSide::Queenside)
            .is_err());
        assert!(board.can_castle(Color::Black, BoardSide::Kingside).is_err());
    }

    #[test]
    fn test_castle_check() {
        let board = vec![
            "BR .. .. .. BK .. .. BR",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. WR .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. BR .. .. ..",
            ".. .. .. .. .. .. .. ..",
            "WR .. .. .. WK .. .. WR",
        ];
        let board = Board::from_string_vec(board);
        assert!(board
            .can_castle(Color::White, BoardSide::Queenside)
            .is_err());
        assert!(board.can_castle(Color::White, BoardSide::Kingside).is_err());
        assert!(board
            .can_castle(Color::Black, BoardSide::Queenside)
            .is_err());
        assert!(board.can_castle(Color::Black, BoardSide::Kingside).is_err());
    }

    #[test]
    fn test_castle_threaten() {
        let board = vec![
            "BR .. .. .. BK .. .. BR",
            ".. .. .. .. .. .. .. ..",
            ".. .. WR .. .. .. .. WN",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. BP .. .. .. .. BN",
            "WR .. .. .. WK .. .. WR",
        ];
        let board = Board::from_string_vec(board);
        assert!(board
            .can_castle(Color::White, BoardSide::Queenside)
            .is_err());
        assert!(board.can_castle(Color::White, BoardSide::Kingside).is_err());
        assert!(board
            .can_castle(Color::Black, BoardSide::Queenside)
            .is_err());
        assert!(board.can_castle(Color::Black, BoardSide::Kingside).is_err());
    }

    #[test]
    fn test_castle_threaten_ok_for_rook() {
        let board = vec![
            "BR .. .. .. BK .. .. BR",
            ".. .. .. .. .. .. .. ..",
            ".. WR .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. BR .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            "WR .. .. .. WK .. .. WR",
        ];
        let board = Board::from_string_vec(board);
        assert!(board.can_castle(Color::White, BoardSide::Queenside).is_ok());
        assert!(board.can_castle(Color::Black, BoardSide::Queenside).is_ok());
    }

    #[test]
    fn test_castle_no_move() {
        let board = vec![
            "BR .. .. .. BK .. .. BR",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            "WR .. .. .. WK .. .. WR",
        ];
        let mut board = Board::from_string_vec(board);
        board.get_mut(WHITE_QUEENSIDE_ROOK).set_moved(true);
        board.get_mut(BoardCoord(4, 7)).set_moved(true);

        assert!(board
            .can_castle(Color::White, BoardSide::Queenside)
            .is_err());
        assert!(board.can_castle(Color::White, BoardSide::Kingside).is_ok());
        assert!(board
            .can_castle(Color::Black, BoardSide::Queenside)
            .is_err());
        assert!(board.can_castle(Color::Black, BoardSide::Kingside).is_err());
    }

    // EN PASSANT TESTS
    #[test]
    fn test_en_passant() {
        let board = vec![
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. BP .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. WP .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
        ];
        let mut board = Board::from_string_vec(board);
        board.lunge(BoardCoord(5, 6));
        assert!(board
            .check_enpassant(Color::White, BoardCoord(4, 4), BoardSide::Kingside)
            .is_ok());
    }

    #[test]
    fn test_en_passant2() {
        let board = vec![
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. BP .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            "WP .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
        ];
        let mut board = Board::from_string_vec(board);
        board.lunge(BoardCoord(0, 1));
        assert!(board
            .check_enpassant(Color::Black, BoardCoord(1, 3), BoardSide::Queenside)
            .is_ok());
    }

    #[test]
    fn test_en_passant_must_lunge() {
        let board = vec![
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. BP .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            "WP .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
        ];
        let mut board = Board::from_string_vec(board);
        board.move_piece(BoardCoord(0, 1), BoardCoord(0, 2));
        board.move_piece(BoardCoord(0, 2), BoardCoord(0, 3));
        assert!(board
            .check_enpassant(Color::Black, BoardCoord(1, 3), BoardSide::Queenside)
            .is_err());
    }

    #[test]
    fn test_en_passant_must_be_same_turn() {
        let board = vec![
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. BP .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            "WP .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
        ];
        let mut board = Board::from_string_vec(board);
        board.lunge(BoardCoord(0, 1));
        // simulate going to next turn without moving anything
        board.clear_just_lunged();
        assert!(board
            .check_enpassant(Color::Black, BoardCoord(1, 3), BoardSide::Queenside)
            .is_err());
    }

    fn assert_valid_movement(board: Vec<&str>, coord: (i8, i8), expected: Vec<&str>) {
        let board = Board::from_string_vec(board);
        assert_valid_movement_board(board, coord, expected);
    }

    fn assert_valid_movement_board(board: Board, coord: (i8, i8), expected: Vec<&str>) {
        use std::collections::HashSet;

        let coord = BoardCoord(coord.0, coord.1);
        let expected = to_move_list(expected);
        let mut move_list = MoveList::reserved();
        get_move_list_ignore_check(&board, coord, &mut move_list);
        println!("Board\n{:#?}", board);
        println!("Actual Move List");
        println!("{}", &move_list);
        println!("Expected Move List");
        println!("{}", &expected);

        let mut move_list_counts = HashSet::new();
        for ele in move_list.0 {
            move_list_counts.insert(ele);
        }

        let mut expected_counts = HashSet::new();
        for ele in expected.0 {
            expected_counts.insert(ele);
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
