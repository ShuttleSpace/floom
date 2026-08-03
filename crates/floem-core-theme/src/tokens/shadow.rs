#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowData {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color_a: u8,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ElevationTokens {
    pub levels: [ShadowData; 6],
}
impl Default for ElevationTokens {
    fn default() -> Self {
        Self {
            levels: [
                ShadowData {
                    offset_x: 0.0,
                    offset_y: 1.0,
                    blur: 2.0,
                    spread: 0.0,
                    color_a: 30,
                },
                ShadowData {
                    offset_x: 0.0,
                    offset_y: 2.0,
                    blur: 4.0,
                    spread: 0.0,
                    color_a: 40,
                },
                ShadowData {
                    offset_x: 0.0,
                    offset_y: 4.0,
                    blur: 8.0,
                    spread: 0.0,
                    color_a: 50,
                },
                ShadowData {
                    offset_x: 0.0,
                    offset_y: 6.0,
                    blur: 12.0,
                    spread: 0.0,
                    color_a: 60,
                },
                ShadowData {
                    offset_x: 0.0,
                    offset_y: 8.0,
                    blur: 16.0,
                    spread: 0.0,
                    color_a: 70,
                },
                ShadowData {
                    offset_x: 0.0,
                    offset_y: 12.0,
                    blur: 24.0,
                    spread: 0.0,
                    color_a: 80,
                },
            ],
        }
    }
}
