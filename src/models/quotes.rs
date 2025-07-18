#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quote {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub aliases: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuoteAlias {
    pub id: i64,
    pub quote_id: i64,
    pub alias: String,
}
