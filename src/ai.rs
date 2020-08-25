use std::collections::HashMap;
use std::task::Poll;

use rand::seq::SliceRandom;

#[cfg(feature = "perf")]
use flamer::flame;

use crate::board::*;

type Move = (BoardCoord, BoardCoord);

/// This trait describes a computer player. An AIPlayer will have `next_move`
/// called with a certain board position and a player, and is expected to return
/// a legal move. Note that the return type is a `Poll`, so the AIPlayer may
/// spawn a thread if it takes a while to search for the right move. While the
/// player searches, it is expected to return `Poll::Pending`. If this is the case
/// `next_move` will be re-called intermittenly until it returns Poll::Ready
pub trait AIPlayer: std::fmt::Debug {
    /// Given a board, this function should return the next move the AI intends
    /// to play.
    fn next_move(&mut self, board: &BoardState, player: Color) -> Poll<Move>;
    /// Given a board that requires a promotion for a pawn, return what piece the
    /// pawn should be promoted to. By default, this is always a queen, but can
    /// be manually overridden if desired.
    fn next_promote(&mut self, _board: &BoardState) -> Poll<PieceType> {
        Poll::Ready(PieceType::Queen)
    }
}

#[derive(Debug)]
pub struct RandomPlayer {}

impl AIPlayer for RandomPlayer {
    fn next_move(&mut self, board: &BoardState, player: Color) -> Poll<Move> {
        let moves = board.board.get_all_moves(player);
        if moves.is_empty() {
            panic!(format!("Expected AI player to have at least one valid move! Board is in {:?} and needs promote: {:?}", board.checkmate, board.need_promote()))
        }
        let rand_move = *moves.choose(&mut rand::thread_rng()).unwrap();
        Poll::Ready(rand_move)
    }

    fn next_promote(&mut self, _board: &BoardState) -> Poll<PieceType> {
        let choice = *[
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Rook,
            PieceType::Queen,
        ]
        .choose(&mut rand::thread_rng())
        .unwrap();
        Poll::Ready(choice)
    }
}

#[derive(Debug)]
pub struct MinOptPlayer {}

impl AIPlayer for MinOptPlayer {
    fn next_move(&mut self, board: &BoardState, player: Color) -> Poll<Move> {
        let my_moves = board.board.get_all_moves(player);

        // lower score is better. here we have the score as the number of moves the opponent can make afterwards
        let mut best_score = usize::MAX;
        // track the scores each of minopt's moves get
        let mut move_scores: HashMap<usize, Vec<Move>> = HashMap::new();
        // for each of my possible moves, try making it and see how many moves the
        // opponent now has and track best move so far
        for (start, end) in my_moves {
            let mut board = board.clone();
            board.take_turn(start, end);
            let opponent_moves = board.board.get_all_moves(player.opposite());
            let score = opponent_moves.len();

            move_scores.entry(score).or_default().push((start, end));

            if score < best_score {
                best_score = score;
            }
        }

        let best_moves = move_scores.get(&best_score).unwrap();
        println!(
            "Best moves set: {:?} (score {:?} (lower is better))",
            best_moves, best_score
        );
        // get the moves with the least number of remaining possibilities
        Poll::Ready(*best_moves.choose(&mut rand::thread_rng()).unwrap())
    }
}
use std::sync::mpsc;

#[derive(Debug)]
pub struct TreeSearchPlayer {
    state: TreeSearch,
    // The thread to run when calculating next move
    // If none, then no thread is currently running.
    reciever: Option<mpsc::Receiver<((i32, Move), TreeSearch)>>,
}

/// Helper struct for TreeSearchPlayer containing all of the relevant state and
/// searhc parameters
#[derive(Debug, Clone)]
struct TreeSearch {
    /// Max number of plys to search.
    max_depth: usize,
    /// The "expected" sequence of moves, has length of `max_depth`
    principal_variation: Vec<Move>,
    /// For debugging. Counts how many branches were "generated" (were seen by
    /// `get_all_moves()`)
    total_branches: usize,
    /// For debugging. Counts how many branches were actually searched (has `search()`
    /// called on them)
    branches_searched: usize,
}

impl AIPlayer for TreeSearchPlayer {
    fn next_move(&mut self, board: &BoardState, player: Color) -> Poll<Move> {
        match &self.reciever {
            // Set up the thread if it isn't active
            None => {
                let (sender, reciever) = mpsc::channel();
                // We have to do these clones because the board needs to outlive
                // the thread and Rust can't prove that board actually does that
                // (This would cause problems, for example, if there was code in
                // screen.rs that modified the board while the thread ran!)
                // Thus, we must work on a copy of the board.
                // TODO: Consider not using a BoardState internally--instead something
                // faster like a bitboard.
                let board = board.clone();
                let mut treesearch = self.state.clone();
                std::thread::spawn(move || {
                    sender
                        .send((treesearch.search(&board, player), treesearch))
                        .unwrap()
                });
                self.reciever = Some(reciever);
                Poll::Pending
            }
            // Try asking the thread if it's done yet, and resetting it to None if it is
            Some(reciever) => match reciever.try_recv() {
                Ok(((score, move_to_make), state)) => {
                    self.reciever = None;
                    self.state = state;
                    println!("Best move: {:?} with score {:?}", move_to_make, score);
                    println!("principal {:?}", self.state);
                    println!(
                        "Searched {} of {} branches",
                        self.state.branches_searched, self.state.total_branches
                    );

                    Poll::Ready(move_to_make)
                }
                Err(mpsc::TryRecvError::Empty) => Poll::Pending,
                Err(mpsc::TryRecvError::Disconnected) => {
                    panic!("reciever machine broke (sender closed channel)")
                }
            },
        }
    }
}

impl TreeSearchPlayer {
    pub fn new(max_depth: usize) -> TreeSearchPlayer {
        TreeSearchPlayer {
            state: TreeSearch {
                max_depth,
                principal_variation: vec![(BoardCoord(-1, -1), BoardCoord(-1, -1)); max_depth],
                total_branches: 0,
                branches_searched: 0,
            },
            reciever: None,
        }
    }
}

impl TreeSearch {
    fn search(&mut self, position: &BoardState, player: Color) -> (i32, Move) {
        let max_depth = self.max_depth;
        let mut result = (0, (BoardCoord(-1, -1), BoardCoord(-1, -1)), -1, -1);
        for i in 1..=max_depth {
            // TODO: This super hacky. Make max_depth a parameter on score instead.
            // [this_is_fine.dog.png]
            self.max_depth = i;
            self.total_branches = 0;
            self.branches_searched = 0;
            result = self.score(position, 0, i32::MIN, i32::MAX, player);
        }
        (result.0, result.1)
    }

    #[cfg_attr(feature = "perf", flame)]
    fn score(
        &mut self,
        position: &BoardState,
        current_depth: usize,
        mut alpha: i32,
        mut beta: i32,
        player: Color,
    ) -> (i32, Move, i32, i32) {
        // Score the leaf node if we hit max depth or the game would end
        if current_depth >= self.max_depth || position.game_over() {
            let score = self.score_leaf(current_depth, position, player);
            // println!("{}Leaf node score: {:?}", "\t".repeat(current_depth), score);
            return (score, (BoardCoord(-2, -2), BoardCoord(-2, -2)), alpha, beta);
        }

        let mut moves = position.board.get_all_moves(position.current_player);
        // See also: https://www.chessprogramming.org/MVV-LVA
        // We sort here to make the AI check the most "useful" moves first. This
        // helps in causing an earlier alpha or beta cutoff, thereby reducing the
        // number of branches we have to check.
        // We first check all the captures. We sort the captures by the "most valuable victim"
        // and then by the "least valuable attacker". This is nice because it
        // lets us consider the most "useful" moves first--ie: use a pawn to defend an
        // attacking piece, or try and capture a high-value attacker first.
        // Note that sort_by_key will sort with smallest values first.const
        // For normal moves, we start with the most valuable pieces (queen, rook, etc)
        moves.sort_by_key(|&(start, end)| {
            let attacker = position.get(start);
            let victim = position.get(end);
            match &victim.0 {
                None => 10 - value(attacker),
                Some(_) => -(10 * value(victim) - value(attacker)),
            }
        });

        self.total_branches += moves.len();

        let my_turn = player == position.current_player;
        let mut best_score = if my_turn { i32::MIN } else { i32::MAX };
        let mut best_move = self.principal_variation[current_depth];

        // First, try checking the principal moves, to get a better value for alpha and beta
        let principal_move = self.principal_variation[current_depth];
        if let Some(i) = moves
            .iter()
            .position(|&the_move| the_move == principal_move)
        {
            moves.swap(0, i);
        }

        // Then, for each of our moves, try making it and see which one has the best score
        let mut i = 0;
        for (start, end) in moves {
            let mut next_position = position.clone();
            next_position.take_turn(start, end);
            // TODO: This really should get a real analysis, but for now, assuming the
            // player or ourself always promos to queen is an ok compromise.
            if let Some(coord) = next_position.need_promote() {
                next_position.promote(coord, PieceType::Queen);
            }

            let (score, _, _, _) =
                self.score(&next_position, current_depth + 1, alpha, beta, player);

            if my_turn {
                // is it is our turn, pick our best move
                if best_score < score {
                    alpha = alpha.max(score);
                    best_score = best_score.max(score);
                    best_move = (start, end);

                    if alpha >= beta {
                        // entering this block means that the move we just found is better than the worst possible outcome
                        // our opponent can always force onto us, which means we can end our search since no better move
                        // can possibly be better than this one
                        break;
                    }
                }
            } else {
                // if enemy turn, assume they will pick the worst move for us
                if best_score > score {
                    beta = beta.min(score);
                    best_score = best_score.min(score);
                    best_move = (start, end);

                    if alpha >= beta {
                        // entering this block means that the opponent can always force a worse outcome for this than the
                        // best-so-far move we've found, so we should stop searching since no move in this branch
                        // can possibly be better than the best-so-far
                        break;
                    }
                }
            }
            i += 1;
            debug_assert!(best_move != (BoardCoord(-1, -1), BoardCoord(-1, -1)));
        }

        // Add the best moves found so far to the principal move list
        self.principal_variation[current_depth] = best_move;

        self.branches_searched += i;

        (best_score, best_move, alpha, beta)
    }

    #[cfg_attr(feature = "perf", flame)]
    fn score_leaf(&mut self, current_depth: usize, position: &BoardState, player: Color) -> i32 {
        // Scoring function adapted from https://www.chessprogramming.org/Simplified_Evaluation_Function
        // The idea here is to make the AI care more about developing its pieces
        // This is achieve via "position tables", which award bonuses or penalities for
        // placing certain pieces on certain squares
        // For example, the Pawn position table encourages moving the center pawns,
        // so that pieces may be developed, but discourages moving the side pawns,
        // so that castling may be achieved.
        #[rustfmt::skip]
        const PAWN_POSITION_TABLE: [[i32; 8]; 8] = [
            [ 0,  0,  0,  0,  0,  0,  0,  0],
            [50, 50, 50, 50, 50, 50, 50, 50],
            [10, 10, 20, 30, 30, 20, 10, 10],
            [ 5,  5, 10, 25, 25, 10,  5,  5],
            [ 0,  0,  0, 20, 20,  0,  0,  0],
            [ 5, -5,-10,  0,  0,-10, -5,  5],
            [ 5, 10, 10,-20,-20, 10, 10,  5],
            [ 0,  0,  0,  0,  0,  0,  0,  0]
        ];
        // Encourage the knight to be near the center, to maximize the number of
        // squares it can control.
        #[rustfmt::skip]
        const KNIGHT_POSITION_TABLE: [[i32; 8]; 8] = [
            [-50,-40,-30,-30,-30,-30,-40,-50],
            [-40,-20,  0,  0,  0,  0,-20,-40],
            [-30,  0, 10, 15, 15, 10,  0,-30],
            [-30,  5, 15, 20, 20, 15,  5,-30],
            [-30,  0, 15, 20, 20, 15,  0,-30],
            [-30,  5, 10, 15, 15, 10,  5,-30],
            [-40,-20,  0,  5,  5,  0,-20,-40],
            [-50,-40,-30,-30,-30,-30,-40,-50],
        ];
        // Encourage the bishop to avoid the sides
        #[rustfmt::skip]
        const BISHOP_POSITION_TABLE: [[i32; 8]; 8] = [
            [-20,-10,-10,-10,-10,-10,-10,-20],
            [-10,  0,  0,  0,  0,  0,  0,-10],
            [-10,  0,  5, 10, 10,  5,  0,-10],
            [-10,  5,  5, 10, 10,  5,  5,-10],
            [-10,  0, 10, 10, 10, 10,  0,-10],
            [-10, 10, 10, 10, 10, 10, 10,-10],
            [-10,  5,  0,  0,  0,  0,  5,-10],
            [-20,-10,-10,-10,-10,-10,-10,-20],
        ];
        // Encourage the rook to either defend the king, or to threaten the back
        // ranks for checkmate
        #[rustfmt::skip]
        const ROOK_POSITION_TABLE: [[i32; 8]; 8] = [
            [ 0,  0,  0,  0,  0,  0,  0,  0],
            [ 5, 10, 10, 10, 10, 10, 10,  5],
            [-5,  0,  0,  0,  0,  0,  0, -5],
            [-5,  0,  0,  0,  0,  0,  0, -5],
            [-5,  0,  0,  0,  0,  0,  0, -5],
            [-5,  0,  0,  0,  0,  0,  0, -5],
            [-5,  0,  0,  0,  0,  0,  0, -5],
            [ 0,  0,  0,  5,  5,  0,  0,  0]
        ];
        // Encourage the queen to avoid the sides and play around the center
        #[rustfmt::skip]
        const QUEEN_POSITION_TABLE: [[i32; 8]; 8] = [
            [-20,-10,-10, -5, -5,-10,-10,-20],
            [-10,  0,  0,  0,  0,  0,  0,-10],
            [-10,  0,  5,  5,  5,  5,  0,-10],
            [ -5,  0,  5,  5,  5,  5,  0, -5],
            [  0,  0,  5,  5,  5,  5,  0, -5],
            [-10,  5,  5,  5,  5,  5,  0,-10],
            [-10,  0,  5,  0,  0,  0,  0,-10],
            [-20,-10,-10, -5, -5,-10,-10,-20]
        ];
        // Encourage the king to castle and to hide away in the early game.
        #[rustfmt::skip]
        const EARLY_KING_POSITION_TABLE: [[i32; 8]; 8] = [
            [-30,-40,-40,-50,-50,-40,-40,-30],
            [-30,-40,-40,-50,-50,-40,-40,-30],
            [-30,-40,-40,-50,-50,-40,-40,-30],
            [-30,-40,-40,-50,-50,-40,-40,-30],
            [-20,-30,-30,-40,-40,-30,-30,-20],
            [-10,-20,-20,-20,-20,-20,-20,-10],
            [ 20, 20,  0,  0,  0,  0, 20, 20],
            [ 20, 30, 10,  0,  0, 10, 30, 20]
        ];
        // Encourage the king to go out and fight near the end game.
        #[rustfmt::skip]
        // TODO: Use this table for endgame
        #[allow(unused)]
        const LATE_KING_POSITION_TABLE: [[i32; 8]; 8] = [
            [-50,-40,-30,-20,-20,-30,-40,-50],
            [-30,-20,-10,  0,  0,-10,-20,-30],
            [-30,-10, 20, 30, 30, 20,-10,-30],
            [-30,-10, 30, 40, 40, 30,-10,-30],
            [-30,-10, 30, 40, 40, 30,-10,-30],
            [-30,-10, 20, 30, 30, 20,-10,-30],
            [-30,-30,  0,  0,  0,  0,-30,-30],
            [-50,-30,-30,-30,-30,-30,-30,-50]
        ];

        let my_turn = position.current_player == player;
        // A bonus is applied when possible to make the AI prefer checkmate
        let bonus = match position.checkmate {
            CheckmateState::Normal => 0,
            CheckmateState::Check => {
                if my_turn {
                    -400
                } else {
                    400
                }
            }
            CheckmateState::Checkmate => {
                if my_turn {
                    return -999_999_999;
                } else {
                    // Subtracting the current_depth makes the AI prefer shorter
                    // checkmates over longer ones. We also immediately return because
                    // there is no reason to find the position scores since checkmate
                    // is the best possible thing to do.
                    return 999_999_999 - current_depth as i32;
                }
            }
            // A stalemate is draw, and so we try to make the bot play to win when
            // possible.
            CheckmateState::InsuffientMaterial | CheckmateState::Stalemate => -200,
        };

        let mut my_piece_score = 0;
        let mut my_position_score = 0;
        let mut their_piece_score = 0;
        let mut their_position_score = 0;
        for i in 0..8 {
            for j in 0..8 {
                let coord = BoardCoord(i, j);
                // offsets into the position tables
                // we flip them vertically when playing as black because the
                // tables are constructed for white's side
                let (offset_x, offset_y) = match player {
                    Color::White => (i as usize, j as usize),
                    Color::Black => (i as usize, 7 - j as usize),
                };

                let tile = position.get(coord);

                if tile.0.is_none() {
                    continue;
                }
                let piece = tile.0.unwrap();
                use PieceType::*;
                // Values here are also taken from https://www.chessprogramming.org/Simplified_Evaluation_Function
                // It seems that this causes the bot to value the bishops a bit more than
                // the knights.
                let (piece_score, position_score) = match piece.piece {
                    Pawn { .. } => (100, PAWN_POSITION_TABLE[offset_x][offset_y]),
                    Knight => (320, KNIGHT_POSITION_TABLE[offset_x][offset_y]),
                    Bishop => (330, BISHOP_POSITION_TABLE[offset_x][offset_y]),
                    Rook => (500, ROOK_POSITION_TABLE[offset_x][offset_y]),
                    Queen => (900, QUEEN_POSITION_TABLE[offset_x][offset_y]),
                    // TODO use the late position table
                    King => (20000, EARLY_KING_POSITION_TABLE[offset_x][offset_y]),
                };

                let my_piece = piece.color == player;
                if my_piece {
                    my_piece_score += piece_score;
                    my_position_score += position_score;
                } else {
                    their_piece_score += piece_score;
                    their_position_score += position_score;
                }
            }
        }

        my_piece_score + my_position_score - (their_piece_score + their_position_score) + bonus
    }
}

fn value(tile: &Tile) -> i32 {
    use PieceType::*;
    match tile.0 {
        None => -1,
        Some(piece) => match piece.piece {
            Pawn { .. } => 1,
            Knight => 2,
            Bishop => 2,
            Rook => 3,
            Queen => 4,
            King => 0,
        },
    }
}
