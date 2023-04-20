use std::collections::HashMap;

use overlay_ipc::Knob;

use super::ast::*;
use super::util;
use crate::config::{Axis, Button};
use crate::mapper::LayerMask;
use crate::output::{KeyboardKey, MouseAxis, MouseButton};
use crate::stages::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Constant {
  InputAxis(Axis),
  InputButton(Button),
  MouseAxis(MouseAxis),
  MouseButton(MouseButton),
  KeyboardKey(KeyboardKey)
}

#[derive(Clone)]
pub enum Value {
  List(Vec<Value>),
  Struct(HashMap<String, Value>),
  Constant(Constant),
  Pipeline1D(PipelineRef<f32>),
  Pipeline2D(PipelineRef<(f32, f32)>),
  PipelineB(PipelineRef<bool>),
  CompletePipeline(LayerMask, std::rc::Rc<Box<dyn Pipeline<()>>>),
  LayerMask(LayerMask),
  Number(f32),
  Boolean(bool),
  String(String),
  Nothing
}

#[derive(Debug)]
pub struct EvalError(pub String, pub Option<Span>);

impl PartialEq for Value {

  fn eq(&self, other: &Self) -> bool {
    match self {
      Value::List(vec1) => {
        match other {
          Value::List(vec2) => vec1 == vec2,
          _ => false
        }
      },
      Value::Struct(map1) => {
        match other {
          Value::Struct(map2) => map1 == map2,
          _ => false
        }
      },
      Value::Constant(const1) => {
        match other {
          Value::Constant(const2) => const1 == const2,
          _ => false
        }
      },
      Value::Pipeline1D(_) => false, //TODO: compare pipelines by identity?
      Value::Pipeline2D(_) => false,
      Value::PipelineB(_)  => false,
      Value::CompletePipeline(_, _) => false,
      Value::LayerMask(x) => {
        match other {
          Value::LayerMask(y) => x == y,
          _ => false
        }
      },
      Value::Number(x) => {
        match other {
          Value::Number(y) => x == y,
          _ => false
        }
      },
      Value::Boolean(x) => {
        match other {
          Value::Boolean(y) => x == y,
          _ => false
        }
      },
      Value::String(x) => {
        match other {
          Value::String(y) => x == y,
          _ => false
        }
      },
      Value::Nothing => {
        match other {
          Value::Nothing => true, // ?
          _ => false
        }
      }
    }
  }
}

impl std::fmt::Debug for Value {

  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Value::List(v)                 => f.write_str(&format!("{:?}", v)),
      Value::Struct(m)               => f.write_str(&format!("{:?}", m)),
      Value::Constant(c)             => f.write_str(&format!("{:?}", c)),
      Value::Pipeline1D(p)           => f.write_str(&p.borrow().desc()),
      Value::Pipeline2D(p)           => f.write_str(&p.borrow().desc()),
      Value::PipelineB(p)            => f.write_str(&p.borrow().desc()),
      Value::CompletePipeline(ls, p) => f.write_str(&format!("{:?} -> {}", ls, p.desc())),
      Value::LayerMask(mask)         => f.write_str(&format!("{:?}", mask)),
      Value::Number(n)               => f.write_str(&format!("{:?}", n)),
      Value::Boolean(b)              => f.write_str(&format!("{:?}", b)),
      Value::String(s)               => f.write_str(s),
      Value::Nothing                 => f.write_str("()")
    }
  }
}

fn name_of_type(value: &Value) -> &'static str {
  match value {
    Value::List(_)                => "List",
    Value::Struct(_)              => "Struct",
    Value::Constant(_)            => "Constant",
    Value::Pipeline1D(_)          => "Pipeline1D",
    Value::Pipeline2D(_)          => "Pipeline2D",
    Value::PipelineB(_)           => "PipelineB",
    Value::CompletePipeline(_, _) => "CompletePipeline",
    Value::LayerMask(_)           => "LayerMask",
    Value::Number(_)              => "Number",
    Value::Boolean(_)             => "Boolean",
    Value::String(_)              => "String",
    Value::Nothing                => "Nothing"
  }
}

#[derive(Clone)]
pub enum Variable<'a> {
  Value(Value),
  NativeFun(std::rc::Rc<dyn 'a + Fn(&[Value], HashMap<String, Value>) -> Result<Value, Option<String>>>),
  ScriptFun(Vec<String>, Expression),
  KnobFun
}

impl<'a> std::fmt::Debug for Variable<'a> {

  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Variable::Value(v)        => f.write_str(&format!("{:?}", v)),
      Variable::ScriptFun(_, _) => f.write_str("<script fun>"),
      Variable::NativeFun(_)    => f.write_str("<native fun>"),
      Variable::KnobFun         => f.write_str("<knob fun>"),
    }
  }
}

#[derive(Debug)]
pub struct Context<'a> {
  pub layers:      Vec<String>,
  pub vars:        Vec<HashMap<String, Variable<'a>>>,
  pub knobs:       Vec<Knob>,
  pub knob_values: HashMap<String, Value>
}

impl<'a> Context<'a> {
  pub fn new(knob_values: Option<HashMap<String, Value>>) -> Self {
    Self {
      layers:      vec![],
      vars:        vec![{
        let mut map = HashMap::new();
        map.insert("knob".to_string(), Variable::KnobFun);
        map
      }],
      knobs:       vec![],
      knob_values: knob_values.unwrap_or_default()
    }
  }

  fn exists(&self, name: &str) -> bool {
    self.vars.last().unwrap().contains_key(name)
  }

  pub fn insert_var(&mut self, name: &str, value: Value) {
    if !name.starts_with('_') {
      assert!(!self.exists(name), "Variable {} already exists in scope", name);
      self.vars.last_mut().unwrap().insert(name.to_string(), Variable::Value(value));
    }
  }

  //TODO: check that argument names are unique
  fn insert_fun(&mut self, name: &str, args: Vec<String>, body: Expression) {
    if !name.starts_with('_') {
      assert!(!self.exists(name), "Variable {} already exists in scope", name);
      self.vars.last_mut().unwrap().insert(name.to_string(), Variable::ScriptFun(args, body));
    }
  }

  pub fn register_fun<F: 'a + Fn(&[Value], HashMap<String, Value>) -> Result<Value, Option<String>>>(&mut self, name: &str, fun: F) {
    let scope = self.vars.first_mut().unwrap();
    assert!(!scope.contains_key(name), "Variable {} already exists in root scope", name);
    scope.insert(name.to_string(), Variable::NativeFun(std::rc::Rc::new(fun)));
  }

  fn lookup(&self, name: &str) -> Option<Variable<'a>> {
    for scope in (self.vars).iter().rev() {
      if scope.contains_key(name) {
        return Some(scope[name].clone());
      }
    }
    None
  }

  fn get_value(&mut self, name: &str, location: Option<Span>) -> Result<Value, EvalError> {
    if let Some(var) = self.lookup(name) {
      match var {
        Variable::Value(v) => Ok(v),
        Variable::NativeFun(_) | Variable::KnobFun => {
          Err(EvalError(format!("Can't invoke function {} without arguments", name.split('$').next().unwrap()), location))
        },
        Variable::ScriptFun(_arg_names, _body) => {
          /*if arg_names.len() == 0 {
            eval(body, self, false)
          } else {
            let message = format!("Expected {} args in function {}, got 0", arg_names.len(), name.split('$').next().unwrap());
            return Err(EvalError(message, location));
          }*/
          Err(EvalError(format!("Can't invoke function {} without arguments", name.split('$').next().unwrap()), location))
        }
      }
    } else {
      Err(EvalError(format!("Variable {} doesn't exist", name.split('$').next().unwrap()), location))
    }
  }

  fn apply_fun(&mut self, name: &str, posit_args: Vec<Value>, named_args: HashMap<String, Value>, location: Option<Span>) -> Result<Value, EvalError> {
    if let Some(var) = self.lookup(name) {
      match var {
        Variable::Value(v) => Err(EvalError(format!("Can't invoke value {:?} as function", v), location)),
        Variable::NativeFun(fun) => {
          fun(posit_args.as_slice(), named_args).map_err(|str| {
            if let Some(str) = str {
              EvalError(format!("{}: {}", name, str), location)
            } else {
              //TODO: include all values and types in the error message
              let posit_arg_types = posit_args.iter().map(name_of_type).collect::<Vec<&str>>().join(", ");
              EvalError(format!("{}({})", name, posit_arg_types), location)
            }
          })
        },
        Variable::ScriptFun(ref arg_names, ref body) => {

          self.new_scope();

          if arg_names.len() == posit_args.len() + named_args.len() {

            for i in 0..posit_args.len() {
              self.insert_var(&arg_names[i], posit_args[i].clone()); // clone?
            }

            for (name, value) in named_args {
              let prefix = format!("{}$", name);
              if let Some(name) = arg_names.iter().find(|n| n.starts_with(&prefix)) {
                self.insert_var(name, value);
              } else {
                return Err(EvalError(format!("Unknown argument {}", name), location));
              }
            }

          } else {
            let message = format!("Expected {} args in function {}, got {}",
              arg_names.len(), name.split('$').next().unwrap(), posit_args.len() + named_args.len());
            return Err(EvalError(message, location));
          }

          let value = eval((*body).clone(), self, false); // clone?
          self.drop_scope();
          value
        },
        Variable::KnobFun => {
          let args = posit_args.as_slice();
          let opts = named_args;
          match args {
            [Value::String(name), Value::Boolean(default)] => {
              if self.knobs.iter().any(|k| k.name() == *name) {
                return Err(EvalError(format!("Knob {} is already registered", name), location));
              }
              let value = if let Some(Value::Boolean(value)) = self.knob_values.get(name) { *value } else { *default };
              self.knobs.push(Knob::Flag { name: name.clone(), value });
              Ok(Value::Boolean(*default))
            },
            [Value::String(name), Value::String(default), Value::List(options)] => {
              if self.knobs.iter().any(|k| k.name() == *name) {
                return Err(EvalError(format!("Knob {} is already registered", name), location));
              }
              let value = if let Some(Value::String(value)) = self.knob_values.get(name) {
                if let Some(options) = util::strings(options) {
                  if options.contains(value) {
                    value
                  } else {
                    default
                  }
                } else {
                  return Err(EvalError("Options should only contain string values".to_string(), location));
                }
              } else {
                default
              };
              if let Some(options) = util::strings(options) {
                let index = options.iter().position(|opt| opt == value).unwrap_or(0);
                self.knobs.push(Knob::Enum { name: name.clone(), index, options });
              } else {
                return Err(EvalError("Options should only contain string values".to_string(), location));
              }
              Ok(Value::String(value.clone()))
            },
            [Value::String(name), Value::Number(default)] => {
              if self.knobs.iter().any(|k| k.name() == *name) {
                return Err(EvalError(format!("Knob {} is already registered", name), location));
              }
              if let (Some(Value::Number(ref min_value)), Some(Value::Number(max_value))) =
                (opts.get("min_value"), opts.get("max_value"))
              {
                let value = if let Some(Value::Number(value)) = self.knob_values.get(name) { *value } else { *default };
                self.knobs.push(Knob::Number { name: name.clone(), value, min_value: *min_value, max_value: *max_value });
                Ok(Value::Number(value))
              } else {
                Err(EvalError("min_value/max_value should be specified".to_string(), location))
              }
            },
            _ => Err(EvalError("Unknown knob format".to_string(), location))
          }
        }
      }
    } else {
      Err(EvalError(format!("Unknown function: {}", name), location))
    }
  }

  fn new_scope(&mut self) {
    self.vars.push(HashMap::new());
  }

  fn drop_scope(&mut self) {
    self.vars.remove(self.vars.len() - 1);
  }
}

fn eval(expr: Expression, ctx: &mut Context, allow_layer_exprs: bool) -> Result<Value, EvalError> {

  use Expression::*;
  use Operation::*;
  use Statement::*;

  match expr {
    Identifier(id, span) => ctx.get_value(&id, Some(span)),
    Number(n, _)      => Ok(Value::Number(n)),
    Boolean(b, _)     => Ok(Value::Boolean(b)),
    String(s, _)      => Ok(Value::String(s)),
    OpExpr(Access, lhs, rhs, _) => {
      let rhs_span = rhs.span();
      match (eval(*lhs, ctx, false)?, *rhs) {
        (Value::Struct(map), Identifier(field, _)) => {
          map.get(field.as_str())
            .map(|v| Ok(v.clone()))
            .unwrap_or_else(|| Err(EvalError(format!("No entry found for key {}", field), Some(rhs_span))))
        },
        (ref value, Apply(ref fun, ref args, _)) => {

          let mut named_args = false;
          for (name, _) in args {
            if name.is_none() && named_args {
              return Err(EvalError("Named args should follow positional args".to_string(), Some(rhs_span)));
            }
            if name.is_some() {
              named_args = true
            }
          }

          let mut posit_args = vec![(*value).clone()];
          let mut named_args = HashMap::new();

          for (name, expr) in args {
            if let Some(name) = name {
              named_args.insert(name.to_string(), eval((*expr).clone(), ctx, false)?);
            } else {
              posit_args.push(eval((*expr).clone(), ctx, false)?);
            }
          }

          ctx.apply_fun(fun, posit_args, named_args, Some(rhs_span))
        },
        (value, Identifier(ident, _)) => match ctx.lookup(&ident) {
          Some(Variable::Value(_)) => {
            Err(EvalError(format!("{} is supposed to be a function", ident.split('$').next().unwrap()), Some(rhs_span)))
          },
          Some(Variable::ScriptFun(_, _)) => {
            ctx.apply_fun(&ident, vec![value], HashMap::new(), Some(rhs_span))
          },
          Some(Variable::NativeFun(fun)) => {
            fun(&[value], HashMap::new()).map_err(|str| {
              if let Some(str) = str {
                EvalError(format!("{}: {}", ident, str), Some(rhs_span))
              } else {
                EvalError(ident, Some(rhs_span))
              }
            })
          },
          _ => Err(EvalError("No, thanks".to_string(), Some(rhs_span)))
        },
        _ => Err(EvalError("No, thanks".to_string(), Some(rhs_span)))
      }
    },
    Apply(ref fun, ref args, span) => {
      let mut named_args = false;
      for (name, _) in args {
        if name.is_none() && named_args {
          return Err(EvalError("Named args should follow positional args".to_string(), Some(span)));
        }
        if name.is_some() {
          named_args = true
        }
      }

      let mut posit_args = vec![];
      let mut named_args = HashMap::new();

      for (name, expr) in args {
        if let Some(name) = name {
          named_args.insert(name.to_string(), eval((*expr).clone(), ctx, false)?);
        } else {
          posit_args.push(eval((*expr).clone(), ctx, false)?);
        }
      }

      ctx.apply_fun(fun, posit_args, named_args, Some(span))
    },
    OpExpr(op, lhs, rhs, span) => match (eval(*lhs, ctx, false)?, eval(*rhs, ctx, false)?) {
      (Value::Number(x), Value::Number(y)) => match op {
        Add => Ok(Value::Number(x + y)),
        Sub => Ok(Value::Number(x - y)),
        Mul => Ok(Value::Number(x * y)),
        Div => Ok(Value::Number(x / y)),
        Eq  => Ok(Value::Boolean(x == y)),
        _ => Err(EvalError(format!("Can't apply {:?} to numeric operands", op), Some(span)))
      },
      (Value::LayerMask(x), Value::LayerMask(y)) => match op {
        BitOr => Ok(Value::LayerMask(x | y)),
        _ => Err(EvalError(format!("Can't apply {:?} to layer operands", op), Some(span)))
      },
      (Value::String(lhs), Value::String(rhs)) => match op {
        Eq => Ok(Value::Boolean(lhs == rhs)),
        _  => Err(EvalError(format!("Can't apply {:?} to string operands", op), Some(span)))
      },
      (a, b) => Err(EvalError(format!("Can't apply {:?} to operands {:?} and {:?}", op, a, b), Some(span)))
    },
    Scope(statements, expressions, _) => {
      ctx.new_scope();

      for st in statements {
        match st {
          Let(ids, body, span) => {
            let result = eval(*body, ctx, false)?;

            if ids.len() > 1 {
              if let Value::List(v) = result {
                if ids.len() == v.len() {
                  for i in 0..v.len() {
                    ctx.insert_var(&ids[i], v[i].clone());
                  }
                } else {
                  return Err(EvalError(format!("Expected {} vars", v.len()), Some(span))) //TODO: more specific span
                }
              } else {
                return Err(EvalError(format!("Expected list, got {:?}", result), Some(span))) //TODO: more specific span
              }
            } else {
              ctx.insert_var(ids.first().unwrap(), result);
            }
          },
          Def(name, args, body, _) => {
            ctx.insert_fun(&name, args, *body);
          }
        };
      }

      for expr in &expressions {
        if let Layer(names, _, _) = expr {
          for name in names {
            let index = if !ctx.layers.contains(name) {
              ctx.layers.push(name.clone());
              ctx.layers.len() - 1
            } else {
              ctx.layers.iter().position(|n| n == name).unwrap()
            };

            if !ctx.exists(name) {
              ctx.insert_var(name, Value::LayerMask(LayerMask::user_layer(index).unwrap())); // ?
            }
          }
        }
      }

      let mut result = vec![];
      for expr in expressions {
        result.push(eval(expr, ctx, allow_layer_exprs)?);
      }

      ctx.drop_scope();

      if result.len() == 1 {
        Ok(result.remove(0))
      } else {
        Ok(Value::List(result))
      }
    },
    Layer(names, expr, span) => {

      if !allow_layer_exprs {
        return Err(EvalError("Layers must be declared at the top level of config file".to_string(), Some(span)));
      }

      let mut mask = LayerMask::EMPTY;

      for name in names {
        let index = ctx.layers.iter().position(|n| n == &name).unwrap();
        mask = mask | LayerMask::user_layer(index).unwrap(); // ?
      }

      let mut res = util::flatten(eval((*expr).clone(), ctx, false)?);
      for p in &mut res {
        if let Value::CompletePipeline(layers, _) = p {
          *layers = mask; // ?
        } else {
          return Err(EvalError(format!("Expected closed pipeline, got {:?}", p), None)); //TODO: error location
        }
      }

      Ok(Value::List(res))
    },
    IfElse(condition, branch1, branch2, _) => {
      let span = condition.span();
      if let Value::Boolean(value) = eval(*condition, ctx, false)? {
        if value {
          eval(*branch1, ctx, allow_layer_exprs)
        } else {
          eval(*branch2, ctx, allow_layer_exprs)
        }
      } else {
        Err(EvalError("Expected boolean".to_string(), Some(span)))
      }
    }
  }
}

pub fn eval_config(config: Expression, context: &mut Context) -> Result<Value, EvalError> {
  eval(config, context, true)
}

#[cfg(test)]
mod tests {

  use std::collections::HashMap;

  use super::super::parser::parse_config;
  use super::{Context, *};

  #[test]
  fn functions() {
    let code = r#"
      def foo(x, y) = x + y;
      def bar() = 1;
      def baz   = 1;
      foo(1, 2) + bar() + baz()
    "#;
    if let Ok(p) = parse_config(code) {
      assert_eq!(eval_config(p, &mut Context::new(None)).unwrap(), Value::Number(5.0));
    } else {
      panic!();
    }
  }

  #[test]
  fn math() {
    let code = r#"
      1 + 2 * 2, 5.0
    "#;
    if let Ok(p) = parse_config(code) {
      assert_eq!(eval_config(p, &mut Context::new(None)).unwrap(), Value::List(vec![Value::Number(5.0), Value::Number(5.0)]));
    } else {
      panic!();
    }
  }

  #[test]
  fn named_arguments() {
    let code = r#"
      def foo(bar, baz) = { bar, baz };
      foo(baz = 2, bar = 1)
    "#;
    if let Ok(p) = parse_config(code) {
      assert_eq!(eval_config(p, &mut Context::new(None)).unwrap(), Value::List(vec![Value::Number(1.0), Value::Number(2.0)]));
    } else {
      panic!();
    }
  }

  #[test]
  fn scopes() {
    let code = r#"
      let x = 1;
      let y = {
        let x = 2;
        x
      };
      x + y
    "#;
    if let Ok(p) = parse_config(code) {
      assert_eq!(eval_config(p, &mut Context::new(None)).unwrap(), Value::Number(3.0));
    } else {
      panic!();
    }
  }

  #[test]
  fn scopes_2() {
    let code = r#"
      let x = 1;
      let y = {
        x
      };
      x + y
    "#;
    if let Ok(p) = parse_config(code) {
      assert_eq!(eval_config(p, &mut Context::new(None)).unwrap(), Value::Number(2.0));
    } else {
      panic!();
    }
  }

  #[test]
  fn scopes_3() {
    let code = r#"
      let foo = 1;
      def bar = {
        foo
      };
      {
        let foo = 2;
        bar()
      }
    "#;
    if let Ok(p) = parse_config(code) {
      assert_eq!(eval_config(p, &mut Context::new(None)).unwrap(), Value::Number(1.0));
    } else {
      panic!();
    }
  }

  #[test]
  fn scopes_4() {
    let code = r#"
      let foo = 2;
      let foo = foo * 2;
      let foo = foo * 2;
      foo
    "#;
    if let Ok(p) = parse_config(code) {
      assert_eq!(eval_config(p, &mut Context::new(None)).unwrap(), Value::Number(8.0));
    } else {
      panic!();
    }
  }

  #[test]
  fn scopes_5() {
    let code = r#"
      def foo = 2;
      def foo = foo() * 2;
      def foo = foo() * 2;
      foo()
    "#;
    if let Ok(p) = parse_config(code) {
      assert_eq!(eval_config(p, &mut Context::new(None)).unwrap(), Value::Number(8.0));
    } else {
      panic!();
    }
  }

  #[test]
  fn struct_field_access() {
    let code = r#"
      foo.bar.baz
    "#;
    if let Ok(p) = parse_config(code) {

      let mut bar = HashMap::new();
      bar.insert("baz".to_string(), Value::Number(42.0));
      let bar = Value::Struct(bar);

      let mut foo = HashMap::new();
      foo.insert("bar".to_string(), bar);
      let foo = Value::Struct(foo);

      let mut ctx = Context::new(None);
      ctx.insert_var("foo", foo);

      assert_eq!(eval_config(p, &mut ctx).unwrap(), Value::Number(42.0));
    } else {
      panic!();
    }
  }

  #[test]
  fn variables() {
    let code = r#"
      let x, y, z = {1, 2, 3};
      x + y + z
    "#;
    if let Ok(p) = parse_config(code) {
      assert_eq!(eval_config(p, &mut Context::new(None)).unwrap(), Value::Number(6.0));
    } else {
      panic!();
    }
  }

  #[test]
  fn strings() {
    let code = r#"
      "Hello there!\n"
    "#;
    if let Ok(p) = parse_config(code) {
      assert_eq!(eval_config(p, &mut Context::new(None)).unwrap(), Value::String("Hello there!\n".to_string()))
    } else {
      panic!();
    }
  }
}
