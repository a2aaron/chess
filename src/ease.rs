use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Tween<Position, Offset = Position> {
    ease: Ease,
    start: Position,
    pub pos: Position,
    offset: Offset,
    start_time: Instant,
    duration: Duration,
}

impl<Position, Offset> Tween<Position, Offset>
where
    Position: std::ops::Add<Offset, Output = Position> + Copy,
    Offset: Copy,
    f32: std::ops::Mul<Offset, Output = Offset>,
{
    pub fn offset(
        ease: Ease,
        start: Position,
        offset: Offset,
        duration: Duration,
    ) -> Tween<Position, Offset> {
        Tween {
            ease,
            start,
            pos: start,
            offset,
            duration,
            start_time: Instant::now(),
        }
    }

    pub fn upd8(&mut self, now: Instant) {
        let percent = (now - self.start_time)
            .div_duration_f32(self.duration)
            .clamp(0.0, 1.0);
        self.pos = self.start + self.ease.ease(percent) * self.offset
    }
}

impl<Position, Offset> Tween<Position, Offset>
where
    Position: std::ops::Sub<Position, Output = Offset> + Copy,
{
    pub fn new(
        ease: Ease,
        start: Position,
        end: Position,
        duration: Duration,
    ) -> Tween<Position, Offset> {
        Tween {
            ease,
            start,
            pos: start,
            offset: end - start,
            duration,
            start_time: Instant::now(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum Ease {
    Linear,
    OutQuadratic,
    InOutCubic,
    OutElastic,
    OutBack,
    InOutBack,
}

impl Ease {
    fn ease(&self, x: f32) -> f32 {
        use Ease::*;
        match self {
            Linear => x,
            OutQuadratic => 1.0 - (1.0 - x) * (1.0 - x),
            InOutCubic => {
                if x < 0.5 {
                    4.0 * x * x * x
                } else {
                    1.0 - (-2.0 * x + 2.0).powf(3.0) / 2.0
                }
            }
            OutElastic => {
                let c4 = (2.0 * std::f32::consts::PI) / 3.0;
                2.0f32.powf(-10.0 * x) * ((x * 10.0 - 0.75) * c4).sin() + 1.0
            }
            OutBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                1.0 + c3 * (x - 1.0).powf(3.0) + c1 * (x - 1.0).powf(2.0)
            }
            InOutBack => {
                let c1 = 1.70158;
                let c2 = c1 * 1.525;
                if x < 0.5 {
                    ((2.0 * x).powf(2.0) * ((c2 + 1.0) * 2.0 * x - c2)) / 2.0
                } else {
                    ((2.0 * x - 2.0).powf(2.0) * ((c2 + 1.0) * (x * 2.0 - 2.0) + c2) + 2.0) / 2.0
                }
            }
        }
    }

    pub fn interpolate(&self, start: f32, end: f32, percent: f32) -> f32 {
        let percent = self.ease(percent);
        return start * (1.0 - percent) + end * percent;
    }
}
