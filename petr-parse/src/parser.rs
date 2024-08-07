#[cfg(test)]
mod tests;

mod lexer;
use std::rc::Rc;

use lexer::Lexer;
pub use lexer::Token;
use miette::{Diagnostic, SourceSpan};
use petr_ast::{Ast, Comment, ExprId, List, Module};
use petr_utils::{IndexMap, SourceId, Span, SpannedItem, SymbolId, SymbolInterner};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub struct ParseError {
    kind: ParseErrorKind,
    help: Option<String>,
}

impl From<ParseErrorKind> for ParseError {
    fn from(kind: ParseErrorKind) -> Self {
        Self { kind, help: None }
    }
}

impl ParseError {
    pub fn with_help(
        mut self,
        help: Option<impl Into<String>>,
    ) -> Self {
        self.help = help.map(Into::into);
        self
    }
}

impl Diagnostic for ParseError {
    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.help.as_ref().map(|x| -> Box<dyn std::fmt::Display> { Box::new(x) })
    }

    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.kind.code()
    }

    fn severity(&self) -> Option<miette::Severity> {
        self.kind.severity()
    }

    fn url<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.kind.url()
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        self.kind.source_code()
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        self.kind.labels()
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        self.kind.related()
    }

    fn diagnostic_source(&self) -> Option<&dyn Diagnostic> {
        self.kind.diagnostic_source()
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

#[derive(Error, Debug, Diagnostic, PartialEq)]
pub enum ParseErrorKind {
    #[error("Unmatched parenthesis")]
    UnmatchedParenthesis,
    #[error("Expected identifier, found {0}")]
    ExpectedIdentifier(String),
    #[error("Expected token {0}, found {1}")]
    ExpectedToken(Token, Token),
    #[error("Expected one of tokens {}; found {1}", format_toks(.0))]
    ExpectedOneOf(Vec<Token>, Token),
    #[error("Internal error in parser. Please file an issue on GitHub. {0}")]
    InternalError(String),
    #[error("File name could not be converted to module name. petr source names should be a valid identifier.")]
    #[help(
        "Identifiers cannot begin with numbers, contain spaces, or contain symbols other than an underscore or hyphen.\
     Hyphens in file names are transformed into underscores."
    )]
    InvalidIdentifier(String),
    #[error("Internal error in parser. Please file an issue on GitHub. A span was joined with a span from another file.")]
    InternalSpanError(#[label] SourceSpan, #[label] SourceSpan),
    #[error("Invalid token encountered")]
    LexerError,
}

impl ParseErrorKind {
    pub fn into_err(self) -> ParseError {
        self.into()
    }
}

fn format_toks(toks: &[Token]) -> String {
    let mut buf = toks.iter().take(toks.len() - 1).map(|t| format!("{}", t)).collect::<Vec<_>>().join(", ");
    match toks.len() {
        2 => {
            buf.push_str(&format!(" or {}", toks.last().unwrap()));
        },
        x if x > 2 => {
            buf.push_str(&format!(", or {}", toks.last().unwrap()));
        },
        _ => (),
    }
    buf
}

type Result<T> = std::result::Result<T, ParseError>;

pub struct Parser {
    interner: SymbolInterner,
    /// some exprs need to be assigned an ID, because they generate a scope
    /// which is stored in the binder and needs to be retrieved later
    expr_id_assigner: usize,
    lexer: Lexer,
    errors: Vec<SpannedItem<ParseError>>,
    comments: Vec<SpannedItem<Comment>>,
    peek: Option<SpannedItem<Token>>,
    // the tuple is the file name and content
    source_map: IndexMap<SourceId, (&'static str, &'static str)>,
    help: Vec<String>,
}

impl Parser {
    pub fn push_error(
        &mut self,
        err: SpannedItem<ParseErrorKind>,
    ) {
        if self.help.is_empty() {
            return self.errors.push(err.map(|err| err.into_err()));
        }
        let mut help_text = Vec::with_capacity(self.help.len());
        for (indentation, help) in self.help.iter().enumerate() {
            let is_last = indentation == self.help.len() - 1;
            let text = format!(
                "{}{}{}{help}",
                "  ".repeat(indentation),
                if indentation == 0 { "" } else { "↪ " },
                if is_last { "expected " } else { "while parsing " }
            );
            help_text.push(text);
        }
        let err = err.map(|err| err.into_err().with_help(Some(help_text.join("\n"))));
        self.errors.push(err);
    }

    pub fn slice(&self) -> &str {
        self.lexer.slice()
    }

    pub fn intern(
        &mut self,
        internee: Rc<str>,
    ) -> SymbolId {
        self.interner.insert(internee)
    }

    pub fn span(&self) -> Span {
        self.lexer.span()
    }

    pub fn peek(&mut self) -> SpannedItem<Token> {
        if let Some(ref peek) = self.peek {
            *peek
        } else {
            let item = self.advance();
            self.peek = Some(item);
            item
        }
    }

    pub fn new_with_existing_interner_and_source_map(
        sources: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        interner: SymbolInterner,
        mut source_map: IndexMap<SourceId, (&'static str, &'static str)>,
    ) -> Self {
        // TODO we hold two copies of the source for now: one in source_maps, and one outside the parser
        // for the lexer to hold on to and not have to do self-referential pointers.
        let sources = sources
            .into_iter()
            .map(|(name, source)| -> (&'static str, &'static str) {
                let name = name.into();
                let source = source.into();
                (Box::leak(name.into_boxed_str()), Box::leak(source.into_boxed_str()))
            })
            .collect::<Vec<_>>();
        let sources_for_lexer = sources.iter().map(|(_, source)| *source);

        let lexer = Lexer::new_with_offset_into_sources(sources_for_lexer, source_map.len());

        for (name, source) in sources.into_iter() {
            source_map.insert((name, source));
        }

        Self {
            // reuse the interner if provided, otherwise create a new one
            interner,
            lexer,
            errors: Default::default(),
            comments: Default::default(),
            peek: None,
            source_map,
            help: Default::default(),
            expr_id_assigner: 0,
        }
    }

    // TODO: document [ExprId] system
    pub fn new_expr_id(&mut self) -> ExprId {
        let id = self.expr_id_assigner;
        self.expr_id_assigner += 1;
        ExprId(id)
    }

    pub fn new(sources: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        Self::new_with_existing_interner_and_source_map(sources, Default::default(), Default::default())
    }

    pub fn drain_comments(&mut self) -> Vec<Comment> {
        self.comments.drain(..).map(|spanned_item| spanned_item.into_item()).collect()
    }

    /// consume tokens until a node is produced
    #[allow(clippy::type_complexity)]
    pub fn into_result(
        mut self
    ) -> (
        Ast,
        Vec<SpannedItem<ParseError>>,
        SymbolInterner,
        IndexMap<SourceId, (&'static str, &'static str)>,
    ) {
        let nodes: Vec<Module> = self.many::<Module>();
        // drop the lexers from the source map
        (Ast::new(nodes), self.errors, self.interner, self.source_map)
    }

    pub fn interner(&self) -> &SymbolInterner {
        &self.interner
    }

    pub fn many<P: Parse>(&mut self) -> Vec<P> {
        let mut buf = Vec::new();
        loop {
            if *self.peek().item() == Token::Eof {
                break;
            }
            if let Some(parsed_item) = P::parse(self) {
                buf.push(parsed_item);
            } else {
                break;
            }
        }
        buf
    }

    /// parses a sequence separated by `separator`
    /// e.g. if separator is `Token::Comma`, can parse `a, b, c, d`
    /// NOTE: this parses zero or more items. Will not reject zero items.
    pub fn sequence_zero_or_more<P: Parse>(
        &mut self,
        separator: Token,
    ) -> Option<Vec<P>> {
        self.with_help(format!("while parsing {separator} separated sequence"), |p| {
            let mut buf = vec![];
            loop {
                // if there are no items in the buf, we try a backtracking parse in case this is
                // a zero sequence
                let item = if buf.is_empty() {
                    let res = p.with_backtrack(|p| P::parse(p));
                    match res {
                        Ok(item) => Some(item),
                        Err(_) => {
                            // return the zero sequence
                            break;
                        },
                    }
                } else {
                    // there is at least one item in the buf, so we know all parse errors arising
                    // from `P` are valid
                    P::parse(p)
                };
                match item {
                    Some(item) => buf.push(item),
                    None => {
                        break;
                    },
                }
                if *p.peek().item() == separator {
                    p.advance();
                } else {
                    break;
                }
            }

            Some(buf)
        })
    }

    /// parses a sequence separated by `separator`
    /// e.g. if separator is `Token::Comma`, can parse `a, b, c, d`
    /// NOTE: this parses one or more items. Will reject zero items.
    /// alias for `sequence`
    pub fn sequence_one_or_more<P: Parse>(
        &mut self,
        separator: Token,
    ) -> Option<Vec<P>> {
        self.sequence(separator)
    }

    /// parses a sequence separated by `separator`
    /// e.g. if separator is `Token::Comma`, can parse `a, b, c, d`
    /// NOTE: this parses one or more items. Will reject zero items.
    pub fn sequence<P: Parse>(
        &mut self,
        separator: Token,
    ) -> Option<Vec<P>> {
        let mut buf = vec![];
        let errs = loop {
            let item = self.with_backtrack(|p| P::parse(p));
            match item {
                Ok(item) => buf.push(item),
                Err(errs) => {
                    break errs;
                },
            }
            if *self.peek().item() == separator {
                self.advance();
            } else {
                break vec![];
            }
        };
        if buf.is_empty() {
            for err in errs {
                self.errors.push(err)
            }
            None
        } else {
            Some(buf)
        }
    }

    pub fn advance(&mut self) -> SpannedItem<Token> {
        if let Some(tok) = self.peek.take() {
            return tok;
        }
        let next_tok = match self.lexer.advance() {
            Ok(o) => o,
            Err(span) => {
                let span = span.span();
                self.push_error(span.with_item(ParseErrorKind::LexerError));
                return span.with_item(Token::Eof);
            },
        };
        match *next_tok.item() {
            Token::Newline => self.advance(),
            Token::Comment => {
                if let Some(comment) = self.parse::<SpannedItem<Comment>>() {
                    self.comments.push(comment);
                }
                self.advance()
            },
            _ => next_tok,
        }
    }

    /// doesn't push the error to the error list and doesn't advance if the token is not found
    pub fn try_token(
        &mut self,
        tok: Token,
    ) -> Option<SpannedItem<Token>> {
        let peeked_token = self.peek();
        if *peeked_token.item() == tok {
            Some(self.advance())
        } else {
            None
        }
    }

    /// doesn't push the error to the error list and doesn't advance if none of the tokens are
    /// found
    pub(crate) fn try_tokens(
        &mut self,
        toks: &[Token],
    ) -> Option<SpannedItem<Token>> {
        let peeked_token = self.peek();
        if toks.contains(peeked_token.item()) {
            Some(self.advance())
        } else {
            None
        }
    }

    pub fn token(
        &mut self,
        tok: Token,
    ) -> Option<SpannedItem<Token>> {
        self.with_help(format!("while parsing token {tok}"), |p| {
            let peeked_token = p.peek();
            if *peeked_token.item() == tok {
                Some(p.advance())
            } else {
                let span = p.lexer.span();
                p.push_error(span.with_item(ParseErrorKind::ExpectedToken(tok, *peeked_token.item())));
                None
            }
        })
    }

    pub fn parse<P: Parse>(&mut self) -> Option<P> {
        P::parse(self)
    }

    pub fn one_of<const N: usize>(
        &mut self,
        toks: [Token; N],
    ) -> Option<SpannedItem<Token>> {
        match self.peek().item() {
            tok if toks.contains(tok) => self.token(*tok),
            tok => {
                let span = self.lexer.span();
                if N == 1 {
                    self.push_error(span.with_item(ParseErrorKind::ExpectedToken(toks[0], *tok)));
                } else {
                    self.push_error(span.with_item(ParseErrorKind::ExpectedOneOf(toks.to_vec(), *tok)));
                }
                None
            },
        }
    }

    pub fn errors(&self) -> &[SpannedItem<ParseError>] {
        &self.errors
    }

    pub fn with_help<F, T>(
        &mut self,
        help_text: impl Into<String>,
        f: F,
    ) -> T
    where
        F: Fn(&mut Parser) -> T,
    {
        self.push_help(help_text);
        let res = f(self);
        self.pop_help();
        res
    }

    fn push_help(
        &mut self,
        arg: impl Into<String>,
    ) {
        self.help.push(arg.into())
    }

    fn pop_help(&mut self) {
        let _ = self.help.pop();
    }

    /// Performs a backtracking parse, which means that if the inner function returns `None`,
    /// the parser will backtrack to the state before the function was called and revert any
    /// errors that were encountered, returning them as `Err` but crucially not appending them to
    /// self.errors`.
    /// Note that this is NOT a performant method, and it should be used sparingly.
    pub fn with_backtrack<F, T>(
        &mut self,
        f: F,
    ) -> std::result::Result<T, Vec<SpannedItem<ParseError>>>
    where
        F: Fn(&mut Parser) -> Option<T>,
    {
        let checkpoint = self.checkpoint();
        let res = f(self);
        match res {
            Some(res) => Ok(res),
            None => Err(self.restore_checkpoint(checkpoint)),
        }
    }

    fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            errors: self.errors.len(),
            lexer:  self.lexer.clone(),
            peek:   self.peek,
        }
    }

    fn restore_checkpoint(
        &mut self,
        checkpoint: Checkpoint,
    ) -> Vec<SpannedItem<ParseError>> {
        self.lexer = checkpoint.lexer;
        self.peek = checkpoint.peek;
        self.errors.split_off(checkpoint.errors)
    }

    pub fn source_map(&self) -> &IndexMap<SourceId, (&'static str, &'static str)> {
        &self.source_map
    }
}

struct Checkpoint {
    errors: usize,
    lexer:  Lexer,
    peek:   Option<SpannedItem<Token>>,
}

pub trait Parse: Sized {
    fn parse(p: &mut Parser) -> Option<Self>;
}

impl<T> Parse for SpannedItem<T>
where
    T: Parse,
{
    fn parse(p: &mut Parser) -> Option<Self> {
        let before_span = p.lexer.span();
        let result = T::parse(p)?;

        let mut after_span = p.lexer.span();
        if after_span.source() != before_span.source() && after_span.span().offset() == 0 {
            // this is acceptable only if after_span.pos == 0, 0 OR if after_span.pos <= the first
            // whitespace of the next file, which is less likely to be hit and this function
            // doesn't account for (yet).
            //
            // Sometimes, the span actually goes to the last character of the previous file
            // this solves the case where the parse consumes all white space until the next token,
            // which will be in the next file
            //
            // TODO: it would be better if we were smarter about spans, and ideally
            // a span going into the next file wouldn't be generated in this scenario.
            // assignee: sezna
            after_span = before_span.extend(p.source_map.get(before_span.source()).1.len());
        } else if after_span.source() != before_span.source() {
            let span = before_span.with_item(ParseErrorKind::InternalSpanError(after_span.span(), before_span.span()));
            p.push_error(span);
            return None;
        }

        // i think this should be `hi` to `hi`, not 100% though
        Some(before_span.hi_to_hi(after_span).with_item(result))
    }
}

impl Parse for List {
    fn parse(p: &mut Parser) -> Option<Self> {
        p.try_token(Token::OpenBracket)?;
        let elements = p.sequence(Token::Comma)?;
        p.token(Token::CloseBracket)?;
        Some(List {
            elements: elements.into_boxed_slice(),
        })
    }
}
