use itertools::Itertools;

/// Term that could appear in search query
pub enum Term<'q> {
    /// Just a tag (is_positive, body)
    Tag(bool, &'q str),
    /// Element group
    Group(u32),
}

/// Creates an iterator that will output parsed query parts
pub fn parse_query<'q>(query: &'q str) -> impl Iterator<Item = Term<'q>> {
    query.split_whitespace()
        .filter_map(|term| {
            if term.contains(":") {
                let (left, right) = term
                    .split(':')
                    .tuples()
                    .next()?;

                match (left, right) {
                    ("group", id) => id.parse().ok().map(|id| Term::Group(id)),
                    _ => None,
                }
            } else {
                let pos = !term.starts_with("!");
                Some(Term::Tag(pos, if pos { term } else { &term[1..] }))
            }
        })
}