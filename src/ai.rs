use std::collections::HashMap;
use std::task::Poll;

use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

#[cfg(feature = "perf")]
use flamer::flame;

use crate::board::*;

type Move = (BoardCoord, BoardCoord);

pub trait AIPlayer: std::fmt::Debug {
    fn next_move(&mut self, board: &BoardState, player: Color) -> Poll<Move>;
    fn next_promote(&mut self, _board: &BoardState) -> Poll<PieceType> {
        Poll::Ready(PieceType::Queen)
    }
}

#[derive(Debug)]
pub struct RandomPlayer {}

impl AIPlayer for RandomPlayer {
    fn next_move(&mut self, board: &BoardState, player: Color) -> Poll<Move> {
        let moves = board.board.get_all_moves(player);
        if moves.len() == 0 {
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
        Poll::Ready(*best_moves.choose(&mut rand::thread_rng()).unwrap())
    }
}
use std::sync::mpsc;

#[derive(Debug)]
pub struct TreeSearchPlayer {
    state: TreeSearch,
    // The thread to run when calculating next move
    // If none, then no thread is currently running.
    reciever: Option<mpsc::Receiver<((i32, Move, i32, i32), TreeSearch)>>,
}
#[derive(Debug, Clone)]
struct TreeSearch {
    max_depth: usize,
    // Unfortunately, AIs can not dance, so this is always empty
    killer_moves: Vec<Move>,
    total_branches: usize,
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
                // (This would cause problems, for example, if there was code in screen.rs
                // that modified the board while the thread ran!)
                // Thus, we must work on a copy of the board.
                let board = board.clone();
                let mut treesearch = self.state.clone();
                std::thread::spawn(move || {
                    sender
                        .send((
                            treesearch.score(&board, 0, i32::MIN, i32::MAX, player),
                            treesearch,
                        ))
                        .unwrap()
                });
                self.reciever = Some(reciever);
                Poll::Pending
            }
            // Try asking the thread if it's done yet, and resetting it to None if it is
            Some(reciever) => match reciever.try_recv() {
                Ok(((score, move_to_make, _, _), state)) => {
                    self.reciever = None;
                    self.state = state;
                    println!("Best move: {:?} with score {:?}", move_to_make, score);
                    println!("Killer {:?}", self.state);
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
                killer_moves: vec![(BoardCoord(-1, -1), BoardCoord(-1, -1)); 2 * max_depth],
                total_branches: 0,
                branches_searched: 0,
            },
            reciever: None,
        }
    }
}

impl TreeSearch {
    #[cfg_attr(feature = "perf", flame)]
    fn score(
        &mut self,
        position: &BoardState,
        current_depth: usize,
        mut alpha: i32,
        mut beta: i32,
        player: Color,
    ) -> (i32, Move, i32, i32) {
        // Explore only 2 moves ahead
        if current_depth >= self.max_depth || position.game_over() {
            let score = self.score_leaf(current_depth, position, player);
            // println!("{}Leaf node score: {:?}", "\t".repeat(current_depth), score);
            return (score, (BoardCoord(-2, -2), BoardCoord(-2, -2)), alpha, beta);
        }

        let mut moves = position.board.get_all_moves(position.current_player);
        self.total_branches += moves.len();

        let my_turn = player == position.current_player;
        let mut best_score = if my_turn { i32::MIN } else { i32::MAX };
        let mut best_move = (BoardCoord(-1, -1), BoardCoord(-1, -1));
        let mut second_best_move = (BoardCoord(-1, -1), BoardCoord(-1, -1));

        // First, try checking the killer moves, to get a better value for alpha and beta
        let killer_move = self.killer_moves[current_depth * 2];
        let killer_move2 = self.killer_moves[(current_depth * 2) + 1];

        if moves.len() > 1 {
            if let Some(i) = moves.iter().position(|&the_move| the_move == killer_move) {
                moves.swap(0, i);
                // do_killer = true;
            }

            if let Some(i) = moves.iter().position(|&the_move| the_move == killer_move2) {
                moves.swap(1, i);
                // do_killer2 = true;
            }
        }

        // Then, for each of our moves, try making it and see which one has the best score
        let mut i = 0;
        for (start, end) in moves {
            let mut next_position = position.clone();
            next_position.take_turn(start, end).expect(&format!(
                "Expected {:?} -> {:?} to be a legal move!",
                start, end
            ));
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
                    second_best_move = best_move;
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

                    second_best_move = best_move;
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

        // Add the best moves found so far to the killer move list
        self.killer_moves[current_depth * 2] = best_move;
        self.killer_moves[(current_depth * 2) + 1] = second_best_move;

        self.branches_searched += i;
        // println!(
        //     "{} Killer moves: {:?} {:?} (at depth: {})",
        //     "\t".repeat(current_depth),
        //     best_move,
        //     second_best_move,
        //     current_depth,
        // );

        (best_score, best_move, alpha, beta)
    }

    #[cfg_attr(feature = "perf", flame)]
    fn score_leaf(&mut self, current_depth: usize, position: &BoardState, player: Color) -> i32 {
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
