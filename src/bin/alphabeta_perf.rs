use chess::ai;
use chess::board;

use ai::AIPlayer;

use flamescope;

#[cfg(not(feature = "perf"))]
compile_error!("Must be compiled with features=perf");

fn main() {
    let board = vec![
        ".. .. WR .. WR WK .. ..",
        "WP WP BR .. .. WP WP WP",
        ".. .. .. .. .. .. .. ..",
        ".. .. .. .. .. .. .. ..",
        "WQ .. .. WP WK .. BP ..",
        "BP .. .. BQ .. BR .. BP",
        "BP BB .. .. BP WP .. ..",
        ".. BK .. .. .. .. .. ..",
    ];
    let board = board::Board::from_string_vec(board);
    let old_board = board::BoardState::new(board.clone());
    let mut board = board::BoardState::new(board.clone());
    board.current_player = board::Color::Black;

    let mut alphabeta_ai = ai::TreeSearchPlayer { depth: 3 };
    let (start, end) = alphabeta_ai.next_move(&board, board.current_player);
    board
        .take_turn(start, end)
        .expect("Expected move to be legal!");

    println!("{}", old_board.board);
    println!("{}", board.board);
    println!("Done with alphabeta, now saving json file...");

    flamescope::dump(&mut std::fs::File::create("flamegraph.json").unwrap()).unwrap();
}
