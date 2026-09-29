//! Evaluation of the small Tera expressions a template embeds in its *form*
//! rather than its body: `visible_if` conditions deciding whether a control is
//! relevant, and `compute` expressions deriving one field's value from others.

use std::collections::{HashMap, HashSet};

use tera::{Context, Tera};

use crate::field::{FieldValue, FormState};
use crate::template::{FieldDef, FieldType};

/// Compiles each distinct `visible_if` expression once and reuses it, so a
/// form with dozens of conditions doesn't re-parse them every frame.
#[derive(Default)]
pub struct ExprEvaluator {
    tera: Tera,
    /// Expressions we've attempted to register, so a broken one isn't
    /// re-parsed on every frame.
    seen: HashSet<String>,
    /// Of those, the ones that wouldn't compile.
    failed: HashSet<String>,
}

impl ExprEvaluator {
    /// Registers `source` under `name` the first time that name is seen.
    /// Returns whether the template is available to render.
    fn ensure(&mut self, name: &str, source: &str) -> bool {
        if self.seen.insert(name.to_string()) && self.tera.add_raw_template(name, source).is_err() {
            self.failed.insert(name.to_string());
        }
        !self.failed.contains(name)
    }

    /// Whether `expr` is truthy for `ctx`.
    ///
    /// Fails **open**: an expression that won't compile or errors while
    /// rendering reports visible, so a typo surfaces as an always-shown field
    /// rather than silently hiding part of the form.
    pub fn truthy(&mut self, expr: &str, ctx: &Context) -> bool {
        let name = format!("if:{expr}");
        let source = format!("{{% if {expr} %}}1{{% else %}}0{{% endif %}}");
        if !self.ensure(&name, &source) {
            return true;
        }
        self.tera
            .render(&name, ctx)
            .map(|out| out.trim() == "1")
            .unwrap_or(true)
    }

    /// The value `expr` produces for `ctx`: a list when it evaluates to an
    /// array, otherwise its text. `None` if the expression won't compile or
    /// errors — the caller shows that as an empty value rather than failing
    /// the whole form.
    pub fn value(&mut self, expr: &str, ctx: &Context) -> Option<FieldValue> {
        let name = format!("val:{expr}");
        // A plain value comes out behind SCALAR; each list item ends in ITEM.
        let source = format!(
            "{{% set v = {expr} %}}{{% if v is array %}}{{% for x in v %}}{{{{ x }}}}{ITEM}{{% endfor %}}\
             {{% else %}}{SCALAR}{{{{ v }}}}{{% endif %}}"
        );
        if !self.ensure(&name, &source) {
            return None;
        }
        let out = self.tera.render(&name, ctx).ok()?;
        Some(match out.strip_prefix(SCALAR) {
            Some(text) => FieldValue::Text(text.to_string()),
            None => FieldValue::MultiSelect(out.split_terminator(ITEM).map(str::to_owned).collect()),
        })
    }
}

/// Markers [`ExprEvaluator::value`] frames its output with: control
/// characters, so no value can contain them.
const SCALAR: char = '\u{1e}';
const ITEM: char = '\u{1f}';

/// Recomputes every `computed` field in `fields`, writing each result into
/// `state` and into `ctx` — so another computed field, a `visible_if`, and the
/// note body all see the fresh value. Fields are evaluated in declared order,
/// and again while anything changed, so one can build on another declared
/// before or after it (a partial used on its own lists its fields before
/// those of the partials it pulls in).
pub fn recompute_fields(
    fields: &[FieldDef],
    state: &mut FormState,
    ctx: &mut Context,
    evaluator: &mut ExprEvaluator,
) {
    let computed: Vec<&FieldDef> = fields
        .iter()
        .filter(|f| f.field_type == FieldType::Computed)
        .collect();
    // A chain of n fields settles within n passes; the cap stops two fields
    // that feed each other from looping forever.
    for _ in 0..=computed.len() {
        let mut changed = false;
        for field in &computed {
            let value = field
                .compute
                .as_deref()
                .and_then(|expr| evaluator.value(expr, ctx))
                .unwrap_or_else(|| FieldValue::Text(String::new()));
            let json = value.to_json();
            changed |= state.get(&field.key).map(FieldValue::to_json).as_ref() != Some(&json);
            ctx.insert(field.key.clone(), &json);
            state.insert(field.key.clone(), value);
        }
        if !changed {
            break;
        }
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

    fn computed(key: &str, expr: &str) -> FieldDef {
        FieldDef {
            key: key.to_string(),
            label: key.to_string(),
            field_type: FieldType::Computed,
            compute: Some(expr.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn a_computed_field_derives_its_value_from_the_others() {
        let fields = vec![computed("summary", r#"name ~ " (" ~ age ~ " yo)""#)];
        let mut state = FormState::new();
        let mut ctx = ctx_with(&[
            ("name", serde_json::json!("Alex")),
            ("age", serde_json::json!(47)),
        ]);
        let mut ev = ExprEvaluator::default();

        recompute_fields(&fields, &mut state, &mut ctx, &mut ev);

        match state.get("summary").unwrap() {
            FieldValue::Text(s) => assert_eq!(s, "Alex (47 yo)"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn a_computed_field_can_build_on_an_earlier_one() {
        let fields = vec![
            computed("doubled", "n * 2"),
            computed("quadrupled", "doubled | int * 2"),
        ];
        let mut state = FormState::new();
        let mut ctx = ctx_with(&[("n", serde_json::json!(3))]);
        let mut ev = ExprEvaluator::default();

        recompute_fields(&fields, &mut state, &mut ctx, &mut ev);

        match state.get("quadrupled").unwrap() {
            FieldValue::Text(s) => assert_eq!(s, "12"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn a_computed_field_can_build_on_a_later_one() {
        let fields = vec![computed("quadrupled", "doubled | int * 2"), computed("doubled", "n * 2")];
        let mut state = FormState::new();
        let mut ctx = ctx_with(&[("n", serde_json::json!(3))]);
        let mut ev = ExprEvaluator::default();

        recompute_fields(&fields, &mut state, &mut ctx, &mut ev);

        match state.get("quadrupled").unwrap() {
            FieldValue::Text(s) => assert_eq!(s, "12"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn a_computed_list_stays_a_list() {
        let fields = vec![computed("risks", r#"[r for r in ["nerve" if lower else "", "sinus"] if r]"#)];
        let mut state = FormState::new();
        let mut ctx = ctx_with(&[("lower", serde_json::json!(true))]);
        let mut ev = ExprEvaluator::default();

        recompute_fields(&fields, &mut state, &mut ctx, &mut ev);

        match state.get("risks").unwrap() {
            FieldValue::MultiSelect(v) => assert_eq!(v, &["nerve", "sinus"]),
            other => panic!("expected a list, got {other:?}"),
        }
        assert!(ev.truthy(r#""nerve" in risks and risks | length == 2"#, &ctx));
    }

    #[test]
    fn a_computed_value_is_visible_to_later_conditions() {
        let fields = vec![computed("total", "a + b")];
        let mut state = FormState::new();
        let mut ctx = ctx_with(&[
            ("a", serde_json::json!(2)),
            ("b", serde_json::json!(3)),
        ]);
        let mut ev = ExprEvaluator::default();
        recompute_fields(&fields, &mut state, &mut ctx, &mut ev);

        assert!(ev.truthy(r#"total == "5""#, &ctx));
    }

    #[test]
    fn a_broken_compute_expression_yields_an_empty_value_not_a_crash() {
        let fields = vec![computed("oops", "this is ( not valid")];
        let mut state = FormState::new();
        let mut ctx = ctx_with(&[]);
        let mut ev = ExprEvaluator::default();

        recompute_fields(&fields, &mut state, &mut ctx, &mut ev);

        match state.get("oops").unwrap() {
            FieldValue::Text(s) => assert_eq!(s, ""),
            other => panic!("expected empty Text, got {other:?}"),
        }
    }

    #[test]
    fn the_same_expression_text_works_as_both_a_condition_and_a_value() {
        // `truthy` and `value` register under separate names, so reusing one
        // expression string for both doesn't collide in the template cache.
        let mut ev = ExprEvaluator::default();
        let ctx = ctx_with(&[("n", serde_json::json!(5))]);
        assert!(ev.truthy("n", &ctx));
        assert!(matches!(ev.value("n", &ctx), Some(FieldValue::Text(s)) if s == "5"));
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
