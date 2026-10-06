//! Just enough CSV for the broker exports we read: a delimiter, optional
//! double-quoted fields (with `""` as an escaped quote) and one record per
//! line. None of the supported exports put line breaks inside fields.

/// Decodes raw file bytes, dropping a UTF-8 BOM. Files that are not valid
/// UTF-8 are assumed to be Latin-1, which older Swedish bank exports use.
pub fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    }
}

/// Splits `text` into records of fields, skipping blank lines.
pub fn parse(text: &str, delimiter: char) -> Vec<Vec<String>> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| split_line(line, delimiter))
        .collect()
}

fn split_line(line: &str, delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = line.trim_end_matches('\r').chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            c if c == delimiter && !in_quotes => fields.push(std::mem::take(&mut field)),
            c => field.push(c),
        }
    }
    fields.push(field);
    fields
}

/// Parses a number written either as `1234.56` or Swedish-style `1 234,56`.
/// Blank cells and placeholders such as `-` yield `None`.
pub fn number(cell: &str) -> Option<f64> {
    let cleaned: String = cell
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{a0}')
        .map(|c| if c == ',' { '.' } else { c })
        .collect();
    if cleaned.is_empty() || cleaned == "-" {
        return None;
    }
    cleaned.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_quoted_fields_containing_the_delimiter() {
        let rows = parse("\"a\",\"b,c\",3.5\n\n\"d\"\"e\",,x\r\n", ',');
        assert_eq!(rows, vec![vec!["a", "b,c", "3.5"], vec!["d\"e", "", "x"]]);
    }

    #[test]
    fn parses_swedish_and_plain_numbers() {
        assert_eq!(number("-11,827556"), Some(-11.827_556));
        assert_eq!(number("1 234,5"), Some(1234.5));
        assert_eq!(number("0.02548000"), Some(0.025_48));
        assert_eq!(number(""), None);
        assert_eq!(number("-"), None);
    }

    #[test]
    fn decodes_bom_and_latin1() {
        assert_eq!(decode(b"\xEF\xBB\xBFDatum"), "Datum");
        assert_eq!(decode(b"V\xE4rde"), "Värde");
    }
}
