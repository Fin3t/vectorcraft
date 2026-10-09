//! References to earlier results in multi-step runs (`command.batch`, `vectorcraft-cli run`): a
//! string param `"$N"` or `"$N.key.0.key"` is step N's result (1-based) or a value inside it, e.g.
//! `{"ids": ["$1.id", "$2.id"]}`. The same notation as FilmCraft's and EffectCraft's scripts. A
//! string that starts with `$$` is literal text with one `$` (`"$$5.00"` is `"$5.00"`).

use serde_json::Value;

/// Resolve a `$N` / `$N.key.0.key` reference against the results so far; `None` when `s` is not
/// a reference (an ordinary string).
fn reference(s: &str, results: &[Value]) -> Option<Result<Value, String>> {
    let rest = s.strip_prefix('$')?;
    let mut parts = rest.split('.');
    let n: usize = parts.next()?.parse().ok()?;
    let path: Vec<&str> = parts.collect();
    if path.iter().any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')) {
        return None;
    }
    let Some(mut v) = n.checked_sub(1).and_then(|i| results.get(i)) else {
        return Some(Err(format!("`{s}` refers to step {n}, but only {} step(s) ran before", results.len())));
    };
    for p in &path {
        let next = match (v, p.parse::<usize>()) {
            (Value::Array(a), Ok(i)) => a.get(i),
            (Value::Object(o), _) => o.get(*p),
            _ => None,
        };
        match next {
            Some(x) => v = x,
            None => return Some(Err(format!("`{s}`: step {n} returned {v}, which has no `{p}`"))),
        }
    }
    Some(Ok(v.clone()))
}

/// Replace every `$N…` string in `v` (recursively, through objects and arrays) with the
/// referenced result, and `$$…` with the text after the first `$`.
pub fn substitute(v: &Value, results: &[Value]) -> Result<Value, String> {
    Ok(match v {
        Value::String(s) if s.starts_with("$$") => Value::String(s.get(1..).unwrap_or_default().to_string()),
        Value::String(s) => match reference(s, results) {
            Some(r) => r?,
            None => v.clone(),
        },
        Value::Array(a) => Value::Array(a.iter().map(|x| substitute(x, results)).collect::<Result<_, _>>()?),
        Value::Object(o) => Value::Object(o.iter().map(|(k, x)| Ok((k.clone(), substitute(x, results)?))).collect::<Result<_, String>>()?),
        _ => v.clone(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::substitute;

    #[test]
    fn references_resolve_nested_and_leave_other_strings() {
        let results = [json!({"id": 7, "ids": [3, 4]}), json!(null)];
        let p = json!({"id": "$1.id", "ids": ["$1.ids.1", "$1"], "name": "$ 1 is not a ref", "price": "$$5.00", "n": 2});
        assert_eq!(
            substitute(&p, &results).unwrap(),
            json!({"id": 7, "ids": [4, {"id": 7, "ids": [3, 4]}], "name": "$ 1 is not a ref", "price": "$5.00", "n": 2})
        );
        assert!(substitute(&json!("$3.id"), &results).unwrap_err().contains("step 3"));
        assert!(substitute(&json!("$0"), &results).unwrap_err().contains("step 0"));
        assert!(substitute(&json!("$2.id"), &results).unwrap_err().contains("has no `id`"));
        assert!(substitute(&json!("$1.ids.9"), &results).unwrap_err().contains("has no `9`"));
    }
}
