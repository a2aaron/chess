use chess::screen;

use ggez::conf;
use ggez::event;
use ggez::ContextBuilder;

use std::io;

/// Read a string from standard in, optionally printing a message to the user if
/// provided. The inputted string will not contain a trailing newline.
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
    let mut cb = ContextBuilder::new("chess", "a2aaron");
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        // Add the resources path so we can use it.
        // The path will be in a folder called "resources"
        // You can load things from it using paths like "\\my_file.txt"
        let mut path = std::path::PathBuf::from(manifest_dir);
        path.push("resources");
        println!("Adding path {:?}", path);
        // We need this re-assignment alas, see
        // https://aturon.github.io/ownership/builders.html
        // under "Consuming builders"
        cb = cb.add_resource_path(path);
    }
    // Make the Context
    let (mut ctx, mut event_loop) = cb
        .window_setup(conf::WindowSetup::default().title("chess"))
        .window_mode(
            conf::WindowMode::default().dimensions(screen::SCREEN_WIDTH, screen::SCREEN_HEIGHT),
        )
        .build()
        .expect("could not create ggez context!");

    // Create an instance of your event handler.
    // Usually, you should provide it with the Context object to
    // use when setting your game up.

    let mut game_state = screen::Game::new(&mut ctx);
    // Run!

    match event::run(&mut ctx, &mut event_loop, &mut game_state) {
        Ok(_) => println!("Exited cleanly."),
        Err(e) => println!("Error occured: {}", e),
    }
}
