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
  soft tissues WNL, opening adequate, face symmetric; implant consult: opposing dentition
  "restored natural dentition", occlusion "stable and reproducible"). Exception: the implant
  smile line prints only when stated.
- **Referring provider**: print the line only when someone is explicitly stated to have referred
  the patient; a provider who is merely mentioned doesn't count.
- **PMH, medications, allergies**: print "***" when not discussed. PMH lists only conditions that
  are present, never normal findings or denied conditions ("kidney function okay"). A healthy patient gets "No
  chronic medical conditions" / "None" / "NKDA". PMH and medications are "- " bullets, one item
  per line, with short detail only when it bears on the visit (e.g. migraine neurologic symptoms,
  "started <1 month ago, tolerating well"). Every medication and supplement mentioned is listed.
- **Risk factor chips** print "no" when the history was discussed and the factor never came up,
  and "***" when the history wasn't discussed at all.
- **PSH and Social History**: list only what is present, with stated details. Otherwise print
  "Reviewed and updated in chart as applicable."
- **Risks paragraph**: any mention of a risk, complication or consent topic means the full
  discussion took place. With none, print "Risks, benefits, and alternatives: ***".
- **Plan**: the consent-at-surgery bullet is a default when a procedure is planned. Print nothing
  for a consult letter or "Schedule surgery". Anesthesia ends the next procedure line.
- **Follow-up**: when the form has a pathway default (TMJ: 6-8 weeks for conservative care, after
  MRI when one is ordered), use it when no interval was stated. When it has none (third molar),
  print follow-up only when stated.
- **Measurements**: print them only when stated. If the surgeon is heard measuring and the value
  isn't captured, print the label with "***" for the value.
- **Medications** named without a dose take the form's standard regimen text. Misheard drug and
  product names are corrected to the real name from context ("CortCap" = Cortef, "Serticel" =
  Surgicel). A drug that can't be identified confidently prints its class with " (***)", e.g.
  "- Seizure medication (***)".
- **Pathway presets** supply the defaults for unstated choices. For example, the alternatives
  list follows the main diagnosis, and a splint's fabricator defaults to the referring dentist.
- **Diagnoses** the surgeon doesn't name are inferred conservatively, from clear findings only,
  with "***" where the side or the diagnosis is unclear.
- **Standard counseling paragraphs** (e.g. TMJ conservative care, escalation) are inserted when
  the plan implies them and any part of them came up.
- **Every consult** gets PSH, Social History and the sign-off, even when its Templater note lacks
  them.
- **Sedation evaluation**: when IV sedation or GA is planned, infer ASA from the history, and
  default the airway to "Mallampati I; neck full range of motion".
- **Vital structures**: default to "in close proximity". Print "not in close proximity" only on a
  clear statement.
- **Incidental findings**: keep them as short descriptive clinical phrases, with any judgment
  dropped. Stated qualifiers stay ("very mild flattening"), and missing teeth are listed in FDI.
- **Objective means examined**: the exam records only what the surgeon examined or measured. A
  complaint the patient reports (e.g. "my bite has shifted") goes in the HPI.
- **Extra context**: allow a little, only when it was said: at most 2 sentences in the HPI, 1 in
  the assessment and 1 plan bullet. HPI extras cover the prior treatment course only (when it was
  done, how long it helped, why it stopped), never prior providers or life events.
- **Alternatives** list only what was actually offered, defaulting to "observation and no
  treatment". A procedure mentioned only as a future escalation isn't an alternative.
- **Workup-first assessments**: when imaging is ordered to clarify the diagnosis, the assessment
  gives the findings that prompted it ("Given ..., MRI was recommended ..."). Interim
  management follows ("... while awaiting imaging").
- **Social History** holds only its four items (tobacco, heavy alcohol, cannabis, recreational
  drugs).
- **Decision**: surgery being scheduled implies "wishes to proceed"; otherwise print "***". The
  decision sentence is its own paragraph, after a blank line.
- **Vitals**: whenever vitals come up, they are the first exam line, brief and ending "(see chart
  for details)". A stated interpretation ("likely situational") is allowed on that line only.
- **Other extraoral findings** are appended to the Extraoral/TMJ line after a semicolon.
- **Plan extras**: the one allowed extra plan bullet must concern the planned procedure or its
  prep, never other providers' care of other teeth or conditions.
- **Anticoagulants**: when a perioperative plan was stated, the bullet states it ("Apixaban 5 mg
  BID to continue perioperatively per ...; local hemostatic measures planned (gelfoam/Surgicel)").
  Otherwise keep the "to be coordinated with the prescribing provider" wording. Hemostatic agents
  never get their own bullet.
- **Periapical-only imaging**: the "not close" wording becomes "roots do not appear to be in close
  proximity to the ..."; with a panoramic radiograph or CBCT it stays "roots not in close
  proximity to the ...".
- **Extraction consult only**: "- No evidence of acute infection" follows the tooth lines unless
  signs of infection were found. After the risk list and a blank line, the recovery sentence
  prints verbatim whenever risks were discussed ("We discussed procedural details and expected
  recovery with 2-3 days initial discomfort, up to 1-2 weeks lingering soreness."), followed by
  up to 2 sentences of case-specific counseling that was actually said.
- **Per-tooth exam conditions**: when a tooth's condition wasn't described, print "Tooth XX: ***".
  Never carry over a Templater preselected condition (e.g. the extraction form's "grossly
  carious").
- **Per-tooth imaging**: print the normal lesion status by default ("no periapical pathology",
  "no pericoronal pathology"), but bone level only when mentioned.
- **Pathway plan defaults**: a pathway's follow-up interval, chlorhexidine and urgency lines apply
  when unstated, but an antibiotic prints only when it was mentioned.
- **Anesthesia default**: the extraction and implant consults default to "local anesthesia" when
  not stated; the third molar consult prints "***".
- **Pathology impression**: the level of concern and the differential are both inferred
  conservatively from clear findings when the surgeon doesn't state them, with "***" when there is
  no basis. A concerning level always brings squamous cell carcinoma or malignancy into the
  differential.
- **Lesion size** is the exception to the measurement rule: every lesion line carries a size, and
  an unstated one prints "*** mm".
- **Same-day procedure details**: unstated choices take the form defaults (anesthetic agent and
  route, hemostasis, suture); numbers such as the anesthetic volume, and the pathology lab, print
  "***" unless stated.
- **No imaging**: when every lesion is soft tissue, print the form's "No imaging indicated" line;
  when there is a lesion in bone and no read was given, print "- ***".
- **Per-site implant lines** (edentulous span exam and CBCT): print only the descriptors stated;
  a site with none prints "Site XX: ***". Never fill in "adequate" (ridge form, keratinized
  tissue, bone height or width, restorative space).
- **Judgments stay in the assessment**: "nonrestorable" never prints in an exam line, even where
  the Templater form offers it there (extraction and implant consults).
- **Implant CBCT normal lines** are region-aware: "Maxillary sinuses clear..." by default only
  when a planned site is in the maxilla and no sinus disease was read; "No other pathology..."
  by default; the condyles and adjacent-teeth periapical lines only when stated. The header is
  "Images reviewed: " with the images, defaulting to "CBCT" when the type is unclear.
- **Implant HPI goals** default to "a fixed replacement and improved chewing function" when goals
  weren't discussed.
- **Implant interim prosthesis** prints only when an interim replacement was discussed.
- **Tooth present vs missing** (implant consult): a tooth still in the mouth, even fractured to the
  gum line or a retained root, is a tooth present ("Tooth XX:" lines), never an edentulous site.
  "Was lost" is said only of teeth already gone.
- **Tooth-present implant lines**: the HPI gives the tooth's history (prior RCT, how and when it
  fractured) in at most 2 sentences, then "No current pain or other symptoms." unless symptoms
  were mentioned; the prosthesis sentence is for edentulous sites only. Goals and the referral
  request form a second HPI paragraph. The exam line uses the closed terms with the stated
  extent ("fractured to gingival margin; retained root remains"). The CBCT line leads with the
  structural finding, and bone and space descriptors print only when the read states them.
- **Endodontically failed** means the root canal itself failed (persistent infection or a
  periapical lesion); a root-filled tooth that fractured is just "fractured".
- **Anesthesia history** (PSH, every consult): append only named complications: PONV, difficult
  airway, malignant hyperthermia, prolonged emergence, emergence delirium, family history of
  anesthesia reactions. Vague reactions ("felt strongly sedated") are dropped.
- **Head and neck radiation outside the jaws**: when the surgeon states the oral cavity was
  outside the field, the risk factor prints "no (oral cavity not in the field)" and
  osteoradionecrosis drops out; when unclear, it stays "yes, ..." with the ORN risk.
- **Soft tissue grafting** (implant consult): raised as a general possibility, it is one
  assessment sentence; the plan bullet prints only when it was specifically recommended for the
  site.
- **Surgery-visit prescriptions** (implant consult): antibiotics and chlorhexidine never print in
  the consult plan; they belong to the surgery note.
- **Implant follow-up** defaults to "2-3 weeks post-operatively" whenever a next-visit procedure
  is planned, with the provider only when named.
