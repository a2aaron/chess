use crate::board::*;

trait Player {
    fn take_turn(&mut board: BoardState);
}
