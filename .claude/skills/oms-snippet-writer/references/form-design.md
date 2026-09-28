# Templater form design

The surgeon's house style for every Templater form. The goal: a thorough, defensible note in the
fewest clicks and the least typing. Three leading words run through it:

- **one-click**: every choice is visible and takes a single click.
- **lean**: the default note carries only what was picked; nothing prints for an untouched control.
- **flag**: `***`, the surgeon's deliberate "fill this in later" marker, always a choice and never
  the result of leaving something blank.

The reference implementation is the implant consult: `my_templates/partials/common_implant_case.toml`
and `clinic_*_implant.toml`, with the note in `my_templates/clinic_note_implant_consult.toml`.
When in doubt, match it.

## Controls

- **One-click controls.** A LIST with 10 or fewer options shows as chips, and a MULTILIST or
  OPTIONAL as checkboxes. Keep option titles short (24 characters or fewer) so a MULTILIST sits on
  one row. Split a longer list, or chart it.
- **Tooth sites come from a TEETH chart**, never typed. When a note covers both edentulous sites
  and teeth still present, chart both on one odontogram, then describe spans and teeth separately.
- **Free text** is only for names, numbers and narrative. A blank that can be skipped is
  `optional`, so it starts empty and its line or phrase drops out.

## Defaults

- **Preselect the normal or common finding.** Examples: oral hygiene good, occlusion stable,
  parafunction none, opposing natural dentition, CBCT normal findings ticked, every risk factor
  "no", decision proceed, and the usual plan choices (local anesthesia, amoxicillin preop,
  chlorhexidine, schedule surgery).
- **Leave a finding unpicked when it needs an active look**, so choosing it is a deliberate act.
  Examples: periodontal status, adjacent teeth, bone adequacy on CBCT, and allergies (give them an
  NKDA speed button instead of a default).
- **Nothing is required.** Use `(required)` only where an empty choice would break running prose and
  no default is safe, for example laterality inside an op-note sentence.

## Lean output

- **Objective and plan content is built from `?` lines and PARTS phrases**, so each line or phrase
  drops out until something is picked.
- **Every LIST whose line can drop out ends with a `***` option**, the flag.
- **Expanded-note content goes behind an optional add**: an OPTIONAL titled "+ Name", default off,
  whose controls appear only when it is ticked. Examples: extraoral/TMJ exam, additional findings,
  referral request, interim prosthesis, periodontal history.

## Enter once, derive the rest

- **Facts that more than one section needs live in one `common_…` snippet.** Examples: age, sites
  and referring provider. Other snippets reach them with USES.
- **Derive dependent wording instead of asking for it**:
  - tooth/teeth and site/sites grammar;
  - spans ("35-37") and sentence lists ("35, 36 and 37");
  - regions: charted mandibular posterior, maxillary posterior or esthetic-zone sites bring in the
    matching nerve, sinus or esthetic content automatically;
  - history: risk factors bring in their risk phrase and plan bullet automatically.

  Keep a manual checkbox beside every automatic item so it can still be added by hand.
- **Use the referring provider's name** wherever the note refers to them, falling back to "the
  referring dentist". The "Referring provider:" line is omitted when the name is blank.

## Speed buttons

- **A PRESET sits in the section it fills.** Examples:
  - "Healthy: noncontributory / none / NKDA" under PMH;
  - "Healthy perio, sound adjacent teeth" under the exam;
  - "Limited FOV - mandible" under CBCT;
  - "All adequate" inside each CBCT box.
- **Pathway presets set a whole combination**: next procedure, descriptors, rationale and sequence.
  They clear what they don't use (`= none`), so clicking one preset after another never leaves
  stale picks.

## Clinic note content

- **Header.** "Reason for visit:" (a text field with a sensible default), then "Referring
  provider:" when given.
- **HPI.** Concise. It opens with a one-liner, "58-year-old patient presenting for implant evaluation
  of edentulous sites 35-37 and tooth 46.", followed by short sentences: how and when teeth were
  lost, current prosthesis, patient goals.
- **PMH, Medications, Allergies.** Each has its own bold header and a paragraph input, with section
  speed buttons ("Noncontributory", "None", "NKDA").
- **Risk factors.** Labelled bullets whose chips default to "no". Rarer items, such as periodontal
  history, are optional adds.
- **Exam and imaging.**
  - Labelled bullets, with one box per edentulous span and one per tooth present.
  - The conditions picked for a tooth present (heavily restored, fractured, carious, mobile, and
    so on) describe that tooth throughout the note. "nonrestorable" is the fallback.
  - CBCT uses adequate / marginal / inadequate scales for bone height, bone width, vertical and
    mesiodistal restorative space.
  - A vital structure is printed only once picked, as "in close proximity" or "not in close
    proximity".
- **Assessment.**
  - It opens "58-year-old patient presenting with [the specific problem in clinical terms]", e.g.
    "partial edentulism at sites 35-37 and fractured, carious tooth 46 with horizontal alveolar
    ridge deficiency".
  - The recommendation follows, with one short rationale sentence per adjunct.
  - Alternatives go in one sentence.
- **Risks.** Concise and medicolegally sound, in two bullets:
  - "General surgical:" one line;
  - "Specific to the planned procedure:" only the risks of the next procedure's components, of the
    charted sites, and of the patient's risk factors, separated by semicolons.

  Close with the understanding and questions-answered sentence.
- **Plan.** Summarized bullets, each dropping out until picked. "Next procedure" chips (extraction,
  socket preservation, implant placement, GBR, sinus lift, and so on) write the procedure bullet
  with its sites, and drive the specific risks. The overall sequence is one short line.

## Text

- **FDI notation:** "tooth 36", "site 46", "sites 35-37". No "#".
- **Plain keyboard characters:** "-" for dashes, straight quotes, ">=" rather than the symbol.
