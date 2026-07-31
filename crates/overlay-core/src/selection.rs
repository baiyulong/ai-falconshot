use capture_core::Rect;

#[derive(Debug, Clone)]
pub struct SelectionModel {
    pub rect: Rect,
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

impl SelectionModel {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            rect: Rect {
                x,
                y,
                width: 0,
                height: 0,
            },
            is_dragging: false,
            drag_handle: None,
        }
    }

    pub fn update_drag(&mut self, x: i32, y: i32) {
        let start_x = self.rect.x;
        let start_y = self.rect.y;
        self.rect.x = start_x.min(x);
        self.rect.y = start_y.min(y);
        self.rect.width = (x - start_x).unsigned_abs();
        self.rect.height = (y - start_y).unsigned_abs();
    }

    pub fn is_valid(&self) -> bool {
        self.rect.width > 2 && self.rect.height > 2
    }
}
