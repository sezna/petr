use swim_utils::{SourceId, Span, SpannedItem};

use logos::Logos;

use crate::IndexMap;
#[derive(Debug, Logos, PartialEq)]
#[logos(skip r"[ \t]+")]
enum Token {
    #[token("(")]
    OpenParen,
    #[token(")")]
    CloseParen,
    #[token("[")]
    OpenBracket,
    #[token("]")]
    CloseBracket,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("/")]
    Divide,
    #[token("*")]
    Multiply,
    #[regex("[0-9]+")]
    Integer,
    #[regex("[_a-zA-Z][_a-zA-Z0-9]{0,30}")]
    Identifier,
    Eof,
}

pub type LexedSources<'a> = IndexMap<SourceId, (&'a str, logos::Lexer<'a, Token>)>;

pub struct Lexer<'a> {
    sources: LexedSources<'a>,
    source: SourceId,
}

impl<'a> Lexer<'a> {
    pub fn new(sources: impl IntoIterator<Item = &'a str>) -> Self {
        let mut map: IndexMap<_, _> = Default::default();
        let sources = sources.into_iter();
        for source in sources {
            let lexer = Token::lexer(source);
            map.insert((source, lexer));
        }
        Self {
            sources: map,
            source: 0.into(),
        }
    }

    pub fn span(&self) -> Span {
        Span::new(self.source, self.current_lexer().span().into())
    }

    pub fn advance(&mut self) -> SpannedItem<Token> {
        let pre_advance_span = self.span();
        let current_lexer = self.current_lexer_mut();

        match current_lexer.next() {
            None => match self.advance_lexer() {
                Some(_) => return self.advance(),
                None => pre_advance_span.with_item(Token::Eof),
            },
            Some(tok) => self
                .span()
                .with_item(tok.expect("TODO: handle lexer failure")),
        }
    }

    fn current_lexer_mut(&mut self) -> &mut logos::Lexer<'a, Token> {
        &mut self.sources.get_mut(self.source).1
    }

    fn current_lexer(&self) -> &logos::Lexer<'a, Token> {
        &self.sources.get(self.source).1
    }

    /// advances to the next lexer, returning a reference to it if there is one
    fn advance_lexer(&mut self) -> Option<&mut logos::Lexer<'a, Token>> {
        let mut current_lexer = self.current_lexer();
        if Into::<usize>::into(self.source) == self.sources.len() - 1 {
            return None;
        }
        self.source = (Into::<usize>::into(self.source) + 1usize).into();
        Some(self.current_lexer_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;
    fn check<T: AsRef<str>>(sources: Vec<T>, expected: expect_test::Expect) {
        let mut lexer = Lexer::new(sources.iter().map(|s| s.as_ref()));
        let mut toks = vec![];
        loop {
            let next_tok = lexer.advance();
            if next_tok.item() == &Token::Eof {
                break;
            }

            toks.push(next_tok);
        }

        expected.assert_eq(&format!("{toks:#?}"));
    }
    #[test]
    fn test_lexer_advance() {
        check(vec!["I am some source code"], expect![[r#"
            [
                SpannedItem Identifier [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(0), length: 1 } }],
                SpannedItem Identifier [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(2), length: 2 } }],
                SpannedItem Identifier [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(5), length: 4 } }],
                SpannedItem Identifier [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(10), length: 6 } }],
                SpannedItem Identifier [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(17), length: 4 } }],
            ]"#]])
    }

    #[test]
    fn test_lexer_advance_multiple_sources() {
        check(vec!["I am some", "Source code"], expect![[r#"
            [
                SpannedItem Identifier [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(0), length: 1 } }],
                SpannedItem Identifier [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(2), length: 2 } }],
                SpannedItem Identifier [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(5), length: 4 } }],
                SpannedItem Identifier [Span { source: SourceId(1), span: SourceSpan { offset: SourceOffset(0), length: 6 } }],
                SpannedItem Identifier [Span { source: SourceId(1), span: SourceSpan { offset: SourceOffset(7), length: 4 } }],
            ]"#]])
    }

    #[test]
    fn test_symbols() {
        check(vec!["((5 +-/* 2)[]"], expect![[r#"
            [
                SpannedItem OpenParen [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(0), length: 1 } }],
                SpannedItem OpenParen [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(1), length: 1 } }],
                SpannedItem Integer [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(2), length: 1 } }],
                SpannedItem Plus [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(4), length: 1 } }],
                SpannedItem Minus [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(5), length: 1 } }],
                SpannedItem Divide [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(6), length: 1 } }],
                SpannedItem Multiply [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(7), length: 1 } }],
                SpannedItem Integer [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(9), length: 1 } }],
                SpannedItem CloseParen [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(10), length: 1 } }],
                SpannedItem OpenBracket [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(11), length: 1 } }],
                SpannedItem CloseBracket [Span { source: SourceId(0), span: SourceSpan { offset: SourceOffset(12), length: 1 } }],
            ]"#]])
    }
}
