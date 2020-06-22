use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

use crate::board::*;

pub trait AIPlayer: std::fmt::Debug {
    fn next_move(&mut self, board: &BoardState) -> (BoardCoord, BoardCoord);
    fn player_color(&self) -> Color;
    fn next_promote(&mut self, board: &BoardState) -> PieceType;
}
#[derive(Debug)]
pub struct RandomPlayer {
    pub player_color: Color,
}

impl AIPlayer for RandomPlayer {
    fn next_move(&mut self, board: &BoardState) -> (BoardCoord, BoardCoord) {
        let moves = board.board.get_all_moves(self.player_color);
        if moves.len() == 0 {
            loop {}
        }
        *moves.choose(&mut rand::thread_rng()).unwrap()
    }

    fn player_color(&self) -> Color {
        self.player_color
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
