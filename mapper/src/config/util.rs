use super::eval::Value;

pub fn flatten(value: Value) -> Vec<Value> {
  let mut result = vec![];

  fn dfs(val: Value, res: &mut Vec<Value>) {
    match val {
      Value::List(list) => {
        for x in list {
          dfs(x, res);
        }
      },
      _ => res.push(val)
    };
  }

  dfs(value, &mut result);
  result
}

pub fn strings(list: &Vec<Value>) -> Option<Vec<String>> {
  let mut v = vec![];
  for item in list {
    if let Value::String(string) = item {
      v.push(string.clone());
    } else {
      return None;
    }
  }
  Some(v)
}
