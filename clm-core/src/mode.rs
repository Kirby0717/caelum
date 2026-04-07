#[derive(Debug, Clone, Copy, Default)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Command,
}
