---
name: templater-to-heidi
description: Convert a Templater note (my_templates/*.toml) into a Heidi Health AI-scribe template in heidi_templates/, and iterate on it from Heidi output. Use when asked to make or revise a Heidi template, or when the user pastes Heidi output beside their corrected note.
---

# Templater to Heidi

A Heidi template is a set of instructions an LLM follows against an imperfect transcript. The
goal is **parity**: Heidi's note reads like the Templater note the surgeon would have produced
for the same visit, with the same sections, order, wording, defaults and derived content.
Templater's form logic (lists, AUTO conditions, presets, computed fields) becomes written
instructions Heidi can follow without a form.

The reference implementation is `heidi_templates/third_molar_consult.txt`, converted from
`my_templates/clinic_note_third_molar_consult.toml`. Match its shape.

## Steps

1. **Read the references in full**:
   - [`references/heidi-syntax.md`](references/heidi-syntax.md): Heidi's template syntax and
     its limits.
   - [`references/conversion.md`](references/conversion.md): the construct-by-construct mapping
     and the surgeon's standing rules for Heidi.

2. **Read the source note completely.** Read the NOTE `.toml` and every partial in its
   `use_partials`, including partials those partials include. Render it with
   `cargo run -q --bin check_templates -- my_templates <template_id>`. Done when every field,
   option text, AUTO condition, computed field and preset is accounted for, and you know which
   output line each one produces.

3. **Settle the gap policy.** For each section, decide what Heidi prints when the transcript is
   silent: the form default, `***`, omit, or a fixed fallback. The standing rules in
   conversion.md already decide most of them. Put only the open ones to the surgeon with
   AskUserQuestion: at most 4 per call, recommended option first, successive calls if needed.
   Done when every section has a policy.

4. **Write the template** to `heidi_templates/<note_name>.txt`, following conversion.md: the
   global blocks first, then one heading per Templater section in note order.

5. **Lint it.** Done when every item holds:
   - every line the rendered Templater note can produce has a counterpart instruction, with its
     wording verbatim in quotes;
   - every Templater condition (AUTO, region flags, risk factor links, sedation) appears as an
     explicit "if ..., print ..." with its condition spelled out;
   - every placeholder is followed by its instruction, and no instruction contains square
     brackets;
   - every section has its gap policy written out;
   - the text is plain ASCII: `LC_ALL=C grep -nP '[^\x00-\x7F]' <file>` prints nothing;
   - tooth numbers are FDI, and the note ends with the sign-off.

6. **Hand off.** Tell the surgeon to paste the file into Heidi's template editor (Structure
   tab), run real transcripts through it, and paste back Heidi's output with their corrected
   note.

## Iterating on Heidi output

When the surgeon pastes Heidi output beside their corrected note:

1. **Diff line by line**, sorting each difference into one of two groups:
   - a **fix**: Heidi broke an instruction already in the template (dropped a token, skipped a
     condition that was met, lost formatting). Strengthen the instruction; no question needed.
   - a **decision**: the corrected note shows a rule the template doesn't have yet. Ask about
     the general rule behind it, never just the one instance.
2. **Grill the decisions** in rounds with AskUserQuestion, recommended option first. Each answer
   can open follow-ups; ask them in the next round. Done when no difference is unexplained.
3. **Confirm** with one summary of fixes and decisions, and wait for the surgeon's go.
4. **Edit the template**, then re-run the step 5 lint.
5. **Propagate.** Add each new standing rule to conversion.md. When a decision is a general
   preference rather than a Heidi quirk, offer to apply it to the Templater note and to the
   `oms-snippet-writer` skill's `form-design.md` as well.
