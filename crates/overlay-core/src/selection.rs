use capture_core::Rect;

#[derive(Debug, Clone)]
pub struct SelectionModel {
    pub rect: Rect,
    pub anchor: (i32, i32),
    pub is_dragging: bool,
    pub drag_handle: Option<DragHandle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Top,
    Bottom,
    Left,
    Right,
    Move,
}

const HANDLE_SIZE: i32 = 8;

impl SelectionModel {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            rect: Rect::new(x, y, 0, 0),
            anchor: (x, y),
            is_dragging: false,
            drag_handle: None,
        }
    }

    pub fn start_drag(&mut self, x: i32, y: i32) {
        self.anchor = (x, y);
        self.rect = Rect::new(x, y, 0, 0);
        self.is_dragging = true;
        self.drag_handle = None;
    }

    pub fn update_drag(&mut self, x: i32, y: i32) {
        if !self.is_dragging {
            return;
        }
        let (ax, ay) = self.anchor;
        self.rect = Rect::new(ax.min(x), ay.min(y), (x - ax).unsigned_abs(), (y - ay).unsigned_abs());
    }

    pub fn end_drag(&mut self) {
        self.is_dragging = false;
        self.drag_handle = None;
    }

    pub fn is_valid(&self) -> bool {
        self.rect.width > 2 && self.rect.height > 2
    }

    pub fn hit_test_handle(&self, x: i32, y: i32) -> Option<DragHandle> {
        if !self.is_valid() {
            return None;
        }
        let r = &self.rect;
        let hs = HANDLE_SIZE;

        let corners = [
            (DragHandle::TopLeft, r.x, r.y),
            (DragHandle::TopRight, r.right(), r.y),
            (DragHandle::BottomLeft, r.x, r.bottom()),
            (DragHandle::BottomRight, r.right(), r.bottom()),
        ];
        for (handle, hx, hy) in &corners {
            if (x - hx).abs() <= hs && (y - hy).abs() <= hs {
                return Some(*handle);
            }
        }

        let mid_x = r.x + r.width as i32 / 2;
        let mid_y = r.y + r.height as i32 / 2;
        let edges = [
            (DragHandle::Top, mid_x, r.y),
            (DragHandle::Bottom, mid_x, r.bottom()),
            (DragHandle::Left, r.x, mid_y),
            (DragHandle::Right, r.right(), mid_y),
        ];
        for (handle, hx, hy) in &edges {
            if (x - hx).abs() <= hs && (y - hy).abs() <= hs {
                return Some(*handle);
            }
        }

        if r.contains_point(x, y) {
            return Some(DragHandle::Move);
        }
        None
    }

    pub fn start_resize(&mut self, handle: DragHandle, x: i32, y: i32) {
        self.is_dragging = true;
        self.drag_handle = Some(handle);
        self.anchor = (x, y);
    }

    pub fn update_resize(&mut self, x: i32, y: i32) {
        let handle = match self.drag_handle {
            Some(h) => h,
            None => return,
        };
        let mut left = self.rect.x;
        let mut top = self.rect.y;
        let mut right = self.rect.right();
        let mut bottom = self.rect.bottom();

        match handle {
            DragHandle::TopLeft => { left = x; top = y; }
            DragHandle::TopRight => { right = x; top = y; }
            DragHandle::BottomLeft => { left = x; bottom = y; }
            DragHandle::BottomRight => { right = x; bottom = y; }
            DragHandle::Top => { top = y; }
            DragHandle::Bottom => { bottom = y; }
            DragHandle::Left => { left = x; }
            DragHandle::Right => { right = x; }
            DragHandle::Move => {
                let dx = x - self.anchor.0;
                let dy = y - self.anchor.1;
                self.rect.x += dx;
                self.rect.y += dy;
                self.anchor = (x, y);
                return;
            }
        }

        self.rect = Rect::new(
            left.min(right),
            top.min(bottom),
            (right - left).unsigned_abs(),
            (bottom - top).unsigned_abs(),
        );
    }

    pub fn clamp_to_bounds(&mut self, bounds: &Rect) {
        let x = self.rect.x.max(bounds.x);
        let y = self.rect.y.max(bounds.y);
        let mut w = self.rect.width.min((bounds.right() - x) as u32);
        let mut h = self.rect.height.min((bounds.bottom() - y) as u32);
        if x + w as i32 > bounds.right() {
            w = (bounds.right() - x) as u32;
        }
        if y + h as i32 > bounds.bottom() {
            h = (bounds.bottom() - y) as u32;
        }
        self.rect = Rect::new(x, y, w, h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_selection() {
        let sel = SelectionModel::new(100, 200);
        assert_eq!(sel.rect, Rect::new(100, 200, 0, 0));
        assert!(!sel.is_valid());
    }

    #[test]
    fn test_drag_creates_rect() {
        let mut sel = SelectionModel::new(100, 100);
        sel.start_drag(100, 100);
        sel.update_drag(200, 150);
        assert_eq!(sel.rect, Rect::new(100, 100, 100, 50));
        assert!(sel.is_valid());
    }

    #[test]
    fn test_drag_negative_direction() {
        let mut sel = SelectionModel::new(200, 200);
        sel.start_drag(200, 200);
        sel.update_drag(100, 150);
        assert_eq!(sel.rect, Rect::new(100, 150, 100, 50));
    }

    #[test]
    fn test_hit_test_corners() {
        let mut sel = SelectionModel::new(0, 0);
        sel.start_drag(100, 100);
        sel.update_drag(300, 300);
        sel.end_drag();

        assert_eq!(sel.hit_test_handle(100, 100), Some(DragHandle::TopLeft));
        assert_eq!(sel.hit_test_handle(300, 100), Some(DragHandle::TopRight));
        assert_eq!(sel.hit_test_handle(100, 300), Some(DragHandle::BottomLeft));
        assert_eq!(sel.hit_test_handle(300, 300), Some(DragHandle::BottomRight));
    }

    #[test]
    fn test_hit_test_edges() {
        let mut sel = SelectionModel::new(0, 0);
        sel.start_drag(100, 100);
        sel.update_drag(300, 300);
        sel.end_drag();

        assert_eq!(sel.hit_test_handle(200, 100), Some(DragHandle::Top));
        assert_eq!(sel.hit_test_handle(200, 300), Some(DragHandle::Bottom));
        assert_eq!(sel.hit_test_handle(100, 200), Some(DragHandle::Left));
        assert_eq!(sel.hit_test_handle(300, 200), Some(DragHandle::Right));
    }

    #[test]
    fn test_hit_test_move() {
        let mut sel = SelectionModel::new(0, 0);
        sel.start_drag(100, 100);
        sel.update_drag(300, 300);
        sel.end_drag();

        assert_eq!(sel.hit_test_handle(200, 200), Some(DragHandle::Move));
        assert_eq!(sel.hit_test_handle(50, 50), None);
    }

    #[test]
    fn test_resize_bottom_right() {
        let mut sel = SelectionModel::new(0, 0);
        sel.start_drag(100, 100);
        sel.update_drag(300, 300);
        sel.end_drag();

        sel.start_resize(DragHandle::BottomRight, 300, 300);
        sel.update_resize(400, 350);
        assert_eq!(sel.rect, Rect::new(100, 100, 300, 250));
    }

    #[test]
    fn test_resize_top_left() {
        let mut sel = SelectionModel::new(0, 0);
        sel.start_drag(100, 100);
        sel.update_drag(300, 300);
        sel.end_drag();

        sel.start_resize(DragHandle::TopLeft, 100, 100);
        sel.update_resize(50, 50);
        assert_eq!(sel.rect, Rect::new(50, 50, 250, 250));
    }

    #[test]
    fn test_move() {
        let mut sel = SelectionModel::new(0, 0);
        sel.start_drag(100, 100);
        sel.update_drag(200, 200);
        sel.end_drag();

        sel.start_resize(DragHandle::Move, 150, 150);
        sel.update_resize(180, 170);
        assert_eq!(sel.rect, Rect::new(130, 120, 100, 100));
    }

    #[test]
    fn test_clamp_to_bounds() {
        let mut sel = SelectionModel::new(0, 0);
        sel.start_drag(-50, -50);
        sel.update_drag(2000, 2000);
        sel.end_drag();

        let bounds = Rect::new(0, 0, 1920, 1080);
        sel.clamp_to_bounds(&bounds);
        assert_eq!(sel.rect, Rect::new(0, 0, 1920, 1080));
    }
}
