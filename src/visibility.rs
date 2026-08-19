//! Evaluation of `visible_if` expressions — the small Tera conditions a
//! template uses to show a field, group, or speed button only when it's
//! relevant (e.g. carpule counts only once that anesthetic is selected).

use std::collections::{HashMap, HashSet};

use tera::{Context, Tera};

/// Compiles each distinct `visible_if` expression once and reuses it, so a
/// form with dozens of conditions doesn't re-parse them every frame.
#[derive(Default)]
pub struct ExprEvaluator {
    tera: Tera,
    /// Expressions we've attempted to register (successfully or not), so a
    /// broken one isn't retried on every frame.
    seen: HashSet<String>,
}

impl ExprEvaluator {
    /// Whether `expr` is truthy for `ctx`.
    ///
    /// Fails **open**: an expression that won't compile or errors while
    /// rendering reports visible, so a typo surfaces as an always-shown field
    /// rather than silently hiding part of the form.
    pub fn truthy(&mut self, expr: &str, ctx: &Context) -> bool {
        if self.seen.insert(expr.to_string()) {
            let source = format!("{{% if {expr} %}}1{{% else %}}0{{% endif %}}");
            if self.tera.add_raw_template(expr, &source).is_err() {
                return true;
            }
        }
        self.tera
            .render(expr, ctx)
            .map(|out| out.trim() == "1")
            .unwrap_or(true)
    }
}

/// Which scope a `visible_if` was evaluated in: the form as a whole, or one
/// repeatable-group instance (whose own field values are also in scope).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Form,
    Instance(u64),
}

/// The per-frame answers, looked up while drawing the form.
#[derive(Default)]
pub struct Visibility {
    results: HashMap<(String, Scope), bool>,
}

impl Visibility {
    pub fn record(&mut self, expr: &str, scope: Scope, visible: bool) {
        self.results.insert((expr.to_string(), scope), visible);
    }

    /// Whether something guarded by `expr` should be shown. No `visible_if`
    /// (or an expression that wasn't evaluated this frame) means "always".
    pub fn is_visible(&self, expr: Option<&String>, scope: Scope) -> bool {
        let Some(expr) = expr else { return true };
        self.results
            .get(&(expr.clone(), scope))
            .copied()
            .unwrap_or(true)
    }
}

/// A context with `overrides` layered on top of `base` — used so a condition
/// inside a repeatable block can see that block's own fields (shadowing the
/// template's top-level ones) as well as everything outside it.
pub fn context_with_overrides(base: &Context, overrides: &serde_json::Value) -> Context {
    let mut ctx = base.clone();
    if let Some(map) = overrides.as_object() {
        for (key, value) in map {
            ctx.insert(key.clone(), value);
        }
    }
    ctx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_with(pairs: &[(&str, serde_json::Value)]) -> Context {
        let mut ctx = Context::new();
        for (k, v) in pairs {
            ctx.insert((*k).to_string(), v);
        }
        ctx
    }

    #[test]
    fn simple_truthy_and_falsy_conditions() {
        let mut ev = ExprEvaluator::default();
        let ctx = ctx_with(&[
            ("ivga", serde_json::json!(true)),
            ("nitrous", serde_json::json!(false)),
        ]);
        assert!(ev.truthy("ivga", &ctx));
        assert!(!ev.truthy("nitrous", &ctx));
        assert!(ev.truthy("ivga and not nitrous", &ctx));
    }

    #[test]
    fn membership_and_comparison_conditions() {
        let mut ev = ExprEvaluator::default();
        let ctx = ctx_with(&[
            ("local_used", serde_json::json!(["Lidocaine", "Topical"])),
            ("age", serde_json::json!(47)),
        ]);
        assert!(ev.truthy(r#""Lidocaine" in local_used"#, &ctx));
        assert!(!ev.truthy(r#""Articaine" in local_used"#, &ctx));
        assert!(ev.truthy("age < 50", &ctx));
    }

    #[test]
    fn broken_expression_fails_open_to_visible() {
        let mut ev = ExprEvaluator::default();
        let ctx = ctx_with(&[("a", serde_json::json!(true))]);
        assert!(
            ev.truthy("this is not ( valid tera", &ctx),
            "a malformed condition should show the field, not hide it"
        );
    }

    #[test]
    fn unknown_variable_does_not_hide_everything() {
        let mut ev = ExprEvaluator::default();
        let ctx = ctx_with(&[]);
        // Undefined in Tera is falsy in a boolean position, which is the
        // sensible reading of "that field isn't set".
        assert!(!ev.truthy("never_defined", &ctx));
    }

    #[test]
    fn instance_values_shadow_top_level_ones() {
        let base = ctx_with(&[("multiple_teeth", serde_json::json!(false))]);
        let overrides = serde_json::json!({ "multiple_teeth": true });
        let merged = context_with_overrides(&base, &overrides);

        let mut ev = ExprEvaluator::default();
        assert!(!ev.truthy("multiple_teeth", &base));
        assert!(ev.truthy("multiple_teeth", &merged));
    }

    #[test]
    fn visibility_lookup_defaults_to_shown() {
        let mut v = Visibility::default();
        assert!(v.is_visible(None, Scope::Form));
        assert!(v.is_visible(Some(&"never_evaluated".to_string()), Scope::Form));

        v.record("hidden_one", Scope::Form, false);
        assert!(!v.is_visible(Some(&"hidden_one".to_string()), Scope::Form));
        // Same expression, different instance -> independent answer.
        assert!(v.is_visible(Some(&"hidden_one".to_string()), Scope::Instance(7)));
    }
}
