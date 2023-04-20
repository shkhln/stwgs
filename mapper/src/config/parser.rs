use std::collections::HashMap;

use lazy_static::lazy_static;
use pest::iterators::{Pair, Pairs};
use pest::pratt_parser::PrattParser;
use pest::Parser;
use pest_derive::Parser;

use super::ast;

#[derive(Parser)]
#[grammar = "config/grammar.pest"]
struct ConfigParser;

lazy_static! {

  static ref PRATT_PARSER: PrattParser<Rule> = {

    use pest::pratt_parser::{Assoc::*, Op};

    PrattParser::new()
      .op(Op::infix(Rule::equal,    Left))
      .op(Op::infix(Rule::bit_or,   Left))
      .op(Op::infix(Rule::add,      Left) | Op::infix(Rule::subtract, Left))
      .op(Op::infix(Rule::multiply, Left) | Op::infix(Rule::divide,   Left))
      //.op(Op::infix(power, Right))
      .op(Op::infix(Rule::dot,      Left))
  };
}

fn into_typed_ast(pairs: &Pairs<Rule>) -> ast::Expression {
  PRATT_PARSER
    .map_primary(|pair: Pair<Rule>| match pair.as_rule() {
      Rule::number => {
        let s = pair.as_str();
        let n = s.parse::<f32>().unwrap();
        if n == f32::INFINITY {
          let (line, column) = pair.line_col();
          panic!("{}", format!("Number is too large at {}:{}", line, column));
        } else {
          ast::Expression::Number(n, pair.as_span().into())
        }
      },

      Rule::boolean => ast::Expression::Boolean(pair.as_str().parse::<bool>().unwrap(), pair.as_span().into()),

      Rule::string => {
        let s = pair.as_str();
        ast::Expression::String(s[1..(s.len() - 1)].to_string().replace("\\n", "\n"), pair.as_span().into())
      },

      Rule::ident => ast::Expression::Identifier(pair.as_str().to_string(), pair.as_span().into()),

      Rule::function => {
        let mut function = pair.clone().into_inner();

        let ident = function.next().unwrap();

        let mut args = vec![];
        for arg in function {

          let mut name = None;
          let mut expr = None;

          for pair in arg.into_inner() {
            match pair.as_rule() {
              Rule::ident      => name = Some(pair.as_str().to_string()),
              Rule::expression => expr = Some(pair),
              _ => unreachable!()
            };
          }

          args.push((name, into_typed_ast(&expr.unwrap().into_inner())));
        }

        ast::Expression::Apply(ident.as_str().to_string(), args, pair.as_span().into())
      },

      Rule::scope => {
        let mut statements  = vec![];
        let mut expressions = vec![];

        for pair in pair.clone().into_inner().filter(|p| p.as_rule() != Rule::EOI) {
          match pair.as_rule() {
            Rule::let_st => {
              let mut let_st = pair.clone().into_inner();

              let ident = let_st.next().unwrap();
              assert_eq!(ident.as_rule(), Rule::ident);

              let mut identifiers = vec![ident.as_str().to_string()];

              while let_st.peek().filter(|i| i.as_rule() == Rule::ident).is_some() {
                identifiers.push(let_st.next().unwrap().as_str().to_string());
              }

              let inner_expr = let_st.next().unwrap();
              assert_eq!(inner_expr.as_rule(), Rule::expression);

              statements.push(
                ast::Statement::Let(identifiers, Box::new(into_typed_ast(&inner_expr.into_inner())), pair.as_span().into()));
            },

            Rule::def_st => {
              let mut def_st = pair.clone().into_inner();

              let ident = def_st.next().unwrap();
              assert_eq!(ident.as_rule(), Rule::ident);

              let mut arguments = vec![];
              if def_st.peek().unwrap().as_rule() == Rule::def_args {
                for arg in def_st.next().unwrap().into_inner() {
                  assert_eq!(arg.as_rule(), Rule::ident);
                  arguments.push(arg.as_str().to_string());
                }
              }

              let inner_expr = def_st.next().unwrap();
              assert_eq!(inner_expr.as_rule(), Rule::expression);

              statements.push(
                ast::Statement::Def(ident.as_str().to_string(), arguments, Box::new(into_typed_ast(&inner_expr.into_inner())), pair.as_span().into()));
            },
            Rule::expr_list => {
              for expr in pair.into_inner() {
                assert_eq!(expr.as_rule(), Rule::expression);
                expressions.push(into_typed_ast(&expr.into_inner()));
              }
            },
            _ => unreachable!("{:?}", pair)
          }
        }

        ast::Expression::Scope(statements, expressions, pair.as_span().into())
      },

      Rule::expression => into_typed_ast(&pair.into_inner()),

      Rule::layer_expr => {
        let mut layer = pair.clone().into_inner();

        let ident = layer.next().unwrap();
        assert_eq!(ident.as_rule(), Rule::ident);

        let mut identifiers = vec![ident.as_str().to_string()];

        while layer.peek().filter(|i| i.as_rule() == Rule::ident).is_some() {
          identifiers.push(layer.next().unwrap().as_str().to_string());
        }

        let inner_expr = layer.next().unwrap();
        assert_eq!(inner_expr.as_rule(), Rule::expression);

        ast::Expression::Layer(identifiers, Box::new(into_typed_ast(&inner_expr.into_inner())), pair.as_span().into())
      },

      Rule::if_else => {
        let mut if_else = pair.clone().into_inner();

        let condition = if_else.next().unwrap();
        assert_eq!(condition.as_rule(), Rule::expression);

        let branch1 = if_else.next().unwrap();
        assert_eq!(branch1.as_rule(), Rule::expression);

        let branch2 = if_else.next().unwrap();
        assert_eq!(branch1.as_rule(), Rule::expression);

        ast::Expression::IfElse(
          Box::new(into_typed_ast(&condition.into_inner())),
          Box::new(into_typed_ast(&branch1.into_inner())),
          Box::new(into_typed_ast(&branch2.into_inner())),
          pair.as_span().into()
        )
      },

      whatever => unreachable!("What do we have there?\n\t{:?}", whatever)
    })
    .map_infix(|lhs: ast::Expression, op: Pair<Rule>, rhs: ast::Expression| {
      let span = lhs.span().merge(rhs.span());
      match op.as_rule() {
        Rule::add      => ast::Expression::OpExpr(ast::Operation::Add,    Box::new(lhs), Box::new(rhs), span),
        Rule::subtract => ast::Expression::OpExpr(ast::Operation::Sub,    Box::new(lhs), Box::new(rhs), span),
        Rule::multiply => ast::Expression::OpExpr(ast::Operation::Mul,    Box::new(lhs), Box::new(rhs), span),
        Rule::divide   => ast::Expression::OpExpr(ast::Operation::Div,    Box::new(lhs), Box::new(rhs), span),
        Rule::dot      => ast::Expression::OpExpr(ast::Operation::Access, Box::new(lhs), Box::new(rhs), span),
        Rule::bit_or   => ast::Expression::OpExpr(ast::Operation::BitOr,  Box::new(lhs), Box::new(rhs), span),
        Rule::equal    => ast::Expression::OpExpr(ast::Operation::Eq,     Box::new(lhs), Box::new(rhs), span),
        _ => unreachable!(),
      }
    })
    .parse(pairs.clone()) // clone?
}

fn tag_variables(node: &mut ast::Expression, scopes: &mut Vec<HashMap<String, usize>>, counter: &mut usize) {

  use ast::Expression::*;
  use ast::Statement::*;

  match node {
    Identifier(ref mut name, _) => {
      for scope in scopes.iter().rev() {
        if let Some(index) = scope.get(name) {
          *name = format!("{}${}", name, index);
          break;
        }
      }
    },

    Number(_, _) | Boolean(_, _) | String(_, _) => (),

    OpExpr(_, lhs, rhs, _) => {
      tag_variables(lhs, scopes, counter);
      tag_variables(rhs, scopes, counter);
    },

    Apply(name, args, _) => {
      for scope in scopes.iter().rev() {
        if let Some(index) = scope.get(name) {
          *name = format!("{}${}", name, index);
          break;
        }
      }

      for (_, arg) in args {
        tag_variables(arg, scopes, counter);
      }
    },

    Scope(statements, expressions, _) => {
      scopes.push(HashMap::new());
      for st in statements {
        match st {
          Let(ref mut names, body, _) => {
            tag_variables(body, scopes, counter);
            for name in names {
              scopes.last_mut().unwrap().insert(name.clone(), *counter);
              *name = format!("{}${}", name, *counter);
              *counter += 1;
            }
          },

          Def(ref mut name, args, body, _) => {
            scopes.push(HashMap::new());
            for arg in args {
              scopes.last_mut().unwrap().insert(arg.clone(), *counter);
              *arg = format!("{}${}", arg, *counter);
              *counter += 1;
            }

            tag_variables(body, scopes, counter);

            scopes.remove(scopes.len() - 1);

            scopes.last_mut().unwrap().insert(name.clone(), *counter);
            *name = format!("{}${}", name, *counter);
            *counter += 1;
          }
        };
      }
      for expr in expressions {
        tag_variables(expr, scopes, counter);
      }
      scopes.remove(scopes.len() - 1);
    },

    Layer(_, expr, _) => {
      tag_variables(expr, scopes, counter);
    },

    IfElse(condition, branch1, branch2, _) => {
      tag_variables(condition, scopes, counter);
      tag_variables(branch1, scopes, counter);
      tag_variables(branch2, scopes, counter);
    }
  };
}

pub fn parse_config(config: &str) -> Result<ast::Expression, String> {
  match ConfigParser::parse(Rule::scope, config) {
    Ok(pairs) => {
      let mut ast = into_typed_ast(&pairs);
      tag_variables(&mut ast, &mut vec![HashMap::new()], &mut 1);
      Ok(ast)
    },
    Err(err) => Err(format!("{}", err))
  }
}
