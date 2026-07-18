use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Segment {
    pub start: f32,
    pub end: f32,
    pub speaker: String,
}

impl Segment {
    pub fn duration(&self) -> f32 {
        self.end - self.start
    }
}
