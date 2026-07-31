use crate::objects::AnnotationObject;
use crate::style::AnnotationStyle;

pub struct AnnotationDocument {
    pub objects: Vec<AnnotationObject>,
    pub undo_stack: Vec<AnnotationObject>,
    pub redo_stack: Vec<AnnotationObject>,
    pub current_style: AnnotationStyle,
    max_history: usize,
}

impl AnnotationDocument {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_style: AnnotationStyle::default(),
            max_history: 50,
        }
    }

    pub fn add_object(&mut self, object: AnnotationObject) {
        self.objects.push(object);
        self.redo_stack.clear();
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) -> bool {
        if let Some(obj) = self.objects.pop() {
            self.undo_stack.push(obj);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(obj) = self.undo_stack.pop() {
            self.objects.push(obj);
            true
        } else {
            false
        }
    }

    pub fn remove_object(&mut self, index: usize) -> Option<AnnotationObject> {
        if index < self.objects.len() {
            let obj = self.objects.remove(index);
            self.redo_stack.clear();
            Some(obj)
        } else {
            None
        }
    }
}

impl Default for AnnotationDocument {
    fn default() -> Self {
        Self::new()
    }
}
