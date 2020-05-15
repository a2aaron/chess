mod board;

use board::*;
use std::io;

pub fn read_string_from_stdin(message: Option<String>) -> String {
    if let Some(x) = message {
        println!("{}", x);
    }
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input = input.trim().to_string(); // Remove trailing newline
    input
}

fn main() {
    #[rustfmt::skip]
    let setup = vec![
        ".. .. .. .. .. .. .. ..",
        ".. .. .. WP .. .. BP ..",
        ".. .. .. .. .. .. .. ..",
        ".. .. .. .. .. .. .. ..",
        ".. BP .. WQ .. .. .. ..",
        ".. .. .. .. .. .. .. ..",
        ".. BN .. .. .. .. .. ..",
        ".. .. .. .. .. .. WQ ..",
    ];
    let mut board = Board::from_string_vec(setup);
    loop {
        println!("{}", board);
        println!("Select a piece:");
        let a: i8 = read_string_from_stdin(None).parse().unwrap();
        let b: i8 = read_string_from_stdin(None).parse().unwrap();
        println!("Chosen piece: {:?}", board.get((a, b)));
        println!("Select an end location");
        for y in (0..8).rev() {
            for x in 0..8 {
                if board.check_move(Color::White, (a, b), (x, y)).is_ok() {
                    print!("## ");
                } else {
                    print!("{} ", board.get((x, y)));
                }
            }
            println!("");
        }
        let c: i8 = read_string_from_stdin(None).parse().unwrap();
        let d: i8 = read_string_from_stdin(None).parse().unwrap();
        println!("Goal place: {:?}", board.get((c, d)));
        let test: i8 = "1".to_string().parse().unwrap();
        println!("{:?}", board.move_piece(Color::White, (a, b), (c, d)));
    }
}
