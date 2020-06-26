use std::collections::HashMap;

use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

#[cfg(feature = "perf")]
use flamer::flame;

use crate::board::*;

pub trait AIPlayer: std::fmt::Debug {
    fn next_move(&mut self, board: &BoardState, player: Color) -> (BoardCoord, BoardCoord);
    fn next_promote(&mut self, _board: &BoardState) -> PieceType {
        PieceType::Queen
    }
}
#[derive(Debug)]
pub struct RandomPlayer {}

impl AIPlayer for RandomPlayer {
    fn next_move(&mut self, board: &BoardState, player: Color) -> (BoardCoord, BoardCoord) {
        let moves = board.board.get_all_moves(player);
        if moves.len() == 0 {
            panic!(format!("Expected AI player to have at least one valid move! Board is in {:?} and needs promote: {:?}", board.checkmate, board.need_promote()))
        }
        *moves.choose(&mut rand::thread_rng()).unwrap()
    }

    fn next_promote(&mut self, _board: &BoardState) -> PieceType {
        *[
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Rook,
            PieceType::Queen,
        ]
        .choose(&mut rand::thread_rng())
        .unwrap()
    }
}

#[derive(Debug)]
pub struct MinOptPlayer {}

impl AIPlayer for MinOptPlayer {
    fn next_move(&mut self, board: &BoardState, player: Color) -> (BoardCoord, BoardCoord) {
        let my_moves = board.board.get_all_moves(player);

        // lower score is better. here we have the score as the number of moves the opponent can make afterwards
        let mut best_score = usize::MAX;
        // track the scores each of minopt's moves get
        let mut move_scores: HashMap<usize, Vec<(BoardCoord, BoardCoord)>> = HashMap::new();
        // for each of my possible moves, try making it and see how many moves the
        // opponent now has and track best move so far
        for (start, end) in my_moves {
            let mut board = board.clone();
            board.take_turn(start, end).expect(&format!(
                "Expected {:?} -> {:?} to be a legal move!",
                start, end
            ));
            let opponent_moves = board.board.get_all_moves(player.opposite());
            let score = opponent_moves.len();

            move_scores
                .entry(score)
                .or_insert(vec![])
                .push((start, end));

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
        *best_moves.choose(&mut rand::thread_rng()).unwrap()
    }
}

#[derive(Debug)]
pub struct TreeSearchPlayer {
    pub depth: usize,
}

impl AIPlayer for TreeSearchPlayer {
    fn next_move(&mut self, board: &BoardState, player: Color) -> (BoardCoord, BoardCoord) {
        let (score, move_to_make) = self.score(board, 0, player);
        // println!("Best move: {:?} with score {:?}", move_to_make, score);
        move_to_make
    }
}

impl TreeSearchPlayer {
    #[cfg_attr(feature = "perf", flame)]
    fn score(
        &self,
        position: &BoardState,
        current_depth: usize,
        player: Color,
    ) -> (i32, (BoardCoord, BoardCoord)) {
        // Explore only 2 moves ahead
        if current_depth >= self.depth || position.game_over() {
            let score = self.score_leaf(current_depth, position, player);
            // println!("{}Leaf node score: {:?}", "\t".repeat(current_depth), score);
            return (score, (BoardCoord(-2, -2), BoardCoord(-2, -2)));
        }

        let moves = position.board.get_all_moves(position.current_player);
        let my_turn = player == position.current_player;
        let mut best_score = if my_turn { i32::MIN } else { i32::MAX };
        let mut best_move = (BoardCoord(-1, -1), BoardCoord(-1, -1));

        // For each of our moves, try making it and see which one has the best score
        for (start, end) in moves {
            // println!(
            //     "{}Now considering move: {:?} -> {:?} (player: {:?})",
            //     "\t".repeat(current_depth),
            //     start,
            //     end,
            //     position.current_player
            // );
            let mut next_position = position.clone();
            next_position.take_turn(start, end).expect(&format!(
                "Expected {:?} -> {:?} to be a legal move!",
                start, end
            ));
            let (score, _) = self.score(&next_position, current_depth + 1, player);

            if my_turn {
                // is it is our turn, pick our best move
                if best_score < score {
                    // println!("{}Found better move for me! new: {:?} -> {:?} (score: {:?}), old: {:?} (score: {:?})",  "\t".repeat(current_depth),start,end, score, best_move, best_score);
                    best_score = score;
                    best_move = (start, end);
                }
            } else {
                // if enemy turn, assume they will pick the worst move for us
                if best_score > score {
                    // println!("Found better move for opponent! new: {:?} (score: {:?}), old: {:?} (score: {:?})",(start,end), score, best_move, best_score);
                    best_score = score;
                    best_move = (start, end);
                }
            }

            debug_assert!(best_move != (BoardCoord(-1, -1), BoardCoord(-1, -1)));
        }

        // println!(
        //     "{}Best move was: {:?} (score: {:?})",
        //     "\t".repeat(current_depth),
        //     best_move,
        //     best_score
        // );
        (best_score, best_move)
    }

    #[cfg_attr(feature = "perf", flame)]
    fn score_leaf(&self, current_depth: usize, position: &BoardState, player: Color) -> i32 {
        let my_turn = position.current_player == player;
        let bonus = match position.checkmate {
            CheckmateState::Normal => 0,
            CheckmateState::Check => {
                if my_turn {
                    -4
                } else {
                    4
                }
            }
            CheckmateState::Checkmate => {
                if my_turn {
                    -999
                } else {
                    999 - current_depth as i32
                }
            }
            CheckmateState::InsuffientMaterial | CheckmateState::Stalemate => -2,
        };

        let my_pieces = position.board.get_pieces_vec(player);
        let their_pieces = position.board.get_pieces_vec(player.opposite());

        bonus + score_pieces(my_pieces) - score_pieces(their_pieces)
    }
}

fn score_pieces(pieces: Vec<Piece>) -> i32 {
    use PieceType::*;
    let mut score = 0;
    for piece in pieces {
        score += match piece.piece {
            Pawn(_) => 1,
            Knight => 3,
            Bishop => 3,
            Rook => 5,
            Queen => 9,
            King => 0, // Ignore the king
        };
    }
    score
}
