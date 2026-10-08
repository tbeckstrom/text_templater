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
  opposing restored natural dentition, face symmetric, opening adequate, a charted tooth
  nonrestorable, CBCT sinuses clear, every risk factor "no", healing and integration times 4-6
  months, decision "considering", and local anesthesia for the next visit.
- **Sedation evaluation defaults to normal, shown only when sedation is planned.** When the next
  visit is under IV sedation or general anesthesia, the ASA class and airway lines print with
  "ASA class: I" and "Airway: Mallampati I; neck full range of motion" preselected. They stay
  hidden for local anesthesia.
- **Leave a finding unpicked when it needs an active look**, so choosing it is a deliberate act.
  Examples: periodontal status, adjacent teeth, parafunction, lymph nodes, TMJ, deviation,
  muscles, tissue phenotype, current prosthesis, bone adequacy on CBCT, and allergies (give them an
  NKDA speed button instead of a default).
- **Leave optional plan items unpicked**, such as antibiotics and chlorhexidine.
- **No workflow bullets the surgeon would not write**: no "Consult note sent to the referring
  provider", no "Schedule surgery", and no default follow-up interval (it varies, so it is an
  unpicked choice). Urgent scheduling and scheduling after medical clearance stay as picks.
- **Nothing is required.** Use `(required)` only where an empty choice would break running prose and
  no default is safe, for example laterality inside an op-note sentence.

## Lean output

- **Objective and plan content is built from `?` lines and PARTS phrases**, so each line or phrase
  drops out until something is picked.
- **Every LIST whose line can drop out ends with a `***` option**, the flag.
- **Expanded-note content goes behind an optional add**: an OPTIONAL titled "+ Name", default off,
  whose controls appear only when it is ticked. Examples: additional findings, referral request,
  interim prosthesis, periodontal history. An add used in nearly every note may default on, like
  the extraoral/TMJ exam: it defaults on in every clinic consult, with "face symmetric without
  swelling" preselected and nodes and TMJ unpicked.
- **Every charted site gets its exam line.** In an extraction consult each tooth to extract prints
  its own "Tooth XX:" line with its status (e.g. "not visible"), even when it has no other
  finding. Each tooth box also takes an optional short free-text finding for incidentals, written
  in clinical terms, e.g. "gingival operculum noted distal to second molar, does not appear
  inflamed".

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
  - "Healthy: no chronic medical conditions / none / NKDA" under PMH;
  - "Healthy perio, sound adjacent teeth" under the exam;
  - "Limited FOV - mandible" under CBCT;
  - "All adequate" inside each CBCT box.
- **Pathway presets set a whole combination**: next visit, descriptors, rationale and sequence.
  They clear what they don't use (`= none`), so clicking one preset after another never leaves
  stale picks.
- **Pathway presets that span sections sit in the top row of the note form**, defined in the NOTE.
  They fill exam and CBCT boxes as well as the assessment and plan. Examples: "Implant
  appropriate", "Exo - bone graft", "GBR - lateral aug - staged". They set every span or tooth box
  that exists when clicked, so the sites are charted first.

## Clinic note content

- **Header.** "Reason for visit:" (a text field with a sensible default), then "Referring
  provider:" when given.
- **HPI.** Concise. It opens with a one-liner, "58-year-old patient presenting for implant evaluation
  of edentulous sites 35-37 and tooth 46.", followed by short sentences: how and when teeth were
  lost, current prosthesis, patient goals. Current symptoms read "Patient reports no symptoms
  currently." or "Patient reports pain and swelling.". Pertinent negatives go in their own
  paragraph after a blank line: "No hx of facial swelling, food trapping, or pain associated with
  these sites." (a MULTILIST of the denied items).
- **PMH, Medications, Allergies.** Each has its own bold header and a paragraph input, with section
  speed buttons ("No chronic medical conditions", "None", "NKDA").
- **Past surgical and social history** (third molar consult; `clinic_history_surgical` and
  `clinic_history_social`, which the extraction history prints only when the note uses them).
  The order is PMH, PSH, Medications, Allergies, Social History, then risk factors.
  - PSH is a comma list of procedures, with prior anesthesia problems (PONV, difficult airway,
    malignant hyperthermia, family history of anesthesia reactions) appended as "history of ...".
  - Social history prints a bullet only for what is present, with stated details: tobacco
    (status from the risk factor chip, plus details), heavy alcohol, cannabis (frequency, route)
    and recreational drugs. These are documented only, without counseling, except cannabis.
  - Either section with nothing present prints "Reviewed and updated in chart as applicable."
- **Risk factors.** Labelled bullets whose chips default to "no". Rarer items, such as periodontal
  history, are optional adds. Tobacco stays here as well as in social history, because it drives
  the dry socket risk and the cessation bullet. Cannabis belongs in social history, not here; when
  it is used and sedation is planned it adds an assessment sentence ("Cannabis use was noted; the
  patient was counseled that regular cannabis use may reduce the effectiveness of sedation and
  increase anesthetic requirements, and abstinence for at least 2 weeks before surgery was
  recommended.") and the plan bullet "Advised to abstain from cannabis for at least 2 weeks
  before sedation."
- **Adjacent teeth in a third molar consult** default to a probing line, "PD WNL at distal sites
  of 17, 27, 37 and 47" (the second molars next to the charted teeth, derived). A site picked as
  deep moves to "PD 5+ mm at distal of 37". This overrides the "adjacent teeth unpicked" default
  above, which still applies to implant consults.
- **Exam and imaging.**
  - Objective only: descriptive findings, never judgments such as "not concerning" or
    "benign-appearing". The interpretation belongs in the assessment, if anywhere.
  - Labelled bullets, with one box per edentulous span and one per tooth present.
  - The conditions picked for a tooth present (heavily restored, fractured, carious, mobile, and
    so on) describe that tooth throughout the note. "nonrestorable" is the fallback.
  - CBCT uses adequate / marginal / inadequate scales for bone height, bone width, vertical and
    mesiodistal restorative space, plus an "mm" chip that opens a box for the measurement.
  - A vital structure is printed only once picked. Its proximity defaults to "in close
    proximity", with "not in close proximity" one click away. Exception: in a third molar
    consult the vital structure is derived from the tooth (18 and 28 the maxillary sinus, 38 and
    48 the inferior alveolar canal) and always prints, defaulting to "in close proximity".
  - A finding shared by every tooth (e.g. incomplete root development) prints once as a summary
    line, "Incomplete root development of teeth 18, 28, 38 and 48."; otherwise it goes on each
    affected tooth's line.
  - Sinus pneumatization and sinus membrane thickening are right / left / bilateral chips,
    unpicked by default.
- **Assessment.**
  - It opens "58-year-old patient presenting with [the specific problem in clinical terms]", e.g.
    "partial edentulism at sites 35-37 and fractured, carious tooth 46 with horizontal alveolar
    ridge deficiency". This problem statement stands alone as its own paragraph.
  - Derive problems that follow from the findings, e.g. "insufficient space for eruption"
    whenever a third molar is charted as partial or complete bony impaction.
  - The recommendation follows, with one short rationale sentence per adjunct.
  - Alternatives go in one sentence.
  - Before the risks, one line: "Treatment timing and sequencing were discussed as outlined in the
    plan below."
- **Risks.** Concise and medicolegally sound, in two bullets:
  - "General surgical:" one line;
  - "Specific to the planned procedure:" only the risks of the next procedure's components, of the
    charted sites, and of the patient's risk factors, separated by semicolons.

  Close, after a blank line, with the understanding and questions-answered sentence, followed
  directly by the shared decision-making sentence.
- **Plan.** Summarized bullets, each dropping out until picked, in this order:
  - **Next visit.** Chips (extraction, socket preservation, implant placement, GBR, sinus lift, and
    so on) write the procedure with its sites, followed by the anesthesia ("under local
    anesthesia"; "under IV sedation; NPO and escort instructions reviewed"). The anesthesia is
    part of this line, never a separate "Anesthesia:" bullet. They drive the specific risks.
  - **To do prior to next visit.** An indented sub-list of information to gather and prep:
    pickable items (medical records, medical consult, labs, and so on) plus automatic ones
    (surgical guide when the next visit places an implant, A1c for diabetes, anticoagulation,
    sedation prep).
  - **Overall sequence.** A numbered sub-list, one step per line, with each healing period on its
    own line. A staged pathway adds a re-evaluation step before implant placement (repeat CBCT
    and intraoral scan; fabricate the guide if the site is appropriate). The last step names the
    restoring dentist.
  - Then optional lines such as possible soft tissue grafting, antibiotics, interim prosthesis
    and consent.

- **Sign-off.** Every NOTE ends with the surgeon's name, "Thomas Beckstrom", on its own line
  after the plan.

## Printed sheet

The web app prints any note as a paper form the surgeon fills in by hand in the room and enters
afterwards. The sheet mirrors the form (same sections, labels and order), so every form rule above
is also a sheet rule. On top of them:

- **Defaults do the work on paper too.** A preselected normal finding prints bold and unmarked,
  "assumed unless marked", so the surgeon inks only the exceptions. Good defaults mean less ink.
- **Leave desk-only sections off.** Imaging read from the CBCT at the desk goes in the NOTE's
  PRINT SKIP, so the sheet holds what happens in the room: history, exam and the discussion.
- **Build talking points once, as a checklist.** A list the surgeon covers out loud, such as the
  risks, is a PARTS marked `checklist`. The note joins it into its sentence, and the sheet prints
  each item as its own tick-box, so both read the same list. The risks snippet has two: the
  general line and the specific risks.
- **Fit one sheet, front and back.** Give a long form label a short `print label` rather than
  shortening the form. Trim a paragraph blank with `print lines` when it rarely needs three lines,
  and use `print: no` for anything never filled in by hand.

The implant consult is the reference: it prints on one sheet with the CBCT skipped.

## Text

- **FDI notation:** "tooth 36", "site 46", "sites 35-37". No "#".
- **Plain keyboard characters:** "-" for dashes, straight quotes, ">=" rather than the symbol.
