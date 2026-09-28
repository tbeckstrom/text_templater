# Snippet language

The plain-text format every snippet is written in. The `snippet-to-toml` skill converts it to Templater TOML, so follow it exactly. How to use each construct well (defaults, flags, lean lines) is in `form-design.md`.

------------------------------------------------

BLOCKS

Every response is one code block of paste-ready plain text containing one or more blocks (SNIPPET or NOTE). Separate blocks with a line of five equals signs:

=====

A SNIPPET block looks like this:

SNIPPET: op_approach_submandibular
PREFIX: submand
TITLE: Submandibular approach
USES: common_implant_case            (optional line; see USES)
TEXT:
<the prose, with placeholders>

<definitions: LIST / MULTILIST / OPTIONAL / BLANK / TEETH / REPEAT / PARTS / AUTO / PRESET / SECTION>

- SNIPPET: the snake_case name (see naming in oms-style.md).
- PREFIX: a short lowercase tag, unique to this snippet (e.g. submand, enuc, imp_exam). Keeps this snippet's form fields separate from other snippets in the same note.
- TITLE: a short human-readable name, plain keyboard characters ("Exam - implant consult"). Becomes the form's section heading.
- TEXT: everything after this line, up to the first definition, is the prose.

------------------------------------------------

PLACEHOLDERS (inside TEXT, or inside option text)

- LIST_NAME: an UPPERCASE_NAME with at least two words joined by underscores (so it can never be confused with abbreviations like BMP or ORIF). Every placeholder has exactly one matching definition in the same snippet, or in a snippet named on its USES line.
- ***(label): a free-text blank. The label becomes the form label. Modifiers go after a semicolon:
  - ***(label): starts as *** (the flag). Use it when the value must be filled in.
  - ***(label; optional): starts empty. Its `?` line or PARTS phrase drops out while it is empty.
  - ***(label; paragraph): a multi-line box, e.g. ***(Past medical history; paragraph, optional).
- BLANK_NAME: a named blank, defined with `BLANK: BLANK_NAME (label: Age; optional)` (same modifiers). Use it when a blank is shared through USES or appears more than once.
- @snippet_name@: embeds another snippet at this spot. It must be defined in the same response or already exist.
- **text**: bold, for note headings only (e.g. **FINDINGS:**).

------------------------------------------------

LEAN LINES: `?`

A TEXT line starting with `?` is printed only when every control on it has a value (a LIST picked, a MULTILIST with at least one pick, an OPTIONAL on, a blank non-empty, a PARTS with at least one phrase). Otherwise the whole line, including its line break, drops out:

?- Periodontal status: PERIO_STATUS
?- Adjacent teeth: ADJACENT_TEETH

The `?` itself is not printed. Use `?` lines for every objective and plan bullet.

------------------------------------------------

LIST: single choice (chips)

LIST: LATERALITY (required)
- right
- left
- bilateral

Options can be short words inserted as-is, or titled options whose title is shown in the form while the longer text goes into the note:

LIST: FACIAL_VESSELS_STATEMENT
. Facial vessels not seen
  - The facial vessels were not encountered within the operative field.
. Facial vessels protected
  - The facial vessels were identified and protected during dissection.

Do not mix short and titled options in one list. A titled list writes its flag option as:

. ***
  - ***

A LIST is optional by default: unpicked, it prints nothing. Mark it (required) only when running prose would break without it and no default is safe. A LIST used in running prose needs a (default) or (required). Clicking the picked chip again clears it.

MULTILIST: multiple choice (checkboxes)

MULTILIST(and): BONE_GRAFT_TYPES
- particulate allograft
- particulate xenograft

The word in parentheses says how picked items are joined in the note:
- and: "a, b and c"
- or: "a, b or c"
- comma: "a, b, c"
- semicolon: "a; b; and c" (use when items contain commas)
- space: "a b c" (use for titled options that are full sentences)
- newline: one per line (use for titled options that are full paragraphs or list lines)

Selected items always appear in the order they are listed, not the order they were clicked, so order the options the way they should read.

DEFAULTS

Add (default) after an option to preselect it. For a MULTILIST, several options may be marked:
- right (default)
. Facial vessels protected (default)
  - The facial vessels were identified and protected during dissection.

NESTED CHOICES

An option's text may itself contain a placeholder (another LIST/MULTILIST or a ***(label) blank). That inner control only appears in the form when the outer option is picked. Define the inner list as a normal top-level definition in the same snippet:

MULTILIST(space): OPTIONAL_ADDS
. Extraction Single
  - ... The patient elected to PROCEED_DEFER the proposed extraction ...

LIST: PROCEED_DEFER
- proceed with
- defer

Only nest one level deep.

OPTIONAL: a checkbox that includes or leaves out a sentence, line or block

OPTIONAL: SINUS_STATEMENT (default off)
  No communication with the maxillary sinus was noted.

Place SINUS_STATEMENT in TEXT where the sentence belongs. Use OPTIONAL instead of a one-option MULTILIST. Controls placed in its indented text appear in the form only while it is on. That makes it the construct for an optional add: title it "+ Name" and put the add-on's own chips inside.

OPTIONAL: EXTRAORAL_EXAM (default off; label: + Extraoral/TMJ exam)
  - Extraoral/TMJ: EXTRAORAL_FINDINGS

------------------------------------------------

TEETH: an FDI odontogram (click, drag, tooth-type and quadrant buttons)

TEETH: IMPLANT_SITES (label: Implant sites)

To chart two lists on one odontogram with a paint mode per list (a tooth is in only one), name the partner on the second definition:

TEETH: EDENTULOUS_SITES (label: Edentulous)
TEETH: PRESENT_TEETH (label: Tooth present; same chart as: EDENTULOUS_SITES)

In TEXT a TEETH placeholder reads as its teeth in FDI order, joined with "and" ("35, 36 and 37"). Suffixes give derived wording:
- NAME.spans: runs of adjacent teeth ("35-37 and 46")
- NAME.site / NAME.Site: "site" or "sites" (capitalized form for a line start)
- NAME.tooth / NAME.Tooth: "tooth" or "teeth"
- NAME.count: the number of teeth

REGIONS, for AUTO and conditions: maxillary posterior (14-18, 24-28), mandibular posterior (34-38, 44-48), esthetic zone (11-13, 21-23, 31-33, 41-43).

------------------------------------------------

REPEAT: a block the surgeon can add any number of times, or one block per charted tooth or span

REPEAT(newline): FRACTURE_SITES (label: Fracture site)
  - FRACTURE_REGION fracture, FRACTURE_DISPLACEMENT.

LIST: FRACTURE_REGION (in FRACTURE_SITES)
- symphyseal
- angle

Place FRACTURE_SITES in TEXT; each block renders its indented text once, joined like a MULTILIST. Lists and blanks used only inside the repeated text are marked (in REPEAT_NAME).

Driven by a TEETH chart, blocks appear and disappear with the chart (no Add button):

REPEAT(newline): EXAM_SPANS (label: Edentulous span; each span of EDENTULOUS_SITES)
REPEAT(newline): EXAM_TEETH (label: Tooth present; each tooth of PRESENT_TEETH)

Inside, THIS_SITE is the block's tooth ("46") or span ("35-37"); THIS_SITE.Site reads "Site" or "Sites". A `?` line inside a REPEAT drops out per block.

------------------------------------------------

PARTS: phrases joined into one line, each dropping out on its own

PARTS(semicolon): SPAN_CBCT (in CBCT_SPANS)
  - bone height BONE_HEIGHT
  - bone width BONE_WIDTH
  - VITAL_STRUCTURE PROXIMITY

A phrase prints only when every control in it has a value. The picked phrases are joined like a MULTILIST (same join words), and an empty PARTS prints nothing, so it drops its `?` line:

?- THIS_SITE.Site THIS_SITE: SPAN_CBCT

A phrase may instead carry a condition, printed when the condition holds: `- injury to adjacent teeth (when NEXT_PROCEDURE is set)`. This is how risk lists are built. PARTS is not a control, so the controls inside it don't count toward the one-level nesting limit; a PARTS inside an OPTIONAL add is fine.

------------------------------------------------

AUTO: text included automatically when a condition holds

AUTO: TOBACCO_BULLET (when TOBACCO_USE is Current, Vaping; or ticked: + Tobacco cessation)
  - Tobacco cessation counseling provided.

Place the name in TEXT like an OPTIONAL. Conditions:
- X is A, B: a LIST picked as one of the titles
- X has A, B: a MULTILIST or TEETH with any of these picked
- X is set: anything picked or typed
- X in REGION: a TEETH list with a tooth in a named region
- combine with and / or; parentheses group

`or ticked: <label>` adds a checkbox that includes the text by hand when the condition doesn't. Give every automatic risk or plan item one.

------------------------------------------------

USES: sharing facts between snippets

Facts several sections need (age, sites, referring provider) are defined once, in a common_... snippet with an empty TEXT. Another snippet names it on its USES line and then uses those placeholders as its own, in TEXT, conditions and presets:

USES: common_implant_case

A USES snippet that a note may leave out is marked optional, and conditions on its placeholders are simply false when it's absent:

USES: common_implant_case, clinic_history_implant (optional)

A NOTE built from snippets that share a common_... snippet embeds it first (it prints nothing).

------------------------------------------------

PRESET: a one-click button that fills in several controls

PRESET: Mand angle, favorable
  FRACTURE_REGION = angle
  FRACTURE_DISPLACEMENT = minimally displaced
  SINUS_STATEMENT = off
  BONE_GRAFT_TYPES = particulate allograft, BMP

Use option titles (or the short option text) as values, on/off for OPTIONAL, and `none` to clear a LIST or MULTILIST. A PRESET shows in the form section it is defined under (the TITLE, or the SECTION before it). Mark (in REPEAT_NAME) to put it inside every block of a REPEAT, setting that block only:

PRESET: All adequate (in CBCT_SPANS)
  BONE_HEIGHT = adequate
  BONE_WIDTH = adequate

A pathway preset sets its whole combination and clears what it doesn't use (`= none`).

A preset outside a REPEAT can also set a list inside one, in every block the REPEAT has when it is clicked: write the REPEAT's name, a dot, then the list. Blocks driven by a TEETH chart must be charted first.

PRESET: GBR - lateral aug - staged
  NEXT_PROCEDURE = GBR
  EXAM_SPANS.RIDGE_FORM = Width deficient
  CBCT_SPANS.BONE_WIDTH = inadequate

SECTION: starts a new form heading for the definitions after it (e.g. separate "Past medical history", "Medications" and "Allergies" headings). REPEATs and PRESETs follow the section they are defined in.

------------------------------------------------

NOTE: a full note assembled from snippets

NOTE: op_note_mandible_orif
NAME: Op Note - Mandible ORIF
TEXT:
**INDICATIONS:**
@op_indications_mandible_fracture@

**FINDINGS:**
@op_findings_mandible_fracture@

**PROCEDURE IN DETAIL:**
@op_approach_submandibular@
@op_fixation_...@
@op_closure_...@

A NOTE may also contain its own prose, placeholders and definitions (with a PREFIX: line), but keep it mostly a list of embedded snippets.

A PRESET defined in a NOTE sits in the row at the top of the form. Use it for a pathway preset that spans several snippets; it may set any list of the snippets the NOTE embeds, including lists inside their REPEATs (REPEAT_NAME.LIST).

------------------------------------------------

OUTPUT STYLE

All output is paste-ready plain text in a single code block, in plain keyboard characters.

When creating snippets:
- Provide the SNIPPET header lines, TEXT, and all definitions
- Avoid unnecessary explanation unless requested

When a snippet reuses a small recurring piece (local anesthesia infiltration, irrigation and hemostasis, standard risks), make it its own common_... snippet and embed it with @name@, so it is written once.
