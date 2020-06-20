use ggez::graphics::mint;
use ggez::graphics::Rect;

// Returns a point located at the center of the rectangle. Assumes that the
// upper left corner of the Rect is where (x, y) is and that rectangles
// grow to the right and downwards.
pub fn get_center(rect: Rect) -> mint::Point2<f32> {
    mint::Point2 {
        x: rect.x + rect.w / 2.0,
        y: rect.y + rect.h / 2.0,
    }
}

pub fn from_dims((w, h): (f32, f32)) -> Rect {
    Rect::new(0.0, 0.0, w, h)
}

// Returns a rectangle located at (0, 0) with dimensions (w, h)
pub fn get_dims(rect: Rect) -> Rect {
    Rect::new(0.0, 0.0, rect.w, rect.h)
}

// Return a rectangle the same size as inner, centered along the vertical center
// of outer.
pub fn center_vert(outer: Rect, inner: Rect) -> Rect {
    let point = get_center(outer);
    Rect::new(point.x - inner.w / 2.0, inner.y, inner.w, inner.h)
} // Return a rectangle the same size as inner, centered along the horizontal center
  // of outer.
pub fn center_horiz(outer: Rect, inner: Rect) -> Rect {
    let point = get_center(outer);
    Rect::new(inner.x, point.y - inner.h / 2.0, inner.w, inner.h)
}

// Return a rectangle the same size as inner, centered inside of outer
pub fn center_inside(outer: Rect, inner: Rect) -> Rect {
    let point = get_center(outer);
    center(point.x, point.y, inner.w, inner.h)
}

// Returns a rect such that its center is located (x, y). Assumes that the
// upper left corner of the Rect is where (x, y) is and that rectangles
// grow to the right and downwards.
pub fn center(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(x - w / 2.0, y - h / 2.0, w, h)
}

// Evenly distribute a number of `goal_size` Rect inside of `bounding_box`.
fn distribute_horiz(num_rects: u32, bounding_box: Rect, goal_size: Rect) -> Vec<Rect> {
    let bounding_boxes = divide_horiz(num_rects, bounding_box);
    let rects = bounding_boxes
        .iter()
        .map(|bounding| center_inside(*bounding, goal_size));

    rects.collect()
}

// Evenly divide bounding_box into `num_rects` smaller rects, horizontally.
fn divide_horiz(num_rects: u32, bounding_box: Rect) -> Vec<Rect> {
    let offset_x = bounding_box.x;
    let offset_y = bounding_box.y;
    let width = bounding_box.w / num_rects as f32;
    let height = bounding_box.h;
    let mut rects = vec![];
    for i in 0..num_rects {
        rects.push(Rect::new(
            i as f32 * width + offset_x,
            offset_y,
            width,
            height,
        ));
    }
    rects
}

// Aligns the inner rect to the bottom of the outer rect
fn align_bottom(outer: Rect, inner: Rect) -> Rect {
    let outer_bottom = outer.y + outer.h;
    Rect::new(inner.x, outer_bottom - inner.h, inner.w, inner.h)
}

fn from_points(start_x: f32, start_y: f32, end_x: f32, end_y: f32) -> Rect {
    Rect::new(start_x, start_y, end_x - start_x, end_y - start_y)
}

fn divide_vert(num_rects: u32, bounding_box: Rect) -> Vec<Rect> {
    let offset_x = bounding_box.x;
    let offset_y = bounding_box.y;
    let width = bounding_box.w;
    let height = bounding_box.h / num_rects as f32;
    let mut rects = vec![];
    for i in 0..num_rects {
        rects.push(Rect::new(
            offset_x,
            i as f32 * height + offset_y,
            width,
            height,
        ));
    }
    rects
}
