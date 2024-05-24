

struct QuadTree<T> {
    elements: (Option<T>, Option<T>, Option<T>, Option<T>),
}

impl<T> QuadTree<T> {

    pub fn new() -> Self {
        Self { 
            elements: (None, None, None, None) 
        }
    }

    pub fn delete(&mut self, x: usize, y: usize) -> Option<T> {
        match (x, y) {
            (0, 0) => {
                self.elements.0.take()
            },
            (1, 0) => {
                self.elements.1.take()
            },
            (0, 1) => {
                self.elements.2.take()
            },
            (1, 1) => {
                self.elements.3.take()
            },
            _ => None
        }
    }

    pub fn set(&mut self, x: usize, y: usize, value: T) -> Option<T> {
        match (x, y) {
            (0, 0) => {
                self.elements.0.replace(value)
            },
            (1, 0) => {
                self.elements.1.replace(value)
            },
            (0, 1) => {
                self.elements.2.replace(value)
            },
            (1, 1) => {
                self.elements.3.replace(value)
            },
            _ => None
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&T> {
        match (x, y) {
            (0, 0) => {
                let Some(inner) = &self.elements.0 else { return None; };
                Some(inner)
            },
            (1, 0) => {
                let Some(inner) = &self.elements.1 else { return None; };
                Some(inner)
            },
            (0, 1) => {
                let Some(inner) = &self.elements.2 else { return None; };
                Some(inner)
            },
            (1, 1) => {
                let Some(inner) = &self.elements.3 else { return None; };
                Some(inner)
            },
            _ => None
        }
    }

}