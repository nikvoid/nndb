use std::ops::Range;
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
    /// Search in external metadata
    Meta(&'q str),
    /// Raw text that do not match existing patterns
    Raw(&'q str),
}

/// Creates an iterator that will output parsed query parts
pub fn parse_query(query: &str) -> impl Iterator<Item = Term<'_>> {
    parse_query_with_span(query)
        .map(|(.., term)| term)
}


/// Parses single search term
pub fn parse_term(term: &str) -> Option<Term<'_>> {
    if term.is_empty() {
        None
    } else if term.contains(':') {
        let (left, right) = term.split_once(':')?;

        // Right part could be quoted
        match (left, right.trim_matches('"')) {
            ("group", id) => id.parse().ok().map(Term::Group),
            ("extgroup", id) => id.parse().ok().map(Term::ExtGroup),
            ("meta", text) => Some(Term::Meta(text)),            
            _ => Some(Term::Raw(term)),
        }
    } else if !TAG_REX.is_match(term) {
        // Allow only valid tags
        let pos = !term.starts_with('!');
        Some(Term::Tag(pos, if pos { term } else { &term[1..] }))
    } else {
        Some(Term::Raw(term))
    }
}

/// Creates an iterator that will output parsed query parts with source span.
///
/// Returns `(byte_span, char_span, term)`
pub fn parse_query_with_span(
    query: &str
) -> impl Iterator<Item = (Range<usize>, Range<usize>, Term<'_>)> {
    
    let mut span = 0..0;
    let mut char_span = 0..0;
    let mut in_quote = false;
    let mut iter = query.chars().peekable();

    std::iter::from_fn(move || {
        loop {
            return match iter.next()? {
                // Word end boundary
                ch if ch.is_whitespace() && !in_quote || iter.peek().is_none() => {
                    let mut span_n = span.clone();
                    let mut char_span_n = char_span.clone();

                    // Move span
                    span.end += ch.len_utf8();
                    char_span.end += 1;
                    span.start = span.end;
                    char_span.start = char_span.end;

                    // Special case for last char
                    if iter.peek().is_none() {
                        span_n.end += ch.len_utf8();
                        char_span_n.end += 1;
                    }
                    
                    // This will discard empty strings
                    match parse_term(&query[span_n.clone()]) {
                        Some(term) => {
                            Some((span_n, char_span_n, term))
                        },
                        None => continue
                    }
                },
                
                // Text
                ch => {
                    // Quote
                    if ch == '"' {
                        in_quote = !in_quote;
                    }
                    
                    span.end += ch.len_utf8();
                    char_span.end += 1;
                    continue;
                }
            }
        }
    })
}

#[test]
fn test_parse_with_span() {
    let query = "abc def \t sad !tag grp:1     \tgroup:1 extgroup:50  тег !нетег meta:\"quo ted: sequence\"  end";
    let terms: Vec<_> = parse_query_with_span(query).collect();
    assert_eq!(
        [
            (0..3, 0..3, Term::Tag(true, "abc")),
            (4..7, 4..7, Term::Tag(true, "def")),
            (10..13, 10..13, Term::Tag(true, "sad")),
            (14..18, 14..18, Term::Tag(false, "tag")),
            (19..24, 19..24, Term::Raw("grp:1")),
            (30..37, 30..37, Term::Group(1)),
            (38..49, 38..49, Term::ExtGroup(50)),
            (51..57, 51..54, Term::Tag(true, "тег")), 
            (58..69, 55..61, Term::Tag(false, "нетег")),
            (70..94, 62..86, Term::Meta("quo ted: sequence")),
            (96..99, 88..91, Term::Tag(true, "end"))
        ].as_slice(), 
        terms.as_slice()
    );
}