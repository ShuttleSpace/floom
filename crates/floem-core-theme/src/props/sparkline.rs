pub struct SparklineProps {
    /// Series of values rendered as a mini bar chart.
    pub values: Vec<f32>,
    /// Rendered width in px.
    pub width: f32,
    /// Rendered height in px.
    pub height: f32,
    /// When true, bars use a translucent fill; otherwise solid.
    pub fill: bool,
    /// Width of each bar in px.
    pub line_width: f32,
}

impl Clone for SparklineProps {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            width: self.width,
            height: self.height,
            fill: self.fill,
            line_width: self.line_width,
        }
    }
}

impl std::fmt::Debug for SparklineProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SparklineProps")
            .field("values", &self.values)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("fill", &self.fill)
            .field("line_width", &self.line_width)
            .finish()
    }
}

impl Default for SparklineProps {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            width: 200.0,
            height: 48.0,
            fill: true,
            line_width: 4.0,
        }
    }
}
