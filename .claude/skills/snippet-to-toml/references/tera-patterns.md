# Tera patterns for the newer constructs

The TOML and Tera each snippet-language construct becomes, beyond the simple placeholders in
SKILL.md. The implant consult is the working reference for every pattern here:
`my_templates/partials/common_implant_case.toml` (TEETH helpers, regions),
`clinic_exam_implant.toml` (lean lines, PARTS, span/tooth REPEATs, optional add),
`clinic_imaging_implant.toml`, `clinic_risks_implant.toml` (PARTS with conditions, USES optional),
and `clinic_assessment_plan_implant.toml` (plan lines, AUTO bullets, pathway presets).

## `***` flag options

Short list: `"***"` as the last option. Titled list: `{ label = "***", text = "***" }`.

## Lean `?` lines

Collect the snippet's `?` lines into one list, so any of them (including the first) can drop out
without leaving a blank line:

```
{%- set lines = [
  ("Oral hygiene: " ~ imp_exam_oral_hygiene) if imp_exam_oral_hygiene else "",
  ("Periodontal status: " ~ imp_exam_perio_status_text) if imp_exam_perio_status else "",
] -%}
{%- for l in lines %}{% if l %}- {{ l }}
{% endif %}{% endfor %}
```

A single `?` line between fixed lines can instead carry its own line break inside the `if`:
`previous line.{% if k %}\n- Label: {{ k }}{% endif %}`.

"Has a value" is the bare key for every control: a dropdown key is `""` when unpicked, a
multiselect `[]`, a checkbox `false`, a blank `""`. Test the key; print `<key>_text` for titled
options. Every control on the line joins the test with `and`.

## PARTS

```
{%- set parts = [
  ("bone height " ~ b.bone_height) if b.bone_height else "",
  (b.vital_structure ~ " " ~ b.proximity) if b.vital_structure and b.proximity else "",
  "injury to adjacent teeth" if <condition> else "",
] -%}
{%- set parts = [p for p in parts if p] | unique -%}
```

Join words:
- semicolon: `{{ parts[:-1] | join(sep="; ") }}; and {{ parts[-1] }}` when there's more than one,
  otherwise `{{ parts | first }}`.
- and: the MULTILIST and-loop over `parts`.
- comma: `join(sep=", ")`.

A `?` line holding the PARTS tests `parts` (non-empty).

## TEETH

- `TEETH: NAME` becomes `type = "teeth"`; the loader supplies the 52 FDI options.
- `same chart as: OTHER` puts `linked = "<this key>"` on OTHER's field, not on this one; only the
  linking field is drawn.
- The context gets `<key>` (teeth in FDI order) and `<key>_spans` (e.g. `["35-37", "46"]`).

Derived wording goes in hidden computed fields in the snippet that defines the TEETH, each with
`visible_if = "false"`. Keys must never end in `_text`. Create only the helpers something uses:

| Suffix | Key | `compute` |
|---|---|---|
| NAME | `<key>_list` | `(k[:-1] \| join(sep=", ")) ~ " and " ~ k[-1] if k \| length > 1 else (k \| first if k else "")` |
| NAME.spans | `<key>_span_list` | same, over `<key>_spans` |
| NAME.site / .Site | `<key>_site_word` | `"sites" if k \| length > 1 else "site"` (capitalized variant for .Site) |
| NAME.tooth / .Tooth | `<key>_tooth_word` | `"teeth" if k \| length > 1 else "tooth"` |
| NAME.count | `<key>_count` | `k \| length` |

A region flag is a hidden computed field. Sum the lengths when the region spans two TEETH lists:

```
compute = '"1" if ([t for t in k if t in ["34", "35", "36", "37", "38", "44", "45", "46", "47", "48"]] | length) > 0 else ""'
```

The regions are:
- maxillary posterior: 14-18, 24-28;
- mandibular posterior: 34-38, 44-48;
- esthetic zone: 11-13, 21-23, 31-33, 41-43.

## REPEAT driven by TEETH

- **Group keys:** `[[groups]]` with `source = "<teeth key>"`, `source_as = "site"`, plus `spans = true`
  for "each span of". Set `section` to the section the REPEAT is defined in, so its boxes sit
  inside that heading.
- **THIS_SITE** is `b.site`. THIS_SITE.Site is `{{ "Sites" if "-" in b.site else "Site" }}`.
- **Other snippets** read a group by its key as a list of objects. A snippet that isn't guaranteed
  to be in the note guards the read with `is defined`, e.g. the assessment describing each tooth
  present by the exam's conditions.

## Conditions (AUTO, `when`, visibility)

| Language | Tera |
|---|---|
| X is A, B | `k in ["A", "B"]` (one value: `k == "A"`) |
| X has A, B | `"A" in k or "B" in k` |
| X is set | `k` |
| X in REGION | the region flag key |
| from an `(optional)` USES snippet | `(k is defined and <clause>)` |

`or ticked: Label` adds a `type = "checkbox"` field, `default = false`, labelled `+ Label`. That
checkbox is `or`-ed into the condition.

AUTO text renders as `{% if <condition> %}text{% endif %}`. On its own line, it goes in the `lines`
list as `"text" if <condition> else ""`.

## USES, BLANK, OPTIONAL adds

- **USES** adds each non-optional snippet to `use_partials`. Placeholders map to that snippet's
  keys: read its TOML, whose first line records its prefix. An `(optional)` USES stays out of
  `use_partials`, and every read of it is guarded with `is defined`.
- **A NOTE whose snippets share a `common_...` snippet** lists it first in `use_partials` and
  includes it first in the body. Its body is `""`, so it prints nothing, but the include keeps the
  lint from reporting it unused.
- **BLANK** becomes a text field: `default = "***"`, or `default = ""` when optional; paragraph
  becomes `type = "textarea"`.
- **OPTIONAL (label: + Name)** becomes a checkbox with that label. Every control defined inside its
  text gets `visible_if = "<optional key>"`, `and`-ed with any visibility it already has.

## PRESET

- `section = "<section it was defined under>"` on the speed button.
- `(in REPEAT)` becomes `[[groups.speed_buttons]]` on that group, with unprefixed keys.
- `= none` becomes `[]` for a multiselect and `""` for a dropdown.

## Tera gotchas

- **Arrays have no `+`.** Build separate lists, or use a comprehension.
- **Computed values are strings.** Compare numbers with `| int`, e.g. `imp_site_count | int > 1`.
- **Precedence:** filters bind tighter than `~`, and `a ~ b if c else d` means `(a ~ b) if c else d`.
- **Slicing:** `list[:-1]` and `list[-1]` work; the `slice` filter doesn't.
- **Newlines:** TOML drops the newline straight after `"""`. When a body's last line already ends
  in a newline (a `lines` or REPEAT loop), close `"""` right after the final tag. Otherwise the note
  gets a double blank line between sections.
