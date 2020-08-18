use rand::{self, Rng};

use ggez::graphics::spritebatch::{SpriteBatch, SpriteIdx};
use ggez::graphics::{self, Color, DrawParam, Image};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

/// This struct manages a spritebatched collection of particles, which move
/// each time upd8 is called.
#[derive(Debug)]
pub struct ParticleSystem {
    /// ggez's spritebatcher. This is what actually gets modified at runtime.
    spritebatch: SpriteBatch,
    /// A list of the sprite indicies. This is used at draw time to set the
    /// relevant parameters. The corresponding index of the values in `positions`
    /// and `velocities` corresponds to this particular sprite index.
    sprites: Vec<SpriteIdx>,
    positions: Vec<Point2<f32>>,
    velocities: Vec<Vector2<f32>>,
}

impl ParticleSystem {
    pub fn new(ctx: &mut Context, pos: Point2<f32>) -> ParticleSystem {
        // TODO: pass in an image instead of making one right here
        let image = Image::solid(ctx, 8, Color::from_rgb_u32(0xFF0000)).unwrap();
        let mut spritebatch = SpriteBatch::new(image);
        let mut sprites = vec![];
        let mut positions = vec![];
        let mut velocities = vec![];
        for _ in 0..10 {
            sprites.push(spritebatch.add(DrawParam::default()));
            positions.push(pos);
            velocities.push(Vector2::new(
                rand::thread_rng().gen_range(-1.0, 1.0),
                rand::thread_rng().gen_range(-1.0, 1.0),
            ));
        }

        ParticleSystem {
            spritebatch,
            sprites,
            positions,
            velocities,
        }
    }

    pub fn upd8(&mut self) {
        for (pos, vel) in self.positions.iter_mut().zip(&self.velocities) {
            *pos += vel;
        }
    }

    pub fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        for (&sprite, pos) in self.sprites.iter().zip(&self.positions) {
            self.spritebatch.set(sprite, (*pos,))?;
        }

        graphics::draw(ctx, &self.spritebatch, DrawParam::default())
    }
}
