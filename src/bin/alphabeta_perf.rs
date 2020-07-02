use std::time::Instant;

use chess::ai;
use chess::board;

use ai::AIPlayer;

#[cfg(feature = "perf")]
use flamescope;

fn main() {
    let now = Instant::now();
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

    let mut alphabeta_ai = ai::TreeSearchPlayer::new(6);
    let start: board::BoardCoord;
    let end: board::BoardCoord;
    loop {
        match alphabeta_ai.next_move(&board, board.current_player) {
            std::task::Poll::Ready((start_, end_)) => {
                start = start_;
                end = end_;
                break;
            }
            std::task::Poll::Pending => continue,
        }
    }

    board
        .take_turn(start, end)
        .expect("Expected move to be legal!");

    let duration = now.elapsed();

    println!("{}", old_board.board);
    println!("{}", board.board);
    println!("Done with alphabeta, now saving json file...");
    println!("Took {:?}", duration);

    // Use https://www.speedscope.app/ to view the flamegraph!
    #[cfg(feature = "perf")]
    flamescope::dump(&mut std::fs::File::create("flamegraph.json").unwrap()).unwrap();
}
