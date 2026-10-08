# Heidi template syntax

From Heidi Health's Template Academy
(https://support.heidihealth.com/en/collections/10339282-heidi-health-s-template-academy). Re-read
it if Heidi's behaviour seems to have changed.

## Building blocks

- **Section heading**: plain text, for example `HPI:`. Heidi formats headings itself, so write no
  `**` markers.
- **Placeholder** `[...]`: names the content that goes there, for example `[Past medical history]`.
  It holds a description, never examples.
- **AI instruction** `(...)`: tells Heidi how to handle the content. It sits directly after its
  placeholder.
- **Verbatim text** `"..."`: printed word for word. Inside an instruction, quoted text is the
  exact wording to print.

## Rules

- Keep each bracket type separate: no square brackets inside round ones, and no nesting. Write
  variable parts as capitalized tokens such as AGE or TEETH, defined once in a global block.
- **Global instructions** are round-bracket blocks at the very top or bottom. If one is ignored,
  move it to the other end.
- **Conditionals**: write "If CONDITION, print "..."; else omit." Every "if" needs a stated
  outcome for when it fails. Keep nesting to 3 levels at most, and conditions specific ("if the
  patient takes an anticoagulant", not "if relevant").
- **Gap handling**: "Only include if explicitly mentioned ..., else omit" removes a line or
  section. A fallback prints fixed text instead, such as "***" or "NKDA".
- **Directive words** ("Always", "Never", "Must") strengthen instructions Heidi tends to skip.
- Heidi is unreliable at arithmetic, date maths and tables.
- The account's Voice setting (Brief to Super Detailed) changes output length for every template.

## Observed behaviour

- **Long condition lists get skipped.** With 12 risk items as prose, Heidi missed two whose
  conditions were met. A numbered list, with an instruction to decide yes or no for each item,
  fixes it.
- **A token gets dropped when its value is missing**, so "AGE-year-old" became "Patient" or
  "The patient presenting", even with "never drop it". Give a worked example of both forms ("38-year-old
  patient presenting..." / "***-year-old patient presenting...") and name the wrong form to avoid.
- **Fixed term lists invite invented extras** (a "Caffeine" social line), and they also suppress
  stated items that aren't on the list (stress as an aggravating factor). Say which lists are
  closed ("never add other items") and which accept other stated items.
- **"Not in close proximity" was inferred from vague talk.** Default to the conservative wording,
  and print the alternative only on a clear statement.
- **Judgments leak into objective sections** ("not concerning"). Keep a global rule forbidding
  them.
- **Bullets** are lost when the surgeon copies out of Heidi, not inside Heidi. Ask for plain text
  lines that start with "- ".
- Size: a template of about 23,000 characters was accepted.
