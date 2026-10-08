# Converting a Templater note

## Template layout

1. **Global blocks**, each its own round-bracket paragraph, in this order (copy them from the
   reference template and adapt them to the note):
   - GLOBAL INSTRUCTIONS: specialty and note type, follow structure and wording exactly, concise
     style, "the patient" in place of the name, plain keyboard characters, bullets as "- " lines.
   - CLINICAL TERMINOLOGY: the surgeon explains findings in plain English, so the note translates
     them back. Give a mapping of the lay phrases that note's findings use.
   - TOOTH NUMBERING: FDI only; how to convert spoken positions, Universal numbers and "8s".
   - TRANSCRIPT QUALITY: infer meaning from a poor transcript; never invent; what `***` means.
   - RADIOGRAPH READ: a dictated or contextual-note read overrides in-room talk for imaging, and
     the later read wins.
   - OBJECTIVE SECTIONS: exam and imaging hold descriptions only, never judgments.
   - WORDING TOKENS: define AGE, the sites token (e.g. TEETH), DECISION-MAKER and any other
     derived phrase the Templater `common_...` snippet computes.
2. **Sections**: one heading per Templater section, in note order. Each section gets one
   placeholder plus an instruction, or several when one section has parts with different gap
   policies (assessment, alternatives, risks, decision).
3. **Sign-off**: the last line, verbatim: `"Thomas Beckstrom"`.

## Construct mapping

| Templater | Heidi instruction |
|---|---|
| LIST option texts | "print exactly one of: ..." with each text quoted, plus "Default to ..." when the form has a default |
| MULTILIST | "followed by ... joined with commas and "and", using these terms where they fit: ..." |
| `?` line / lean line | "only if mentioned, else omit this line" |
| field default | "Default to "..." if not mentioned" (only where the gap policy keeps defaults) |
| `***` flag | "print "***"" for the gap the surgeon must fill |
| AUTO / `when` / region flags | "If CONDITION, print verbatim: "..."", one per line, conditions spelled out (e.g. "if tooth 38 or 48 is being treated") |
| risk factor that drives a risk and a plan bullet | state the condition at both places, worded the same way |
| checklist PARTS (risks) | a numbered strict checklist: "decide yes or no for each against the PLAN and history; include every yes; never skip one whose condition is met" |
| computed wording (tooth/teeth, lists, decision-maker phrase) | a WORDING TOKEN defined once in the global block |
| REPEAT per site | "one line for every tooth in TEETH: "- Tooth XX: ..."" |
| PRESET / pathway | write out the outcome as conditions; Heidi has no buttons |
| paragraph breaks in the body | "as a separate paragraph after a blank line" |

## Standing rules (surgeon decisions)

Apply these without asking; add new ones here as they are settled.

- **Exam lines**: use the Templater form defaults when they weren't mentioned (oral hygiene good,
  soft tissues WNL, opening adequate, face symmetric).
- **PMH, medications, allergies**: print "***" when not discussed. A healthy patient gets "No
  chronic medical conditions" / "None" / "NKDA".
- **Risk factor chips** print "no" when the history was discussed and the factor never came up,
  and "***" when the history wasn't discussed at all.
- **PSH and Social History**: list only what is present, with stated details. Otherwise print
  "Reviewed and updated in chart as applicable."
- **Risks paragraph**: any mention of a risk, complication or consent topic means the full
  discussion took place. With none, print "Risks, benefits, and alternatives: ***".
- **Plan**: the consent-at-surgery bullet is a default when a procedure is planned. Print nothing
  for a consult letter or "Schedule surgery", and follow-up only when stated. Anesthesia ends the
  next procedure line.
- **Sedation evaluation**: when IV sedation or GA is planned, infer ASA from the history, and
  default the airway to "Mallampati I; neck full range of motion".
- **Vital structures**: default to "in close proximity". Print "not in close proximity" only on a
  clear statement.
- **Incidental findings**: keep them as short descriptive clinical phrases, with any judgment
  dropped.
- **Extra context**: allow a little, only when it was said: at most 2 sentences in the HPI, 1 in
  the assessment and 1 plan bullet.
- **Decision**: surgery being scheduled implies "wishes to proceed"; otherwise print "***".
