# Note Templater

A personal desktop app for writing structured notes (e.g. "Follow-up Visit," "Initial Consult")
quickly and accurately. Pick a template, fill in a generated form, watch the note render live,
then copy it to your clipboard. Templates are plain TOML files you can add or edit yourself.

## Running

```
cargo run
```

On first launch the app creates its data directory (on macOS:
`~/Library/Application Support/note-templater/`) and seeds it with four example templates
(`follow_up_visit.toml`, `initial_consult.toml`, `oral_surgery.toml`, `oms_procedure_note.toml`),
`shared_fields.toml`, and a `partials/` directory. Edit these files, add your own `.toml` files alongside them, and click
**Reload Templates** in the app to pick up changes without restarting.

## Writing a template

Each template is one `.toml` file in the templates directory:

```toml
id = "follow_up_visit"        # optional; defaults to the filename (without .toml)
name = "Follow-up Visit"      # shown in the template list
description = "Standard follow-up visit note"   # optional
use_shared = ["patient_name", "dob", "visit_date"]  # optional, see "Shared fields" below

# `body` is the Tera template for the rendered note. IMPORTANT: it must be written
# BEFORE any [[fields]] blocks below it — in TOML, keys written after an
# array-of-tables attach to the last table entry, not the document root, so a
# trailing `body = "..."` after your last [[fields]] block would silently vanish.
body = """
Patient: {{ patient_name }} (DOB: {{ dob }})
{% if follow_up_needed %}Follow-up recommended in {{ follow_up_weeks }} week(s).{% endif %}
"""

[[fields]]
key = "follow_up_needed"      # the variable name used in `body`
label = "Follow-up needed?"   # shown next to the form control
type = "checkbox"
default = false

[[fields]]
key = "follow_up_weeks"
label = "Follow-up in (weeks)"
type = "number"
min = 1
max = 52
default = 2
```

`body` is rendered with [Tera](https://keats.github.io/tera/docs/#templates) — supports
`{% if %}` / `{% else %}`, `{% for %}` loops, filters, etc. Every field's `key` becomes a
variable available in `body`.

### Formatting

Anywhere in `body` (static text or a field's rendered value), wrap text in these markers for
light formatting:

| Marker | Effect |
|---|---|
| `**bold**` | bold |
| `*italic*` | italic |
| `__underline__` | underline |

The preview shows bold as an emphasized color rather than a heavier font weight (egui doesn't
bundle a bold font face), but the *copied* result gets real bold when pasted into an app with its
own bold font. Two buttons handle the copy: **Copy to Clipboard** copies the formatted version
(with a plain-text fallback built in, for apps that don't support rich paste), and **Copy Plain
Text** always copies with every marker stripped, no formatting. This isn't Markdown — just these
three toggles.

A marker only takes effect when it **hugs the text it formats** and is **closed on the same
line**. That keeps fill-in blanks and ordinary punctuation intact:

| Text | Result |
|---|---|
| `**Heading**` | bold |
| `# __ to # __` | left alone — the marker is followed by a space |
| `The ___ root` | left alone — a run of the same character isn't a marker |
| `# ***` | left alone — no closing partner on that line |
| `snake_case` | left alone — a single `_` is never a marker |
| `**unclosed` | left alone — an unbalanced marker stays visible, so the typo is obvious |

So blanks you leave for manual completion survive into the note rather than being silently
deleted, and a stray marker can't run away and format the rest of the document.

### Field types

| `type`        | Widget                  | Value in `body`                          |
|---------------|--------------------------|-------------------------------------------|
| `text`        | single-line text box     | string                                     |
| `textarea`    | multi-line text box      | string                                     |
| `number`      | numeric stepper          | number (`min`/`max` optional)              |
| `dropdown`    | single-select combo box  | string (needs `options = [...]`)           |
| `checkbox`    | checkbox                 | boolean                                    |
| `multiselect` | one checkbox per option  | list of strings (needs `options = [...]`, loop over it with `{% for %}`) |
| `date`        | text box + calendar picker | ISO string `YYYY-MM-DD` (`default = "today"` fills today's date); typed text that doesn't parse as a date is shown in red and blocks the Copy button |

Every field supports: `key` (required), `label` (required), `type` (required),
`required` (bool, default `false` — blocks the Copy button until filled), `default`, and
`visible_if` (below).

Whole numbers render without a decimal point, so `{{ carpules }}` gives `2`, not `2.0`.

#### Computed fields

`type = "computed"` is a read-only value worked out from the other fields by a Tera expression
rather than typed in:

```toml
[[fields]]
key = "total_local_ml"
label = "Total local anesthetic (mL)"
type = "computed"
compute = "(lidocaine_carpules + bupivacaine_carpules) * 1.7"
```

It's recalculated whenever the form changes, and the result is available to the body, to
`visible_if` conditions, and to later computed fields (they're evaluated in declared order, so one
can build on another). A computed field never blocks the Copy button, and isn't stored in
history/drafts — it's derived again from the saved inputs. An expression that won't compile shows
as blank rather than breaking the form.

#### Grouping fields into collapsible sections

`section` puts a field under a foldable heading. Consecutive fields naming the same section are
drawn together:

```toml
[[fields]]
key = "ivga"
label = "IV general anesthesia"
type = "checkbox"
section = "Anesthesia"
```

Fields without a `section` are drawn plainly, so existing templates look unchanged. A section
whose fields are all hidden by `visible_if` is skipped rather than left as an empty heading.

#### Options that expand to longer prose

A `dropdown`/`multiselect` choice can be written as a table carrying the sentence it stands for,
so the form shows a short label while the note gets the full wording:

```toml
[[fields]]
key = "socket"
label = "Socket treatment"
type = "multiselect"
options = [
  { label = "Curretted", text = "Gently curetted the socket" },
  { label = "Irrigated", text = "Irrigated the site with copious sterile saline" },
]
```

The template then gets `socket` (the labels you picked) *and* `socket_text` (their expansions):

```
{{ socket_text | join(sep=". ") }}
```

Expansions always come out in **declared option order**, not the order you happened to tick them,
so prose reads in the sequence the template author intended. (Multiselect values are stored in
declared order too.) For a `dropdown`, `<key>_text` is a single string. A choice with no `text`
falls back to its label, and a field where no choice defines `text` publishes no `_text` at all.

#### Showing a field only when it's relevant

`visible_if` takes a Tera expression evaluated against the current form values:

```toml
[[fields]]
key = "lidocaine_carpules"
label = "Lidocaine carpules"
type = "number"
visible_if = "'Lidocaine' in anesthetics"
```

It works on `[[fields]]`, `[[groups]]` and `[[speed_buttons]]`, and inside a group it can also see
that block's own fields (`visible_if = "multiple_teeth"`). A hidden field never blocks the Copy
button, even if it's `required`. An expression that won't compile shows the field rather than
hiding it, so a typo is visible instead of silent.

### Shared fields

Fields you reuse across most templates (patient name, DOB, visit date, ...) can be defined once
in `shared_fields.toml`:

```toml
[[fields]]
key = "patient_name"
label = "Patient Name"
type = "text"
required = true
```

Then pull them into any template with `use_shared = ["patient_name", ...]` — they're prepended
to that template's own `[[fields]]`, in the order listed, so they appear first in the form and
are available in `body` exactly like any other field.

`shared_fields.toml` reuses *form fields* across templates. To reuse a chunk of the rendered
*note text* itself, see Composable templates below.

### Composable templates

A `partials/` subdirectory next to your templates holds reusable body sections. Each `.tera` file
in it becomes includable, by filename (without `.tera`), from any template's `body`:

```
# partials/header.tera
Patient: {{ patient_name }} (DOB: {{ dob }})
Visit Date: {{ visit_date }}
```

```toml
# any template.toml
body = """
{% include "header" %}
Visit Type: {{ visit_type }}
...
"""
```

Edit `partials/header.tera` once and every template that includes it updates. A template can
include as many partials as it wants, in any order, and mix them with its own content — this is
just [Tera's `{% include %}`](https://keats.github.io/tera/docs/#include), so anything a `.tera`
partial references (like `patient_name` above) still needs to be one of that template's fields
(own or shared).

#### Partials that bring their own fields

A partial written as `partials/<name>.toml` instead of `.tera` can also carry the form controls
its text needs — `[[fields]]`, `[[groups]]` (with their own `[[groups.fields]]` and
`[[groups.speed_buttons]]`), and `[[speed_buttons]]`. That lets a whole section move between
notes as one unit:

```toml
# partials/procedure_blocks.toml
body = """
{% for b in blocks %}{{ b.section_header }}: ...
{% endfor %}
"""

[[groups]]
key = "blocks"
label = "Procedure Block"

[[groups.fields]]
key = "section_header"
label = "Tooth / area"
type = "text"
```

```toml
# any note that wants that section
use_partials = ["procedure_blocks"]
body = """
...
{% include "procedure_blocks" %}
...
"""
```

`use_partials` is what pulls the controls in — a template that only `{% include %}`s a `.toml`
partial without listing it gets the text but not the fields (and will fail to render). Each
template gets its own copy of the controls, so two notes sharing a partial keep separate values.
Partial fields are appended after the template's own. A key defined in both the template and a
partial is reported as a load error rather than silently shadowing. A `.toml` partial may
`use_shared`, but partials can't currently include other partials.

### Speed buttons

Buttons that fill in several fields at once for a frequently-used pattern — e.g. one click sets
both the visit reason and the medical history for a routine visit:

```toml
[[speed_buttons]]
label = "Annual Visit"
values = { chief_complaint = "Annual check up", follow_up_needed = false }
```

`values` uses the same TOML shape as a field's `default` (strings, numbers, booleans, or arrays
for `multiselect`). They render as a row of buttons above the form fields; a key that doesn't
match a current field, or whose value doesn't match that field's type, is just skipped rather
than erroring — a speed button always does what it can with the rest.

A speed button can also (or instead) append pre-filled instances to a repeatable group — see
below.

### Repeatable groups

For a section that can occur any number of times per note — e.g. one "Procedure Step" per tooth
worked on — define a `[[groups]]` with its own `[[groups.fields]]`, just like the top-level
`[[fields]]`:

```toml
[[groups]]
key = "procedures"        # the variable name used in `body`
label = "Procedure Step"  # shown on the "+ Add" button and each instance's header

[[groups.fields]]
key = "tooth"
label = "Tooth #"
type = "text"
required = true

[[groups.fields]]
key = "procedure_type"
label = "Procedure"
type = "dropdown"
options = ["Extraction", "Implant", "Bone Graft"]
```

This renders as its own section below the regular fields, with "+ Add Procedure Step" appending a
new instance (each its own little form, with a "Remove" button) and no limit on how many you can
add — including zero. In `body`, the group's key is an array of objects, one per instance, with
each field as a property:

```
{% for p in procedures %}
  {{ loop.index }}. Tooth {{ p.tooth }}: {{ p.procedure_type }}
{% endfor %}
```

A `[[speed_buttons]]` can append one or more pre-filled instances at once by setting `group` (the
group's `key`) and `group_values` (a list of tables, one per instance to add, in the same shape as
`[[groups.fields]]` defaults) — handy for "add a whole quadrant at once":

```toml
[[speed_buttons]]
label = "+ Upper Right Quadrant (1-8)"
group = "procedures"
group_values = [
  { tooth = "1", procedure_type = "Extraction" },
  { tooth = "2", procedure_type = "Extraction" },
  # ...
]
```

A required field inside a group instance blocks Copy the same way a top-level required field
does. See `examples/templates/oral_surgery.toml` for a complete example.

A group can also carry its own `[[groups.speed_buttons]]`, which fill in **only the block they sit
in** — handy when each block is one of a handful of stereotyped procedures.

#### Groups driven by a selection

Instead of manual add/remove, a group can track a `multiselect`: set `source` to that field's key
and you get exactly one block per selected choice, created and removed as the selection changes.
`source_as` names the choice inside each block (default `item`):

```toml
[[groups]]
key = "tooth_diagnoses"
label = "Diagnosis"
source = "diagnosis_teeth"   # a multiselect of tooth numbers
source_as = "tooth"          # -> {{ t.tooth }} in the body
```

Blocks keep whatever you've typed into them when unrelated choices are added or removed, and they
survive a draft/history round-trip. Source-driven groups have no Add/Remove buttons.

See `examples/templates/oms_procedure_note.toml` for a large template using all of this together.

## Keyboard shortcuts

| Key | Action |
|---|---|
| `⌘⏎` | Copy the note (same as the Copy to Clipboard button) |
| `/` | Jump to the template search box |

## Checking templates without opening the app

```
cargo run --bin check_templates                          # the app's own templates
cargo run --bin check_templates -- examples/templates    # the bundled examples
cargo run --bin check_templates -- <dir> <template_id>   # also print that note
```

Reports load errors, lists the partials (and what each contributes), and renders every template
with default values — the quickest way to catch a broken `{% include %}`, a bad `visible_if`, or
a `use_partials` typo. Exits non-zero if anything fails, so it works in a pre-commit hook.

It also **lints** for the mistakes that load and render fine but mean the form isn't doing what
you intended:

* a field nothing reads (counting `<key>_text` expansions and `source`-driven groups as reads)
* a `visible_if` or `compute` naming a field that doesn't exist — usually a rename left behind
* a speed button setting a key no field has, so the button silently does nothing
* a group driven by a `source` that isn't a multiselect/dropdown
* a `computed` field with no `compute` expression
* a partial no template includes

Lints are warnings — they don't fail the run.

## History and Drafts

Every note you copy (from either Copy button) is appended to a local log (`history.jsonl` in the
app data directory) and shown in the **History** panel, where "Restore" reloads the form exactly
as it was when you copied, and "Copy again" re-copies — always as plain text, formatting markers
stripped, regardless of which button you originally used. Restore only appears when the entry's
saved fields still line up with the template's current fields — an entry from before this feature
existed, or from a template whose fields changed since, only offers "Copy again".

Switching templates, reloading templates, or closing the app while a form has unsaved changes
saves a **draft** (up to 10, one per template) that you can pick back up from the **Drafts**
panel with "Resume" — same restorability rule as History.
