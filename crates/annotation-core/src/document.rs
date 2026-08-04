use crate::objects::AnnotationObject;
use crate::style::AnnotationStyle;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationDocument {
    pub objects: Vec<AnnotationObject>,
    #[serde(skip)]
    redo_stack: Vec<AnnotationObject>,
    pub current_style: AnnotationStyle,
}

impl AnnotationDocument {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            redo_stack: Vec::new(),
            current_style: AnnotationStyle::default(),
        }
    }

    pub fn add_object(&mut self, object: AnnotationObject) {
        self.objects.push(object);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> bool {
        if let Some(obj) = self.objects.pop() {
            self.redo_stack.push(obj);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(obj) = self.redo_stack.pop() {
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

    pub fn clear(&mut self) {
        self.objects.clear();
        self.redo_stack.clear();
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn can_undo(&self) -> bool {
        !self.objects.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn hit_test(&self, x: f32, y: f32, tolerance: f32) -> Option<usize> {
        for (i, obj) in self.objects.iter().enumerate().rev() {
            if obj.contains_point(x, y, tolerance) {
                return Some(i);
            }
        }
        None
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for AnnotationDocument {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::AnnotationObject;
    use crate::style::AnnotationStyle;
    use capture_core::Rect;

    fn test_rect() -> AnnotationObject {
        AnnotationObject::Rectangle {
            rect: Rect::new(10, 10, 100, 50),
            style: AnnotationStyle::default(),
        }
    }

    #[test]
    fn test_add_and_count() {
        let mut doc = AnnotationDocument::new();
        assert_eq!(doc.object_count(), 0);
        doc.add_object(test_rect());
        assert_eq!(doc.object_count(), 1);
    }

    #[test]
    fn test_undo_redo() {
        let mut doc = AnnotationDocument::new();
        doc.add_object(test_rect());
        assert!(doc.can_undo());
        assert!(!doc.can_redo());

        assert!(doc.undo());
        assert_eq!(doc.object_count(), 0);
        assert!(!doc.can_undo());

        assert!(doc.redo());
        assert_eq!(doc.object_count(), 1);
    }

    #[test]
    fn test_undo_empty() {
        let mut doc = AnnotationDocument::new();
        assert!(!doc.undo());
    }

    #[test]
    fn test_remove_object() {
        let mut doc = AnnotationDocument::new();
        doc.add_object(test_rect());
        doc.add_object(test_rect());
        assert_eq!(doc.object_count(), 2);

        let removed = doc.remove_object(0);
        assert!(removed.is_some());
        assert_eq!(doc.object_count(), 1);
    }

    #[test]
    fn test_remove_out_of_bounds() {
        let mut doc = AnnotationDocument::new();
        assert!(doc.remove_object(5).is_none());
    }

    #[test]
    fn test_clear() {
        let mut doc = AnnotationDocument::new();
        doc.add_object(test_rect());
        doc.add_object(test_rect());
        doc.clear();
        assert_eq!(doc.object_count(), 0);
        assert!(!doc.can_undo());
        assert!(!doc.can_redo());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut doc = AnnotationDocument::new();
        doc.add_object(test_rect());
        let json = doc.to_json().unwrap();
        let restored = AnnotationDocument::from_json(&json).unwrap();
        assert_eq!(restored.object_count(), 1);
    }

    #[test]
    fn test_hit_test() {
        let mut doc = AnnotationDocument::new();
        doc.add_object(AnnotationObject::Rectangle {
            rect: Rect::new(10, 10, 100, 50),
            style: AnnotationStyle::default(),
        });
        assert_eq!(doc.hit_test(50.0, 30.0, 5.0), Some(0));
        assert_eq!(doc.hit_test(200.0, 200.0, 5.0), None);
    }
}
