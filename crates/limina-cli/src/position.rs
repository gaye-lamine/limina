//! Character offsets to line and column, and the paragraphs of a file.

/// Line and column (both 1-based, counted in characters) of the character
/// at `char_offset`. Lines end at `\n`, so the `\r` of a CRLF ending never
/// shifts a column on the next line. An offset past the end gives the
/// position just after the last character.
pub fn line_col(text: &str, char_offset: usize) -> (usize, usize) {
    let (mut line, mut column) = (1, 1);
    for ch in text.chars().take(char_offset) {
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

/// A block of non-blank lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paragraph<'a> {
    /// Character offset of its first character in the file.
    pub start: usize,
    /// Its text, without the trailing line ending.
    pub text: &'a str,
}

/// The paragraphs of `text`: blocks of lines separated by blank lines.
pub fn paragraphs(text: &str) -> Vec<Paragraph<'_>> {
    let mut out = Vec::new();
    // (byte start, char start) of the open paragraph, and its byte end.
    let mut open: Option<(usize, usize)> = None;
    let mut end = 0;
    let (mut byte, mut chars) = (0, 0);
    for line in text.split_inclusive('\n') {
        if line.trim().is_empty() {
            if let Some((b, c)) = open.take() {
                out.push(Paragraph {
                    start: c,
                    text: text[b..end].trim_end(),
                });
            }
        } else {
            if open.is_none() {
                open = Some((byte, chars));
            }
            end = byte + line.len();
        }
        byte += line.len();
        chars += line.chars().count();
    }
    if let Some((b, c)) = open {
        out.push(Paragraph {
            start: c,
            text: text[b..end].trim_end(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_positions() {
        let text = "ab\ncd\n\nef";
        assert_eq!(line_col(text, 0), (1, 1));
        assert_eq!(line_col(text, 1), (1, 2));
        assert_eq!(line_col(text, 3), (2, 1));
        assert_eq!(line_col(text, 4), (2, 2));
        assert_eq!(line_col(text, 7), (4, 1));
        assert_eq!(line_col(text, 99), (4, 3));
    }

    #[test]
    fn multi_byte_positions_count_characters() {
        // "é" is 2 bytes, "→" 3, "😀" 4: columns count characters.
        let text = "é→😀x\nçy";
        assert_eq!(line_col(text, 3), (1, 4));
        assert_eq!(line_col(text, 6), (2, 2));
        let start = text.find('y').expect("y");
        assert_eq!(line_col(text, text[..start].chars().count()), (2, 2));
    }

    #[test]
    fn crlf_does_not_shift_columns() {
        let text = "a\r\nbc";
        assert_eq!(line_col(text, 3), (2, 1));
        assert_eq!(line_col(text, 4), (2, 2));
    }

    #[test]
    fn paragraphs_keep_their_character_start() {
        let text = "é one\ntwo\n\n  \r\nthree 3\r\n";
        let p = paragraphs(text);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].text, "é one\ntwo");
        assert_eq!(p[0].start, 0);
        assert_eq!(p[1].text, "three 3");
        assert_eq!(line_col(text, p[1].start), (5, 1));
        let offset: usize = text.chars().take(p[1].start).count();
        assert_eq!(
            text.chars().skip(offset).take(5).collect::<String>(),
            "three"
        );
    }
}
