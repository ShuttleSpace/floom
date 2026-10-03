#[derive(Debug, Clone)]
pub struct CardProps {
    pub elevation: u8,
}
impl Default for CardProps {
    fn default() -> Self {
        Self { elevation: 1 }
    }
}
