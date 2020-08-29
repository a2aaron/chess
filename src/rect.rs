use ggez::graphics::{mint, Rect};

/// Returns a point located at the center of `rect`. Assumes that `rect.x` and
/// `rect.y` define the upper left corner of the Rect and that rectangles
/// grow to the right and downwards.
pub fn get_center(rect: Rect) -> mint::Point2<f32> {
    mint::Point2 {
        x: rect.x + rect.w / 2.0,
        y: rect.y + rect.h / 2.0,
    }
}

/// Returns a rectangle located at (0, 0) with dimensions (w, h)
pub fn from_dims((w, h): (f32, f32)) -> Rect {
    Rect::new(0.0, 0.0, w, h)
}

/// Returns a rectangle located at (0, 0) with the dimensions of `rect`
pub fn get_dims(rect: Rect) -> Rect {
    Rect::new(0.0, 0.0, rect.w, rect.h)
}

// Return a rectangle the same size as `inner`, centered along the vertical center
// of `outer`.
pub fn center_vert(outer: Rect, inner: Rect) -> Rect {
    let point = get_center(outer);
    Rect::new(point.x - inner.w / 2.0, inner.y, inner.w, inner.h)
}

// Return a rectangle the same size as `inner`, centered along the horizontal center
// of `outer`.
pub fn center_horiz(outer: Rect, inner: Rect) -> Rect {
    let point = get_center(outer);
    Rect::new(inner.x, point.y - inner.h / 2.0, inner.w, inner.h)
}

// Return a rectangle the same size as `inner`, centered inside of `outer`
pub fn center_inside(outer: Rect, inner: Rect) -> Rect {
    let point = get_center(outer);
    center(point.x, point.y, inner.w, inner.h)
}

// Returns a rect such that its center is located at (x, y). Assumes that the
// upper left corner of the Rect is where (rect.x, rect.y) is and that rectangles
// grow to the right and downwards.
pub fn center(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(x - w / 2.0, y - h / 2.0, w, h)
}
