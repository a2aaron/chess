use std::time::{Duration, Instant};

use rand::{self, Rng};

use ggez::graphics::spritebatch::{SpriteBatch, SpriteIdx};
use ggez::graphics::{self, Color, DrawParam, Image};
use ggez::nalgebra as na;
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
    positions: Vec<na::Point2<f32>>,
    velocities: Vec<na::Vector2<f32>>,
    duration: Duration,
    start: Instant,
}

impl ParticleSystem {
    pub fn new(
        ctx: &mut Context,
        pos: na::Point2<f32>,
        angle: na::Vector2<f32>,
        spread: f32,
        intensity: f32,
        num_particles: usize,
    ) -> ParticleSystem {
        // TODO: pass in an image instead of making one right here
        let image = Image::solid(ctx, 8, Color::from_rgb_u32(0xFF0000)).unwrap();
        let mut spritebatch = SpriteBatch::new(image);
        let mut sprites = vec![];
        let mut positions = vec![];
        let mut velocities = vec![];
        for _ in 0..num_particles {
            sprites.push(spritebatch.add(DrawParam::default()));
            positions.push(pos);
            let scale = rand::thread_rng().gen_range(0.1, intensity);
            let rotate =
                na::geometry::Rotation2::new(rand::thread_rng().gen_range(-spread, spread));
            let vector = scale * (rotate * angle.normalize());
            velocities.push(vector);
        }

        ParticleSystem {
            spritebatch,
            sprites,
            positions,
            velocities,
            duration: Duration::from_secs_f32(1.0),
            start: Instant::now(),
        }
    }

    pub fn upd8(&mut self) {
        // do nothing, this particle system should be dropped.
        if Instant::now() - self.start > self.duration {
            return;
        }
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
