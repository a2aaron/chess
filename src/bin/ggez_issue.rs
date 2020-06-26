use ggez::conf;
use ggez::event;
use ggez::graphics;
use ggez::mint;
use ggez::{Context, GameResult};
use std::env;
use std::path;

struct MainState {
    text: graphics::Text,
    batch: graphics::spritebatch::SpriteBatch,
}

impl MainState {
    fn new(ctx: &mut Context) -> GameResult<MainState> {
        let font = graphics::Font::new(ctx, "/freeserif.ttf")?;
        let mut text = graphics::Text::new("Hello world!");
        text.set_font(font, graphics::Scale::uniform(48.0));

        let image = graphics::Image::new(ctx, "/player.png")?;
        let batch = graphics::spritebatch::SpriteBatch::new(image);
        let s = MainState { text, batch };
        Ok(s)
    }
}

impl event::EventHandler for MainState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::BLACK);

        for i in 0..100 {
            let fi = i as f32 * 20.0;
            self.batch.add(graphics::DrawParam {
                dest: mint::Point2 { x: fi, y: fi },
                ..graphics::DrawParam::default()
            });
        }

        graphics::draw(ctx, &self.batch, (mint::Point2 { x: 0.0, y: 300.0 },))?;
        graphics::draw(ctx, &self.text, (mint::Point2 { x: 10.0, y: 10.0 },))?;
        graphics::draw(ctx, &self.text, (mint::Point2 { x: 10.0, y: 10.0 },))?;
        graphics::present(ctx);

        Ok(())
    }
}

pub fn main() {
    let c = conf::Conf::new();
    let mut cb = ggez::ContextBuilder::new("heckin_strange", "ggez");

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        // Add the resources path so we can use it.
        let mut path = std::path::PathBuf::from(manifest_dir);
        path.push("resources");
        println!("Adding path {:?}", path);
        // We need this re-assignment alas, see
        // https://aturon.github.io/ownership/builders.html
        // under "Consuming builders"
        cb = cb.add_resource_path(path);
    }
    let (mut ctx, mut event_loop) = cb.build().expect("failed to build ctx");

    let mut state = MainState::new(&mut ctx).unwrap();
    match event::run(&mut ctx, &mut event_loop, &mut state) {
        Ok(_) => println!("Exited cleanly."),
        Err(e) => println!("Error occured: {}", e),
    }
}
