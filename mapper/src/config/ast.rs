#[derive(Clone, Copy, Debug)]
pub struct Span(pub (usize, usize), pub (usize, usize));

impl<'a> From<pest::Span<'a>> for Span {
  fn from(span: pest::Span) -> Self {
    Self(span.start_pos().line_col(), span.end_pos().line_col())
  }
}

impl Span {

  pub fn show_in_source(self, source: &str) -> String {
    let s = self.0;
    let e = self.1;

    if s.0 == e.0 {
      let line = source.lines().nth(s.0 - 1).unwrap();
      format!("     |\n{:4} | {}\n     | {}{}", s.0, line, " ".repeat(s.1 - 1), "^".repeat(e.1 - s.1))
    } else {
      let line_s = source.lines().nth(s.0 - 1).unwrap();
      let line_e = source.lines().nth(e.0 - 1).unwrap();
      format!("     |\n{:4} | {}\n     | {}{}\n ... \n{:4} | {}\n     | {}",
        s.0, line_s, " ".repeat(s.1 - 1), "^".repeat(line_s.len() - s.1 + 1),
        e.0, line_e, "^".repeat(e.1 - 1))
    }
  }

  pub fn merge(self, other: Span) -> Self {
    Self(self.0, other.1)
  }
}

#[derive(Clone, Debug)]
pub enum Expression {
  Apply(String, Vec<(Option<String>, Expression)>, Span),
  Identifier(String, Span),
  Layer(Vec<String>, Box<Expression>, Span),
  Number(f32, Span),
  Boolean(bool, Span),
  String(String, Span),
  OpExpr(Operation, Box<Expression>, Box<Expression>, Span),
  Scope(Vec<Statement>, Vec<Expression>, Span),
  IfElse(Box<Expression>, Box<Expression>, Box<Expression>, Span)
}

impl Expression {

  pub fn span(&self) -> Span {
    use self::Expression::*;
    match self {
      Apply(_, _, span)     => *span,
      Identifier(_, span)   => *span,
      Layer(_, _, span)     => *span,
      Number(_, span)       => *span,
      Boolean(_, span)      => *span,
      String(_, span)       => *span,
      OpExpr(_, _, _, span) => *span,
      Scope(_, _, span)     => *span,
      IfElse(_, _, _, span) => *span
    }
  }
}

#[derive(Clone, Debug)]
pub enum Statement {
  Def(String, Vec<String>, Box<Expression>, Span),
  Let(Vec<String>, Box<Expression>, Span)
}

#[derive(Clone, Debug)]
pub enum Operation {
  Add, Sub, Mul, Div, Access /* ? */, BitOr, Eq
}
