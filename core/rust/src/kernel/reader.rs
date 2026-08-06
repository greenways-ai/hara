#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct Reader<'a> {
    source: &'a str,
    cursor: usize,
    line: usize,
    column: usize,
    history: Vec<Position>,
}
impl<'a> Reader<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            cursor: 0,
            line: 1,
            column: 1,
            history: Vec::new(),
        }
    }
    pub fn position(&self) -> Position {
        Position {
            offset: self.cursor,
            line: self.line,
            column: self.column,
        }
    }
    pub fn peek_char(&self) -> Option<char> {
        self.source[self.cursor..].chars().next()
    }
    pub fn read_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.history.push(self.position());
        self.cursor += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1
        } else {
            self.column += 1
        }
        Some(ch)
    }
    pub fn unread_char(&mut self) -> Option<char> {
        let position = self.history.pop()?;
        let ch = self.source[position.offset..self.cursor].chars().next();
        self.cursor = position.offset;
        self.line = position.line;
        self.column = position.column;
        ch
    }
    pub fn read_while(&mut self, predicate: impl Fn(char) -> bool) -> String {
        let mut value = String::new();
        while let Some(ch) = self.read_char() {
            if predicate(ch) {
                value.push(ch)
            } else {
                self.unread_char();
                break;
            }
        }
        value
    }
    pub fn read_until(&mut self, predicate: impl Fn(char) -> bool) -> String {
        self.read_while(|ch| !predicate(ch))
    }
    pub fn is_eof(&self) -> bool {
        self.cursor >= self.source.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{Position, Reader};
    #[test]
    fn tracks_unicode_lines_columns_and_unread() {
        let mut r = Reader::new("λ\nx");
        assert_eq!(
            r.position(),
            Position {
                offset: 0,
                line: 1,
                column: 1
            }
        );
        assert_eq!(r.read_char(), Some('λ'));
        assert_eq!(r.position().column, 2);
        r.unread_char();
        assert_eq!(r.position().column, 1);
        r.read_char();
        r.read_char();
        assert_eq!(
            r.position(),
            Position {
                offset: 3,
                line: 2,
                column: 1
            }
        );
    }
}
