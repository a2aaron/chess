use std::time::{Duration, Instant};

use rand::{self, Rng};

use ggez::graphics::spritebatch::{SpriteBatch, SpriteIdx};
use ggez::graphics::{self, Color, DrawParam, Image};
use ggez::nalgebra as na;
use ggez::{Context, GameResult};

use crate::ease::{Ease, Tween};

const PI: f32 = std::f32::consts::PI;

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
    positions: Vec<Tween<na::Point2<f32>, na::Vector2<f32>>>,
    sizes: Vec<Tween<f32>>,
    rotations: Vec<Tween<f32>>,
    colors: Vec<Color>,
    duration: Duration,
    start_time: Instant,
}

impl ParticleSystem {
    pub fn new(
        ctx: &mut Context,
        start: na::Point2<f32>,
        angle: na::Vector2<f32>,
        spread: f32,
        intensity: f32,
        size: u16,
        num_particles: usize,
    ) -> ParticleSystem {
        // TODO: pass in an image instead of making one right here
        let image = Image::solid(ctx, size, Color::from_rgb_u32(0xFF0000)).unwrap();
        let mut spritebatch = SpriteBatch::new(image);
        let mut sprites = vec![];
        let mut positions = vec![];
        let mut sizes = vec![];
        let mut rotations = vec![];
        let mut colors = vec![];
        for _ in 0..num_particles {
            sprites.push(spritebatch.add(DrawParam::default()));
            let rotate =
                na::geometry::Rotation2::new(rand::thread_rng().gen_range(-spread, spread));
            let scale = rand::thread_rng().gen_range(0.1, intensity);
            let duration = rand::thread_rng().gen_range(0.3, 0.5);
            let offset: na::Vector2<f32> = scale * (rotate * angle.normalize());
            let pos = Tween::offset(
                Ease::OutQuadratic,
                start,
                offset,
                Duration::from_secs_f32(duration),
            );
            positions.push(pos);
            sizes.push(Tween::new(
                Ease::OutQuadratic,
                1.0,
                2.0,
                Duration::from_secs_f32(1.0),
            ));

            rotations.push(Tween::new(
                Ease::OutQuadratic,
                rand::thread_rng().gen_range(-PI, PI),
                rand::thread_rng().gen_range(-PI * 2.0, PI * 2.0),
                Duration::from_secs_f32(duration),
            ));

            colors.push(Color::from_rgb(
                rand::thread_rng().gen_range(200, 255),
                0,
                0,
            ));
        }

        ParticleSystem {
            spritebatch,
            sprites,
            positions,
            sizes,
            colors,
            rotations,
            duration: Duration::from_secs_f32(1.0),
            start_time: Instant::now(),
        }
    }

    pub fn upd8(&mut self) {
        // do nothing, this particle system is done.
        if self.start_time.elapsed() > self.duration {
            return;
        }
        let now = Instant::now();
        for tween in &mut self.positions {
            tween.upd8(now);
        }

        for tween in &mut self.sizes {
            tween.upd8(now);
        }

        for tween in &mut self.rotations {
            tween.upd8(now);
        }
    }

    pub fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        for ((((&sprite, pos), size), rotation), &color) in self
            .sprites
            .iter()
            .zip(&self.positions)
            .zip(&self.sizes)
            .zip(&self.rotations)
            .zip(&self.colors)
        {
            self.spritebatch.set(
                sprite,
                DrawParam::default()
                    .offset(na::Point2::new(0.5, 0.5))
                    .dest(pos.pos)
                    .scale(na::Vector2::new(size.pos, size.pos))
                    .rotation(rotation.pos)
                    .color(color),
            )?;
        }

        graphics::draw(ctx, &self.spritebatch, DrawParam::default())
    }
}
