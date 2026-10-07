# The prompt for the prose pass

*For Ray: paste this whole file, every time, whoever is reading it — a fresh
Fable, a resumed Fable, or Opus picking up the pieces. The router below sends
each of them to the right place, so you do not have to remember which is which.*

---

## First: which reader are you?

**Check whether `book-pdf/HANDOFF.md` exists and has content in it.**

- **It does not exist, or it is empty.** You are starting this pass. Everything
  below is addressed to you, in the order written. Begin with the tone study.

- **It exists and has content.** A pass is already underway — whether or not you
  are the model that began it. **Go straight to "Resuming: taking over a pass in
  progress" at the end of this file** and follow it. Treat everything in between
  as reference you consult when you need it, not instructions to execute: in
  particular, the tone study has been done, chapters have been written, and the
  section about having written the existing draft is not about you.

Getting this wrong is expensive in both directions — redoing the tone study wastes
the budget, and skipping it produces a chapter in the wrong voice. Thirty seconds
checking `HANDOFF.md` settles it.

---

You are writing *Rust for RTL Verification* — 40 chapters, an interlude, and four
appendices, in `book-pdf/src/`. The framework it teaches (`rustdv/`) is finished
and every example runs. A draft of the manuscript exists and is stale: it was
written against an earlier design that has since been reversed.

## Read these two, then start

1. `book-pdf/FABLE.md` — what the book argues and what it must stop claiming.
2. `book-pdf/chapter-notes.md` — one row per chapter. Read the header now, a row
   when you reach that chapter.

**Do not read `output/.design-decisions.md`.** It is 3,000 lines of internal
history and everything you need from it is already in those two files. If you
find yourself wanting it, they have a gap — say so.

You may read anything in `rustdv/`, `output/examples/`, and
`../rustdv-reference/` (cocotb, pyuvm, four releases of the SystemVerilog UVM,
and the example code from both earlier books).

## The source for how this book should sound

`../rustdv-reference/salemi_books/Python for RTL Verification.pdf` is Ray's
previous book and the model for this one. **Read enough of it to absorb how it
teaches** — the front matter, an early teaching chapter, the `uvm_test` chapter
that maps to this book's ch23, and a sequences chapter. Not cover to cover; you
are after approach and voice, not content.

(You may also refer to *The UVM Primer* if you have SystemVerilog questions.)

What to take: how a chapter opens, how a mechanism is introduced before it is
named, how much is explained before the first listing, how transcripts are used
as evidence, the rhythm of the sentences, where the jokes sit.

**What not to take, and this matters.** That book makes a case for *not* having
types, and it makes it with enthusiasm. This book is not the rebuttal. The last
draft of this manuscript did try to be the rebuttal, and it is why the whole pass
is being redone. Both moves — celebrating types, celebrating their absence — make
the type system the subject. **The subject is verification.** Typing is a design
trade with real costs on both sides, and `FABLE.md` states the position the book
actually takes. Take the pedagogy; leave the advocacy.

Nor its audience. That book was written for Python readers; this one addresses
SystemVerilog and Python engineers equally, so nothing may assume the reader has
used cocotb or pyuvm.

**Write the study down** in `HANDOFF.md` before you write a chapter — what you
took, in enough detail that someone who has not opened the PDF can write in the
same voice. This is the highest-value thing you will produce all pass.

## You wrote the current draft, and you may throw it away

The existing manuscript is your own earlier work, so preserving it is not
protecting an irreplaceable voice — it is protecting a first draft by the same
writer. **A blank sheet is allowed, and for any chapter whose argument was wrong
it is usually the better choice.** Patching around a broken premise produces
prose with visible seams.

Read the current chapter before you decide — it may hold a good explanation or a
joke worth keeping, and it tells you what ground the chapter has to cover. Then
choose: revise it, or start clean. Your call, per chapter.

Judgement, not rules: chapters whose argument is intact and whose listings are
frozen (all of Part I) rarely need a rewrite; chapters that argued for the
reversed design almost always do.

## The constraints

Everything else is yours. These are not.

- **You change no code. Ever.** You write only in `book-pdf/`. Not `rustdv/`, not
  `output/`, not `sim/`, and no `.rs`, `.toml`, `.json`, `.sh` or HDL file.
- **The code wins.** The examples are verified running; the manuscript is known
  stale. If an example looks wrong, tell Ray — never "fix" it.
- **Transcripts are copied verbatim** from the chapter READMEs in
  `output/examples/` (and `STATUS.md` for the shipped testbench). Never retype,
  tidy, invent or regenerate one. Never run a simulator.
- **Part I listings are frozen.** `book-sync` compares chapters 1–14 against
  their example files verbatim and the pre-push hook enforces it. Prose around
  them is yours; a character inside a code block is not.
- **Do not sell types, and do not disparage what came before** — Python,
  SystemVerilog, the UVM, pyuvm, cocotb, or the earlier books. `FABLE.md` has the
  reasoning. This is the note Ray most wants changed from the last draft.
- **Ask rather than invent.** Anything marked ASK RAY in `chapter-notes.md`, and
  any fact a chapter needs that the examples do not contain.

Do not run the regression.

**Do render, as a structural check.** Run `mdbook build book-pdf` whenever you
touch `SUMMARY.md` or add a file — you are adding at least Appendix D — so a
broken TOC entry or an orphaned chapter surfaces immediately. If `mdbook` is not
on the path, it ships prebuilt and offline in the toolchain drop:

```sh
tar -xzf toolchain-drop/mdbook-v*.tar.gz -C /tmp/rust/bin mdbook
```

Expect `WARN The command mdbook-pdf ... was not found, but is marked as
optional` — that is correct here and the build still exits 0. The PDF backend
needs a Chromium the sandbox lacks. The rendered output under `book-pdf/book/`
is generated and gitignored; never edit it, and do not be surprised that it
changes.

**Do not read the rendered output.** Reviewing the book as a reader sees it is a
separate pass that happens after the numbering is final, because half of what it
catches is references that no longer resolve. That pass is not yours.

## Figures: one sequence, reconciled at the end

**One numbering space per chapter.** Code listings, drawings, tables and
transcripts all draw from the same sequence, in order of appearance, and every
one of them is a "Figure" (D110). If Figure 1 is a drawing, the first listing is
Figure 2. Nothing is renamed and there is no scheme for you to pick — `FABLE.md`
has the reasoning.

So: **do not inventory the existing captions, and do not let their numbers
constrain a chapter.** Write the chapter, put the figures where they belong, and
number them as they appear. Add drawings and tables where they earn a place —
that is now possible, and three things in Part II that are currently ASCII art
inside code comments probably want to be pictures.

When the book is finished, write **`book-pdf/renumbering-spec.md`**: for each
chapter you changed, the figure numbers your prose now uses, mapped to the code
they correspond to, and flagging any listing whose number moved because you
inserted a figure ahead of it. A mechanical pass then edits the `.rs` captions to
match. Note two constraints in the spec for whoever runs it: edits must be **in
place and line-count-neutral**, because transcripts embed `file:line`; and Part
I's numbering feeds `manifest.json` and `book-sync`, so say explicitly whether
anything there moved.

## Order of work, and why it is fixed

There is a fixed budget and it may run out before the book is done. If it does,
**Opus 5 takes over from your notes** — see the last section of this file, which
is what it will read. So: every judgement call early, every repetition late.

1. **The tone study** above, written into `HANDOFF.md`.
2. **Chapter 1.** It sets the frame once for the whole book. Get it right and
   every later chapter gets shorter, because none of them has to re-argue it.

**Then stop.** Show Ray the tone study and chapter 1, and wait. This is the only
mandatory checkpoint in the pass, and it exists because voice is the one thing
neither of you can verify in advance: everything else in this file can be checked
against a file on disk, and how the prose *sounds* cannot. A wrong turn caught at
chapter 1 costs one chapter; caught at chapter 30 it costs the budget. Do not
treat silence as approval — wait for an answer.
3. **The rustdv catalogue and Appendix D** — the `prelude::*` debt in
   `FABLE.md`. This gates all of Part II.
4. **The chapters whose argument is reversed**: ch24, ch27, ch31, ch32, ch36,
   ch39, then ch28, ch29, ch34. The most conceptually expensive work in the book
   and the least suitable for a successor. Do it while you can.
5. **Two exemplars, declared as such** — one Part I chapter and one ordinary
   Part II chapter, written to full quality, with the routine you used recorded
   in `HANDOFF.md` and both named in its Exemplars table. **A successor is told to
   read these as its model for the voice**, so they matter more than their
   position in this list suggests: they are the only place the voice exists as
   prose rather than as description.
6. **Everything else**, in `SUMMARY.md` order.
7. **`renumbering-spec.md`**, last.

After that the work leaves you: a mechanical pass reconciles the `.rs` captions
to your numbering, the book is rendered, and someone reads it end to end against
the rendered output. Leave `HANDOFF.md` in a state that makes those three steps
obvious.

One note. **The Interlude and ch40** both present the shipped testbench in
`rustdv/tinyalu_tb/`; its transcript is in the last entry of `STATUS.md`. The book
ends at ch40 — there is no chapter 41 (D111).

## The handoff

**Never leave a chapter half-written**, so a takeover always starts at a clean
boundary. **Update `book-pdf/HANDOFF.md` after every chapter** — a paragraph is
enough. Assume you get no warning.

```
# Handoff

## The tone study
<what you took from Python for RTL Verification, in enough detail to write
without the PDF>

## Durable decisions (append-only)
<every judgement call, one line each, with the reason>

## Exemplars
| Archetype | Chapter | What to imitate |

## Cross-chapter threads
<callbacks, running examples, promises made in one chapter and kept in another —
the things that break silently when chapters are written in isolation>

## Progress
| Chapter | State | Notes |

## Next action
<the single next thing, specific enough to start cold>

## Open questions for Ray
```

## Efficiency

One chapter at a time. For each, read four things: its row in `chapter-notes.md`,
its current file in `book-pdf/src/`, its example crate, and that crate's
`README.md` for the transcript. **Never read the manuscript in bulk.**

After the chapter-1 checkpoint, a finished chapter and a one-line handoff update
is the report — no summaries back to Ray beyond that. The exception is anything
you had to guess: raise that immediately, however small, rather than saving it.

Start with the tone study.

---

# Resuming: taking over a pass in progress

*The router at the top sent you here because `HANDOFF.md` has content. This
section is yours whether you are Opus 5 taking over, or Fable returning in a
fresh session with its context gone. Either way, two things above do not apply:
the tone study is already done, and if you are not Fable you did not write the
existing draft.*

## Cold start, in order

1. **`book-pdf/HANDOFF.md`, in full.** It is the state of the world: what was
   decided, what is written, what comes next. Read it before anything else.
2. **`book-pdf/FABLE.md`, in full.** What the book argues and how it sounds. Not
   optional — the tone rules in it are the ones Ray cares most about.
3. **The exemplar chapters named in `HANDOFF.md`.** Read them as *finished
   prose*, not as instructions. Two worked chapters teach the voice more reliably
   than any description of it, and imitating them is your job.
4. **The tone study section of `HANDOFF.md`.** If it reads thin, spend one
   chapter of `../rustdv-reference/salemi_books/Python for RTL Verification.pdf`
   on it rather than guessing — that book is the model, and a shallow imitation
   of a summary of it is the worst of both.

Then resume from `HANDOFF.md`'s **Next action**, one chapter at a time, with the
same four reads per chapter listed under Efficiency above. That per-chapter rule
is what makes this handoff possible: no chapter depends on context Fable
accumulated, so you are not missing anything by having arrived late.

## What carries over unchanged

The constraints — no code edits, code wins, transcripts verbatim, Part I listings
frozen, don't sell types, don't disparage what came before, ask rather than
invent. The order of work. The figure convention. The `mdbook` structural check.
Blank sheet is still permitted where a chapter's argument was wrong.

## What is different for you

- **You did not write the draft.** Where Fable was told it could throw away its
  own work freely, you are revising *two* authors: the stale original and the
  chapters Fable has already rewritten. **Do not re-litigate a finished
  chapter.** If one looks wrong, note it for Ray and move on — rewriting
  completed work is how a budget disappears with nothing to show.
- **Consistency now matters more than flair.** Fable set the conventions; your
  job is to extend them, not improve on them. A book of forty-one chapters in one
  voice beats a book with six better ones.
- **You inherit the handoff duty.** Keep `HANDOFF.md` current after every
  chapter, and never leave a chapter half-written. You may be handing off too.
- **`HANDOFF.md`'s open questions are Ray's**, not yours to settle. Same for
  anything marked ASK RAY in `chapter-notes.md`.

## If `HANDOFF.md` is missing or thin

Then the handoff failed and you should say so rather than improvising. Read
`FABLE.md`, `chapter-notes.md`, and the most recently modified chapters in
`book-pdf/src/` to infer the conventions in use, write down what you inferred as
the new Durable decisions, and tell Ray what you had to guess. Reconstructing the
notes is cheaper than writing forty chapters that disagree with the first ten.
