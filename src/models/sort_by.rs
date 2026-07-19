#[derive(Default, PartialEq, Clone)]
pub enum SortBy {
    #[default]
    Date,
    Name,
    Type,
}