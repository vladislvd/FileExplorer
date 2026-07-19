#[derive(Default, PartialEq, Clone, Copy)]
pub enum SortBy {
    #[default]
    Date,
    Name,
    Type,
}