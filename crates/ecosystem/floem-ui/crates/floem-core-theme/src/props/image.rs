#[derive(Debug, Clone, Default)]
pub struct ImageProps {
    /// Image source: a local file path or `http(s)://` URL.
    pub src: String,
    /// Alternative text / caption shown when the image cannot be loaded.
    pub alt: String,
    /// Explicit width in dp. When `None` the image fills its container.
    pub width: Option<f32>,
    /// Explicit height in dp. When `None` the image uses its natural height.
    pub height: Option<f32>,
    /// Clip the corners with a rounded radius.
    pub rounded: bool,
    /// Scale the image to cover the box (object-fit: cover).
    pub cover: bool,
}
