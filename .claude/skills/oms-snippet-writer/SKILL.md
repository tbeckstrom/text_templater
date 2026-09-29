---
name: oms-snippet-writer
description: Write oral & maxillofacial surgery (OMS) Templater snippets, notes and clinic forms - op note sections (indications, findings, approach, fixation, closure), clinic consult sections (HPI, history, exam, imaging, assessment/plan, risks), modifier-22 language - in the Templater snippet language. Use when asked for a snippet, note section, template or form for OMS documentation, or to revise one.
---

# OMS snippet writer

Produce paste-ready snippet-language text (SNIPPET / NOTE blocks) for OMS documentation. The
text is converted into Templater forms, so it must be clinically sound, fast to fill in, *and*
syntactically exact.

## Steps

1. **Read the references in full** before writing, every time:
   - [`references/form-design.md`](references/form-design.md): the surgeon's house style for forms
     (one-click controls, defaults, lean lines, flags, presets) and what clinic notes contain.
   - [`references/oms-style.md`](references/oms-style.md): clinical voice, naming categories,
     standard list wording, op-note content rules.
   - [`references/snippet-language.md`](references/snippet-language.md): the syntax.
   - [`references/examples.md`](references/examples.md): the shape and density to match.

2. **Check what already exists.** If a `my_templates/partials/` folder is present (working in the
   Templater repo), list it:
   - Embed an existing `common_...` or other snippet with `@name@`, or share its facts with USES,
     instead of rewriting it.
   - A revision of an existing snippet keeps its SNIPPET name and PREFIX. Read the current `.toml`
     first and preserve the surgeon's wording, changing only what was asked. Point out older
     wording that breaks form-design.md (e.g. "tooth #" instead of FDI) rather than silently
     changing it.
   - Pick a PREFIX not already used by another file (each file's first line records it).

   Without that folder (e.g. in a claude.ai chat), rely on what the user tells you exists.

3. **Interview before a new note or clinic form.** For a whole new note, or a section whose
   structure isn't settled, interview the surgeon in rounds with the AskUserQuestion tool before
   drafting: sections and scope, which facts are shared, per-site structure, defaults, optional
   adds, presets. Each question carries your recommended option first. The step is done when every
   structural choice is either answered or covered by a default in form-design.md, and the surgeon
   has confirmed the summary. A single snippet with an obvious shape skips this step.

4. **Write the snippet(s)** as one code block, following the references. When generalizing text,
   remove only unnecessary specificity while keeping realistic surgical detail.

5. **Lint your own output** before showing it. Done means every item holds:
   - every UPPERCASE_NAME placeholder has exactly one definition in its block or in a snippet on its
     USES line, and every definition is used (in TEXT, a condition, a preset, or by a USES-ing
     snippet);
   - every `@name@` and USES name is defined in this response or exists in `my_templates/partials/`;
   - no list mixes short and titled options; nesting is at most one level deep (PARTS doesn't count);
   - every LIST in running prose has a (default) or (required); every other LIST sits on a `?`
     line or in a PARTS phrase and ends with a `***` flag option;
   - every objective and plan bullet is a `?` line or a PARTS phrase;
   - every `***` blank carries a `(label)`, and skippable blanks are `optional`;
   - each MULTILIST's and PARTS's join word matches how its items read in the sentence;
   - PRESET values name real options of the lists they set; pathway presets clear what they don't
     use (`= none`);
   - a list covered out loud (the risks) is a `checklist` PARTS, and a NOTE whose sections are
     read at the desk (imaging) names them on a PRINT SKIP line;
   - tooth numbers are FDI without "#", and the text uses plain keyboard characters.

   Fix anything that fails, then re-check.

6. **Hand off.**
   - In the Templater repo: show the code block and ask the surgeon to review it clinically. Once
     they approve (or after applying their edits), convert it with the `snippet-to-toml` skill.
   - Anywhere else: the code block is the deliverable; the surgeon pastes it into the Templater
     repo later.
