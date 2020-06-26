use chess::ai;
use chess::board;

use ai::AIPlayer;

fn main() {
    let board = vec![
        ".. .. .. ..  .. WK ..",
        ".. .. .. .. .. .. .. ..",
        ".. .. .. .. .. .. .. ..",
        ".. .. .. .. .. .. .. ..",
        ".. BR .. .. .. .. .. ..",
        ".. .. BR .. .. .. .. ..",
        ".. .. .. .. .. .. .. ..",
        ".. .. BK .. .. .. .. ..",
    ];
    let board = board::Board::from_string_vec(board);
    let old_board = board::BoardState::new(board.clone());
    let mut board = board::BoardState::new(board.clone());
    board.current_player = board::Color::Black;

    let mut alphabeta_ai = ai::TreeSearchPlayer {};
    let (start, end) = alphabeta_ai.next_move(&board, board.current_player);
    board
        .take_turn(start, end)
        .expect("Expected move to be legal!");

    println!("{}", old_board.board);
    println!("{}", board.board);
}
