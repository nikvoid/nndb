use std::ops::Range;
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;

/// Tag excape regex
pub static TAG_REX: Lazy<Regex> = Lazy::new(|| 
    Regex::new(r#"[\s:,.@#$*'"`|%{}\[\]]+"#).unwrap()
);


/// Term that could appear in search query
#[derive(Debug, PartialEq)]
pub enum Term<'q> {
    /// Just a tag (is_positive, body)
    Tag(bool, &'q str),
    /// Element group
    Group(u32),
    /// External element group
    ExtGroup(u32),
}

/// Creates an iterator that will output parsed query parts
pub fn parse_query(query: &str) -> impl Iterator<Item = Term<'_>> {
    parse_query_with_span(query)
        .map(|(.., term)| term)
}

/// Creates an iterator that will output parsed query parts with source span.
///
/// Returns `(byte_span, char_span, term)`
pub fn parse_query_with_span(
    query: &str
) -> impl Iterator<Item = (Range<usize>, Range<usize>, Term<'_>)> {
    query
        // Extract words with their spans
        .char_indices()
        .enumerate()
        // End padding
        .chain([(query.chars().count(), (query.len(), ' '))])
        .scan((0..0, 0..0, false), |(span, char_span, space), (char_idx, (idx, chr))| {
            let space_prev = *space;
            // TODO: Char span
            *space = chr.is_whitespace();
            match (space_prev, chr.is_whitespace()) {
                (true, true)
                | (false, false) => 
                    Some(None),
                // Word-start boundary
                (true, false) => {
                    span.start = idx;
                    char_span.start = char_idx;
                    Some(None)
                },
                // Word-end boundary
                (false, true) => {
                    span.end = idx;
                    char_span.end = char_idx;
                    // Special case for start
                    if idx == 0 {
                        Some(None)
                    } else {
                        Some(Some((span.clone(), char_span.clone(), &query[span.clone()])))
                    }
                },
            }
        })
        .flatten()
        .filter_map(|(span, char_span, term)| {
            if term.contains(':') {
                let (left, right) = term
                    .split(':')
                    .tuples()
                    .next()?;

                match (left, right) {
                    ("group", id) => id.parse().ok().map(Term::Group),
                    ("extgroup", id) => id.parse().ok().map(Term::ExtGroup),
                    _ => None,
                }
                .map(move |term| (span, char_span, term))
            } else if !TAG_REX.is_match(term) {
                // Allow only valid tags
                let pos = !term.starts_with('!');
                Some((span, char_span, Term::Tag(pos, if pos { term } else { &term[1..] })))
            } else {
                None
            }
        })
}

#[test]
fn test_parse_with_span() {
    let query = "abc def \t sad !tag grp:1     \tgroup:1 extgroup:50  тег !нетег";
    let terms: Vec<_> = parse_query_with_span(query).collect();
    assert_eq!(
        [
            (0..3, 0..3, Term::Tag(true, "abc")),
            (4..7, 4..7, Term::Tag(true, "def")),
            (10..13, 10..13, Term::Tag(true, "sad")),
            (14..18, 14..18, Term::Tag(false, "tag")),
            (30..37, 30..37, Term::Group(1)),
            (38..49, 38..49, Term::ExtGroup(50)),
            (51..57, 51..54, Term::Tag(true, "тег")), 
            (58..69, 55..61, Term::Tag(false, "нетег"))
        ].as_slice(), 
        terms.as_slice()
    );
}