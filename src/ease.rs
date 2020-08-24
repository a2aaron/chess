use std::time::{Duration, Instant};

/// A Tween is a struct which will smoothly interpolate over time a value
/// between a specified start and end point according to some easing function.
/// Under the hood, this works by using a start position and an offset which
/// is slowly applied over time. Note that `upd8` must be called before accessing
/// `pos`, as otherwise the value will be out of date.
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
    /// Create a tween using a start position and an offset. This is good for
    /// types like na::Point2 and na::Vector2, as we can't directly add two points
    /// but we can add a point and a vector. The tween's start time is set to
    /// the current time upon creation.
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

    /// Update `pos` using the supplied current time. This is done so that you
    /// can supply a common `now` across many tweens, which avoids doing many
    /// costly (and basically identical) `Instant::now()` calls.
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
    /// Create a new tween that interpolates `pos` between start and end. This
    /// is good for directly interpolating, assuming that the `Position` type
    /// supplied has `Sub<Output = Offset>`. This works well for most number types.
    /// It will also happen to work for Point2 as Point2 happens to implement
    /// `Sub<Output = Vector2>`.
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

/// An enum representing an ease. `ease` expects a value in [0.0, 1.0].
#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum Ease {
    Linear,
    InQuadratic,
    InCubic,
    InQuartic,
    OutQuadratic,
    OutCubic,
    OutQuartic,
    OutBack,
    OutElastic,
    InOutCubic,
    InOutBack,
}

impl Ease {
    /// Return the eased value of `x`, assuming `x` is in the range [0.0, 1.0]
    /// This function does not clamp its output, so you should probably call
    /// clamp if you end up passing a value of `x` that is outside [0.0, 1.0]
    fn ease(&self, x: f32) -> f32 {
        use Ease::*;
        match self {
            Linear => x,
            InQuadratic => x * x,
            InCubic => x * x * x,
            InQuartic => x * x * x * x,
            OutQuadratic => 1.0 - (1.0 - x) * (1.0 - x),
            OutCubic => 1.0 - (1.0 - x) * (1.0 - x) * (1.0 - x),
            OutQuartic => 1.0 - (1.0 - x) * (1.0 - x) * (1.0 - x) * (1.0 - x),
            OutBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                1.0 + c3 * (x - 1.0).powf(3.0) + c1 * (x - 1.0).powf(2.0)
            }
            OutElastic => {
                let c4 = (2.0 * std::f32::consts::PI) / 3.0;
                2.0f32.powf(-10.0 * x) * ((x * 10.0 - 0.75) * c4).sin() + 1.0
            }
            InOutCubic => {
                if x < 0.5 {
                    4.0 * x * x * x
                } else {
                    1.0 - (-2.0 * x + 2.0).powf(3.0) / 2.0
                }
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

    /// Helper method to interpolate an f32. `percent` should be in the range
    /// [0.0, 1.0]. This method does not clamp its output.
    pub fn interpolate(&self, start: f32, end: f32, percent: f32) -> f32 {
        let percent = self.ease(percent);
        return start * (1.0 - percent) + end * percent;
    }
}
