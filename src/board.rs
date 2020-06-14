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

    /// Attempt to move the piece located at `start` to `end`. This function
    /// returns `Ok()` if the move was successful and `Err` if it was not.
    /// It also sets `current_player` to the opposite color and handles the
    /// "just_lunged" pawn flags.
    /// A pawn needs promotion then this function always fails. You should
    /// call `promote` on the pawn.
    pub fn take_turn(&mut self, start: BoardCoord, end: BoardCoord) -> Result<(), &'static str> {
        use Color::*;
        use MoveType::*;
        if self.board.pawn_needs_promotion().is_some() {
            return Err("A pawn needs to be promoted first.");
        }

        match move_type(&self.board, start, end) {
            Castle(color, side) => {
                self.board.can_castle(color, side)?;
                // Clear the just lunged flags _after_ checking the move is valid
                // That way, invalid moves don't try to clear the flag.
                self.board.clear_just_lunged();
                self.board.castle(color, side);
            }
            Normal => {
                self.board.check_move(self.current_player, start, end)?;
                self.board.clear_just_lunged();
                self.board.move_piece(start, end);
            }
            Lunge => {
                // A lunge is just a special case for a normal move, so we don't
                // really do anything cool here
                self.board.check_move(self.current_player, start, end)?;
                // Clear the old lunge flag before the new one
                self.board.clear_just_lunged();
                self.board.lunge(start);
            }
            EnPassant(side) => {
                self.board
                    .check_enpassant(self.current_player, start, side)?;
                self.board.enpassant(start, end);

                // Don't clear the lunge flag until _after_ we check for enpassant
                // (otherwise we will never be able to :P)
                self.board.clear_just_lunged();
            }
        }

        if self.need_promote().is_none() {
            self.current_player = match self.current_player {
                White => Black,
                Black => White,
            };
        }

        // Update the checkmate status
        self.checkmate = self.board.checkmate_state(self.current_player);

        Ok(())
    }

    pub fn need_promote(&self) -> Option<BoardCoord> {
        return self.board.pawn_needs_promotion();
    }

    /// Attempt to promote the pawn.
    pub fn promote(&mut self, coord: BoardCoord, piece: PieceType) -> Result<(), &'static str> {
        if self.need_promote().is_none() {
            return Err("No pawn needs to be promoted at this time");
        }

        self.board.checK_promote(coord, piece)?;
        self.board.promote_pawn(coord, piece);

        use Color::*;
        self.current_player = match self.current_player {
            White => Black,
            Black => White,
        };

        // Update the checkmate status
        self.checkmate = self.board.checkmate_state(self.current_player);
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
    /// This function also DOES check if the King can castle.
    pub fn get_move_list(&self, coord: BoardCoord) -> Vec<BoardCoord> {
        if !on_board(coord) {
            return vec![];
        }

        if self.need_promote().is_some() {
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

        let mut list = filter_check_causing_moves(&self.board, self.current_player, coord, list);
        match piece.piece {
            PieceType::King => {
                list.0
                    .append(&mut self.board.castle_locations(self.current_player));
            }
            PieceType::Pawn(_) => list
                .0
                .append(&mut self.board.enpassant_locations(self.current_player, coord)),
            _ => {}
        }

        list.0
    }

    pub fn game_over(&self) -> bool {
        match self.board.checkmate_state(self.current_player) {
            CheckmateState::Normal | CheckmateState::Check => false,
            CheckmateState::Checkmate | CheckmateState::Stalemate => true,
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
                    'P' => Tile::new(color, Pawn(false)),
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
        let valid_end_spots = get_move_list_ignore_check(self, start).0;

        if valid_end_spots.contains(&end) {
            Ok(())
        } else {
            Err("Can't move a piece there")
        }
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
        use Color::*;
        let fifth_rank = match player {
            White => 4,
            Black => 2,
        };
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
                piece: PieceType::Pawn(_),
                has_moved: _,
            }) if c == player => (),
            _ => return Err("Capturing piece must be a pawn of the player's color"),
        }

        match captured_pawn.0 {
            Some(Piece {
                color: c,
                piece: PieceType::Pawn(true),
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
        for i in 0..8 {
            for j in 0..8 {
                self.get_mut(BoardCoord(j, i)).set_just_lunged(false);
            }
        }
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

    /// Returns if the player is currently in checkmate
    fn checkmate_state(&self, player: Color) -> CheckmateState {
        use CheckmateState::*;
        match (self.has_legal_moves(player), self.is_in_check(player)) {
            (false, false) => Stalemate,
            (false, true) => Checkmate,
            (true, false) => Normal,
            (true, true) => Check,
        }
    }

    fn is_in_check(&self, player: Color) -> bool {
        !self.is_square_safe(player, &self.get_king(player).unwrap())
    }

    fn has_legal_moves(&self, player: Color) -> bool {
        // TODO: this is also HILARIOUSLY INEFFICENT
        for i in 0..8 {
            for j in 0..8 {
                let coord = BoardCoord(j, i);
                let tile = self.get(coord);
                if tile.is_color(player) {
                    if !get_move_list_full(self, player, coord).0.is_empty() {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Returns Some(BoardCoord) if there is a pawn in the last rank that needs
    /// to be promoted. Otherwise, this functino returns None.
    pub fn pawn_needs_promotion(&self) -> Option<BoardCoord> {
        for i in 0..7 {
            let black_coord = BoardCoord(i, 0);
            if self
                .get(black_coord)
                .is(Color::Black, PieceType::Pawn(false))
            {
                return Some(black_coord);
            }

            let white_coord = BoardCoord(i, 7);
            if self
                .get(white_coord)
                .is(Color::White, PieceType::Pawn(false))
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
    pub fn checK_promote(&self, coord: BoardCoord, piece: PieceType) -> Result<(), &'static str> {
        use PieceType::*;
        let pawn = self.get(coord);
        let color = match coord.1 {
            0 => Color::Black,
            7 => Color::White,
            _ => {
                return Err("Can only promote a pawn at the last rank of the board");
            }
        };

        if !pawn.is(color, Pawn(false)) {
            return Err("Can only promote a pawn of opposite color at this position.");
        }

        match piece {
            Pawn(_) => Err("Can not promote to a pawn"),
            King => Err("Can not promote to a king"),
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

    /// Returns a vector of all of coordinates of the pieces whose color and
    /// piece match. This vector is empty if there are no pieces that match.
    fn get_pieces(&self, color: Color, piece: PieceType) -> Vec<BoardCoord> {
        let mut list = vec![];
        for i in 0..8 {
            for j in 0..8 {
                let coord = BoardCoord::new((j, 7 - i)).unwrap();
                let tile = self.get(coord);
                if tile.is(color, piece) {
                    list.push(coord);
                }
            }
        }
        list
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
/// check and DOES NOT check if the King can castle.
fn get_move_list_full(board: &Board, player: Color, coord: BoardCoord) -> MoveList {
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
            Pawn(_) => check_pawn(board, coord, piece.color),
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
    let mut could_do_first_move = false;
    if on_board(forwards) {
        if board.get(forwards).0.is_none() {
            valid_end_pos.0.push(forwards);
            could_do_first_move = true;
        }
    }

    // if piece has not moved yet, check for double movement
    let double_move = BoardCoord(pos.0, pos.1 + color.direction() * 2);
    let pawn = board.get(pos).0.unwrap();
    if on_board(double_move) {
        if board.get(double_move).0.is_none() && !pawn.has_moved && could_do_first_move {
            valid_end_pos.0.push(double_move);
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
        Pawn(_) | Knight | King => panic!("Expected a Rook, Bishop, or Queen. Got {:?}", piece),
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
        Pawn(_) | Rook | Bishop | Queen => panic!("Expected a Knight or a King, got {:?}", piece),
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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum BoardSide {
    Queenside,
    Kingside,
}

enum MoveType {
    Castle(Color, BoardSide),
    Normal,
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
        (PieceType::Pawn(_), (0, 2), _) => Lunge,
        // An enpassant move will always attempt to move into an empty square
        // while a capture will move onto a nonempty square
        (PieceType::Pawn(_), (-1, 1), true) => EnPassant(Queenside),
        (PieceType::Pawn(_), (1, 1), true) => EnPassant(Kingside),
        _ => Normal,
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
            if let PieceType::Pawn(just_lunged) = &mut piece.piece {
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
            Some(piece) => piece.color == color && piece.piece == piece_type,
        }
    }

    /// Returns true if the Tile is actually a peice that has moved
    fn is_moved_piece(&self) -> bool {
        match self.0 {
            None => false,
            Some(piece) => piece.has_moved,
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

    /// Returns true if the `Tile` actually has a piece and
    /// `piece` matches the `PieceType` of the piece. Note: the `just_lunged`
    /// flag on the Pawn piecetype is ignored.
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
                Pawn(_) => "♟",
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
    has_moved: bool,
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
/// `Pawn` has a `bool` associated with that is true if the piece has just
/// lunged (moved two spaces) on the previous turn.S
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceType {
    Pawn(bool),
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
            Pawn(_) => write!(f, "P"),
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
            Tile::new(Color::White, PieceType::Pawn(false)),
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
            Tile::new(Color::White, PieceType::Pawn(false)),
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
            Tile::new(Color::White, PieceType::Pawn(false)),
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
    const WHITE_KINGSIDE_ROOK: BoardCoord = BoardCoord(7, 0);
    const BLACK_QUEENSIDE_ROOK: BoardCoord = BoardCoord(0, 7);
    const BLACK_KINGSIDE_ROOK: BoardCoord = BoardCoord(7, 7);

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
        let move_list = get_move_list_ignore_check(&board, coord);
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
