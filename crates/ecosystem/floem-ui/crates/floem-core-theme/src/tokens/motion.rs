#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionTokens {
    pub duration_short: f32,
    pub duration_medium: f32,
    pub duration_long: f32,
    pub easing_standard: Easing,
    pub easing_decelerate: Easing,
    pub easing_accelerate: Easing,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Easing {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}
impl Default for MotionTokens {
    fn default() -> Self {
        Self {
            duration_short: 150.0,
            duration_medium: 300.0,
            duration_long: 500.0,
            easing_standard: Easing {
                x1: 0.4,
                y1: 0.0,
                x2: 0.2,
                y2: 1.0,
            },
            easing_decelerate: Easing {
                x1: 0.0,
                y1: 0.0,
                x2: 0.2,
                y2: 1.0,
            },
            easing_accelerate: Easing {
                x1: 0.4,
                y1: 0.0,
                x2: 1.0,
                y2: 1.0,
            },
        }
    }
}
