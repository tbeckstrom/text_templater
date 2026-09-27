---
name: snippet-to-toml
description: Convert pasted snippet-language text (SNIPPET:/NOTE: blocks with LIST:, MULTILIST(...):, OPTIONAL:, REPEAT:, PRESET: definitions, produced by prompts/templater_snippet_prompt.md) into Templater TOML files in my_templates/. Use whenever the user pastes text containing "SNIPPET:" or "NOTE:" blocks, or asks to convert/import snippets.
---

# Snippet language → Templater TOML

The user generates OMS note snippets in another chat using `prompts/templater_snippet_prompt.md`
(read it for the full language) and pastes the result here. Convert every block into TOML under
`my_templates/`, verify it, and report. Never change the clinical wording — convert it exactly,
including punctuation. If the paste breaks the language (undefined placeholder, unknown `@name@`,
mixed short/titled options), stop and ask rather than guessing.

Worked examples of every rule below: `my_templates/partials/op_indications_enucleation_cyst.toml`
(nested choices, required lists, blanks), `op_approach_submandibular.toml` (titled options,
embedded snippet), `common_local_anesthesia_admin.toml` (and-join, defaults),
`op_findings_mandible_fracture.toml` (OPTIONAL line). Match their layout.

## Files

| Block | File |
|---|---|
| `SNIPPET: name` | `my_templates/partials/<name>.toml` |
| `NOTE: name` + `NAME: Title` | `my_templates/<name>.toml` with `name = "<Title>"` |

A revision of an existing snippet (same SNIPPET name) overwrites that file — show the user a
short summary of what changed. Start each file with a comment line
`# SNIPPET: <name>   PREFIX: <prefix>`, then `name = "Snippet: <TITLE>"` — that line makes the
snippet selectable in the app on its own (listed as id `partials/<name>`). `body` must come
before any `[[fields]]` (TOML attaches later keys to the last table).

## Keys

- List/OPTIONAL/REPEAT `NAME` → `<prefix>_<name lowercased>`; if the lowercased name already starts
  with `<prefix>_`, don't double it (`LOCAL_TYPES` in prefix `local` → `local_types`).
- `***(label)` → `<prefix>_<slug(label)>` (slug: lowercase, runs of non-alphanumerics → `_`,
  trimmed). Bare `***` → name it from the sentence. On a clash with another key, append `_2`.
- Never end a key in `_text` (reserved for option expansions).
- Field `label`: the `***(label)` text, or the list name humanized (`FACIAL_VESSELS_STATEMENT` →
  "Facial vessels" is fine — short and clear). Every field gets `section = "<TITLE>"`, or the
  current `SECTION:` name if one precedes its definition.

## Controls

| Language | TOML field |
|---|---|
| `LIST:` | `type = "dropdown"` |
| `MULTILIST(join):` | `type = "multiselect"` |
| `OPTIONAL: NAME (default on/off)` | `type = "checkbox"`, `default = true/false` |
| `***(label)` | `type = "text"`, `default = "***"` (never required) |
| `REPEAT(join): NAME (label: X)` | `[[groups]]` `key`, `label = "X"`; lists marked `(in NAME)` become its `[[groups.fields]]` (keys without prefix) |
| `PRESET: Label` | `[[speed_buttons]]` `label`, `values = { key = value, ... }` (multiselect → array, OPTIONAL → bool) |

Options: short options → `options = ["a", "b"]`; titled options →
`options = [{ label = "Title", text = "sentence" }, ...]`.

Defaults: `(default)` on a LIST option → `default = "<option label>"`; on MULTILIST options →
`default = [...]`. **A LIST with no `(default)` gets `required = true`** — an unset dropdown
renders as empty text, so it must block Copy (like an unresolved Epic SmartList). A MULTILIST
with no default stays optional (nothing picked is valid).

## Body

Placeholder → Tera, in place:

| Control | Renders as |
|---|---|
| LIST, short options | `{{ k }}` |
| LIST, titled | `{{ k_text }}` |
| MULTILIST(and / or) | `{% for v in k %}{% if not loop.first %}{% if loop.last %} and {% else %}, {% endif %}{% endif %}{{ v }}{% endfor %}` (`or` for or) |
| MULTILIST(comma / space / newline) | `{{ k \| join(sep=", ") }}` / `sep=" "` / `sep="\n"` — titled: use `k_text` |
| OPTIONAL | `{% if k %}text{% endif %}` — if it sits on its own line, move the newline inside: `previous line.{% if k %}\n- line{% endif %}` so "off" leaves no blank line |
| REPEAT | `{% for b in k %}{% if not loop.first %}<join>{% endif %}...{{ b.field }}...{% endfor %}` |
| `@name@` | `{% include "name" %}`, and add `"name"` to this file's `use_partials` |
| `***(label)` | `{{ k }}` |

A titled MULTILIST that can be empty and sits mid-paragraph: wrap it so the leading space
disappears too — `{% if k_text %} {{ k_text | join(sep=" ") }}{% endif %}`.

### Nested choices (a placeholder inside option text)

Option `text` is static, so:
1. In the option's `text`, replace the inner placeholder with the marker `[[<inner key>]]`.
2. Define the inner control as its own field with `visible_if` true exactly when a parent option
   containing it is picked: `"'Opt A' in parent"` (multiselect, `or`-ed across options) or
   `"parent == 'Opt A'"` (dropdown). Inner LISTs still get `required = true` — hidden fields never
   block Copy.
3. Where the parent renders, chain one `| replace(from="[[<inner key>]]", to=<value>)` per inner
   control. For an inner MULTILIST, first build its joined string at the top of the body:
   `{%- set <k>_joined %}...join loop...{% endset -%}` and use `to=<k>_joined`; an inner titled
   LIST uses `to=<k>_text`.

## Common pitfalls

- A placeholder starting a sentence with lowercase options renders lowercase — point it out to
  the user rather than editing their options.
- Two snippets in one NOTE both embedding the same `common_…` snippet is fine: it's asked once.
- Don't create a wrapper NOTE the user didn't ask for — named snippets are already selectable.

## Verify, then report

1. Render: copy `my_templates/` to the scratchpad, add a throwaway `demo.toml` with
   `use_partials` + `{% include %}` for each new/changed snippet, and run
   `cargo run -q --bin check_templates -- <scratch dir> demo`. Also inject defaults into a scratch
   copy to exercise nested choices. Show the user the rendered text.
2. `cargo run -q --bin check_templates -- my_templates` must have no load errors or lint
   warnings (named snippets are rendered standalone, so this also renders every snippet with
   its defaults).
3. Report the files written. Remind the user to install with:

```bash
rsync -av --exclude shared_fields.toml my_templates/ "$HOME/Library/Application Support/note-templater/templates/"
```

(`shared_fields.toml` lives only in the app folder; `my_templates/` never carries one, and the
snippet language doesn't use `use_shared`.)

then click **Reload Templates** in the app.
