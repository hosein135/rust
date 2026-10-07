# Handoff

## The tone study

Source: *Python for RTL Verification* (2022). Read: front matter and
Introduction, "Python basics", "uvm_test testbench: 3.0", "uvm_object in
Python" (tail), "Sequence testbench: 7.0". What follows is enough to write in
the voice without opening the PDF.

### The shape of a chapter

Chapters are short — one job, six to ten pages, many headings. A chapter opens
by locating itself in the arc in one or two sentences: what the last testbench
version did, what it could not do, and the gap this chapter fills. Example
opening move (7.0): "Testbench 6.0, our most advanced so far, used UVM
components to build a testbench out of single-function components... Our next
step is to create a testbench whose structure stays the same while generating
different stimuli." The need is stated as a problem the reader already felt —
*then* the mechanism is named. The "Why UVM?" chapter is literally organized
as the questions a testbench must answer ("How do we pass data around the
testbench?"), with the UVM presented as the standardized answers.

Headings are frequent, short, and concrete: a class name ("The BaseTest
class"), a component ("Driver"), a task ("Creating sequences", "Starting a
sequence in a test"). No clever headings.

Chapters end with a **Summary**: two to four short paragraphs restating what
was learned in plain declarative sentences, sometimes with one forward
pointer and occasionally one flagged personal recommendation ("...though I
recommend using the Python copy module functions").

### How a listing is presented

The unit is: one or two sentences saying what the figure will show → the
listing → a walkthrough of only the *new* lines. The walkthrough is a bulleted
list, each bullet `code element`—explanation, in source order. When the code
revises an earlier version, the walkthrough is a *numbered* list of
differences ("There are four differences between this version and the 6.0
version"). Nothing already taught is re-explained; instead a parenthetical
back-reference: "(We defined the RandomTester and MaxTester classes in the
Class-based testbench 2.0 chapter.)"

Announcements are honest and plain: "I'll show the code in figure 1 and
explain it below." "Here is the new TinyALU Driver."

### Transcripts as evidence

Immediately after code runs, the output appears as its own figure: "Here is
the result when we run the test." → transcript → one or two sentences
interpreting a *specific* line of it ("The pyuvm logger puts the path to the
component that logged the message between the square brackets"). Output is
proof, never decoration; every claim about behavior is followed by the lines
that show it. Ends of arcs get a one-line verdict: "We've successfully
defined and run our first pyuvm test."

### Sentence rhythm and person

Short declarative sentences, present tense. "We" is the default — author and
reader doing the work together ("We'll take that code and put it into the
BaseTest"). "I" appears only for authorial choices and opinions, and opinions
are flagged as experience, not decree: "It's been my experience that these
errors are rare enough that..." "You" for instructions to the reader. No
hedging, no throat-clearing, no "note that" pile-ups. Paragraphs are two to
four sentences.

Forward and backward pointers are explicit and chapter-named: "We will
discuss phasing in the next chapter." "The Logging chapter discusses logging
details." Rhetorical questions create pull between sections: "Which raises
the question, where did self.tester come from?" — asked at the end of one
section, answered by the next.

### Where the jokes sit

In parentheses, footnotes, and variable names — never in the argument's
spine. "Users don't extend uvm_void (hence the name.)" A variable named
`no_pi_for_you` in the int-conversion error example. "Not used in
verification often, but cool nonetheless" (complex numbers). Footnote: "Do
not confuse UVM sequences with the Python sequences. Same word, different
meanings." One per few pages, dry, and skippable. The Introduction earns one
longer piece of storytelling (the Roman-chariot/Space Shuttle urban legend as
a frame for language incrementalism) — narrative is allowed in chapter 1
scale-setting, not in mechanism chapters.

### Comparisons anchor, they don't score

New constructs are anchored to what the reader knows in one clause: "from
pyuvm import * ... matches how we do imports in SystemVerilog UVM where we
type import uvm_pkg::*." The comparison's job is orientation, then it gets
out of the way. This book's version: both foils, SystemVerilog first, one
parenthetical — *(SV: `uvm_config_db#(int)::set`; pyuvm: `ConfigDB().set`)* —
per FABLE.md.

### What I am deliberately NOT taking

1. **The typing advocacy.** The source book sells dynamic typing with
   enthusiasm ("Dynamic typing makes it easier to create testbenches and
   results in a more straightforward implementation of the UVM") and argues
   the trade in its own favor ("these errors are rare enough that the work of
   matching parameters is not worth the time lost"). This book takes the
   pedagogy and drops the position — no celebrating types, no celebrating
   their absence, per FABLE.md. The subject is verification.
2. **The single audience.** The source assumes a Python-curious reader and
   compares only against SystemVerilog. This book addresses SV and Python
   engineers equally and may not assume the reader has run cocotb or pyuvm.
3. **"We store these in directories"-grade logistics prose** — the source's
   front matter has some rough passages; imitate its structure, not its
   copyedit.

### The checklist I write against

Open by locating the chapter in the arc; state the problem before naming the
mechanism; introduce every identifier before it appears in a listing; listing
→ walkthrough of new lines only; transcript immediately after, interpreted,
verbatim; back-reference instead of re-explain; end sections with the
question the next section answers; Summary restates plainly; jokes in
parentheses only; comparisons orient, never score; "we" works, "I" opines,
"you" acts.

## Durable decisions (append-only)

- 2026-07-30: Tone study done from front matter + Python basics + uvm_test 3.0
  + uvm_object + Sequence 7.0 chapters. Reason: the four reads fable-prompt.md
  specified, and they triangulate voice across language-teaching and
  UVM-teaching modes.
- 2026-07-30: ch1 — kept the opening (audience framing), history, costs, code
  examples, and both frozen listings; blank-sheet rewrote "Why Rust for
  verification?" and "The plan". Reason: the old middle promised config/
  factory/TLM errors as compile errors (the reversed claim); the plan
  described a five-part structure and no Interlude, contradicting SUMMARY.md.
- 2026-07-30: ch1 states the seam as a block quote ("Types for data and
  ownership. Runtime indirection for topology and binding.") and argues both
  halves in two paragraphs, then the two surviving reasons (scale,
  throughput/emulation) and the two ownership wins ('static-on-spawn and
  async-trait stories, told without jargon, promised for later chapters).
- 2026-07-30: Softened, not cut: the borrow-checker joke, "maddening →
  endearing", "bend your brain exactly once". Cut: "the whole book in
  miniature" (made compile-checking the thesis), "very small tolerance for
  runtime errors" closer, "assembled by committee" jab at SV, the claim that
  Parts II–V showcase compile-error figures.
- 2026-07-30 (Ray): the book never names a repo or path for the examples —
  "you can get a copy from the book's repository" is the whole promise. The
  actual location (output/examples) is a repo-layout detail readers don't
  need and that could change.
- 2026-07-30: The rustdv catalogue is its own unnumbered page,
  `the-rustdv-toolkit.md`, placed after the Interlude and before ch15 in
  SUMMARY.md (not folded into ch17). Reason: ch15's crate already uses the
  prelude and `#[rustdv::test]`, so ch17 is too late; the Interlude precedent
  makes an unnumbered page natural. Appendix D added after C. mdbook build
  green (build with `-d /tmp/...` — the mounted book/ dir can't be cleaned).
- 2026-07-30: Appendix D's Chapter column was assigned from first-use greps
  of the example crates plus chapter subjects. Re-verify each assignment
  when writing its chapter (esp. vpi_bootstrap→17, Event/Lock→16,
  channel→31, CheckSink→24, Active→40); fix silently if a chapter teaches a
  name elsewhere. `first!`/`join!` never appear in example listings — they
  are documented as provided, taught conceptually in ch16 via join2.
- 2026-07-30: Toolkit page ends with a standing promise: no listing uses a
  name not declared by the page or an earlier chapter. Every chapter pass
  must honor it.
- 2026-07-30: Drawing convention — small structural sketches (e.g. ch24's
  tree) are `text` blocks with a `# Figure N:` caption line like transcripts;
  dataflow/architecture drawings (ch31 pipeline, ch34 architecture, ch36
  handshake) are inline SVG wrapped in `<figure>` with an italic
  `<figcaption>` "Figure N: ...". SVG uses stroke #888, fill none, text
  fill=currentColor so both mdBook themes work.
- 2026-07-30 (Ray): SUMMARY.md keeps its bare separators — no rendered part
  titles. Consequence for prose: any "Part I/II" mention must carry chapter
  numbers ("Part II (Chapters 15–40)") so it resolves without TOC labels;
  never a bare "as we'll see in Part II."

## Exemplars

| Archetype | Chapter | What to imitate |
|---|---|---|
| Part I dual-audience pass | ch5 (Ownership) | Surgical edits, not rewrite: stale structure refs (Part IV, channels) → chapter numbers and restored vocabulary (FIFOs, finish_item); soften any line that jabs a source language; kill banned words; listings untouched. Prose already dual-audience stays. |
| Part II testbench chapter | ch23 (uvm_test 3.0) | The full routine: open by locating in the TB-version arc; "In the UVM..." box in shared API terms; listing → walkthrough of new lines only (bulleted `code` — explanation); transcript immediately after, verbatim, with one detail interpreted (e.g. pathless PASSED lines measuring what's missing); re-show convention explained once; forward pointers by chapter number; Summary restates + hooks the next chapter. |

### The per-chapter routine (used for every chapter above)

1. Read the chapter-notes row, the current manuscript file (skim if the
   argument is reversed), the crate `.rs` in full, and the README.
2. Decide revise vs blank sheet. Reversed argument → blank sheet, but mine
   the old file for the "In the UVM" box and any good joke.
3. Figures: follow the crate's captions; insert transcript/drawing figures
   in order of appearance; record every shift in the Renumbering ledger the
   same session. The crate's doc-comments usually carry the argument —
   quote their *substance*, never their D-numbers.
4. Transcripts only from README (verbatim, contiguous excerpts allowed). No
   README transcript → `[TRANSCRIPT NEEDED — ...]` placeholder + add to
   "Transcripts owed" + tell Ray.
5. Update Progress table + ledger; mdbook build after SUMMARY/file changes.

## Cross-chapter threads

- ch1 promises two compiler stories: the async-trait restriction (paid off in
  ch24's "What this costs") and the `'static`-on-spawn story (still owed —
  pay it in ch15/16 or wherever the crate shows it; check before claiming).
- ch24's "Why build and connect exist" promises collections in ch25
  (config-before-children), ch29 (factory override), ch31 (connect over a
  finished subtree). Each of those chapters should land its callback.
- ch24 promises ch31 shows parent and children running *at the same time*
  (D82b/c). ch31's y=2x² pipeline is that payoff.
- ch25 preview line in ch24's summary: BFM via ConfigDb, scoreboard checks in
  `check`.
- ch32 promises ch34's scoreboard shows two `SubscribePort`s + two `WriteSink`
  impls (the imp_decl contrast made concrete). ch33/34/36 manuscripts still
  say `AnalysisFifo` in places — must become `AnalysisBus` when rewritten.
- ch31 fig 9 (pipeline) is called "the DUT-free rehearsal for testbench 7.0"
  — ch36 should call back to it.

- Testbench version numbers (1.0–8.0) are the book's spine, as in the source
  book. Chapter openings recap the previous version's limit.
- The Interlude and ch40 present the same shipped testbench
  (`rustdv/tinyalu_tb/`); its transcript comes from the last entry of
  STATUS.md. No chapter 41 (D111).
- The frame (types for data and ownership; runtime indirection for topology
  and binding) is stated once, in ch1, and never re-argued.

## Renumbering ledger (feeds renumbering-spec.md, written last)

- ch1: unchanged (figs 1–2).
- ch24: figs 1–7 as published; .rs captions 1/4/5/6 already match; fig 2 and
  7 are transcripts; fig 3 is a book-only drawing (no code counterpart, fills
  the crate's deliberate gap at 3).
- ch31: transcripts inserted; book→crate map: 1→1, 2→2, 3→3, 4=transcript,
  5→4, 6→5, 7→6, 8=transcript, 9=drawing (crate "fig 7 text+diagram" — now an
  inline SVG), 10→8, 11→9, 12→10, 13=transcript, 14→11, 15=transcript(error
  report), 16→12, 17=transcript. .rs captions 4,5,6,8,9,10,11,12 must become
  5,6,7,10,11,12,14,16.
- ch32: book→crate map: 1→1, 2→2, 3→3, 4→4, 5=transcript, 6→5, 7=transcript,
  8→6, 9→7, 10=transcript. .rs captions 5,6,7 must become 6,8,9.
- ch36: book→crate map: 1=drawing (SVG handshake; crate's old README had
  "text diagram" at 1), 2→1, 3→2, 4→3, 5→4, 6→5, 7→6, 8→7, 9=transcript
  (owed). .rs captions 1–7 must become 2–8. The AluCommand/AluResult re-show
  block is uncaptioned in both crate and book (not a numbered figure).
- ch39: presentation reorders the crate: book 1→crate 1, 2→crate 6 (AluTest),
  3=transcript, 4→crate 2, 5=transcript, 6→crate 3, 7→crate 4, 8→crate 5,
  9=transcript. .rs captions must become: crate1→1, crate2→4, crate3→6,
  crate4→7, crate5→8, crate6→2. ParallelTest/FibonacciProgramTest listings
  are uncaptioned in crate and unlisted in book (described in prose).
- ch28: book→crate map: 1→1, 2→2, 3→3, 4→4, 5→5, 6→6, 7=transcript(owed:
  MESG dump), 8→7, 9→8, 10=transcript(dump excerpt, verbatim from README),
  11→9, 12=transcript(trace excerpt, from README). .rs captions 7,8,9 must
  become 8,9,11.
- ch29: the crate left gaps (3,5,9,11,14) for transcripts; book fills them
  from the README's continuous run, plus a new fig 16 (PrintOverrides
  output) at the end. **No .rs caption shifts.** CreateByNameTest's output is
  prose, not a figure (crate left no slot).
- ch34: book figs: 1=SVG architecture drawing (new), 2→crate ch34 fig 1
  (AluEnv), 3→crate ch34 fig 2 (AluTest), 4=transcript (verbatim from
  README). .rs captions "Chapter 34, Figure 1/2" must become 2/3. The Tester
  flush loop is quoted as an uncaptioned `rust,ignore` snippet (it is part of
  ch33's fig 1, not re-figured here).
- ch23: book figs 1–7 match the crate's slots exactly (2, 6, 7 transcripts;
  3 = tower text diagram). **No .rs caption shifts.** The re-shown TB 2.0
  classes keep their "Chapter 20, Figure N" captions and are not re-listed
  in the book.
- ch5: listings untouched; prose-only edits. No renumbering.
- ch25: figs 1–10 = crate 1–10 unchanged; fig 11 = transcript (verbatim from
  fresh README) appended. **No .rs caption shifts.**
- ch26: figs 1–7 = crate 1–7 unchanged; figs 8–9 = transcripts (run + log
  file, verbatim from fresh README) appended. **No .rs caption shifts.**
  Note: fig 1 listing needs `use rustdv::sim::log::Level` context — the
  crate imports it; the chapter mentions `Level::Debug` without an import
  line (glob-stays rule, Level is via rustdv::sim — acceptable, App D notes
  `log`).
- ch30: figs 1–5 = crate 1–5 unchanged; fig 6 = transcript (fresh README).
  **No .rs caption shifts.**
- ch33: figs 1–6 = the ch34 crate's "Chapter 33, Figure 1–6" captions,
  unchanged. No transcript (definitions chapter; runs in ch34 by design).
  **No .rs caption shifts.**
- ch35: figs 1–7 = the seven bins, Part-I style (code, `--`, expected output
  from the bins' header comments). Listings lightly trimmed of run-with
  header comments only; all required impls kept (fig 4/6 Display included).
  **No .rs caption shifts.**
- ch40: figs 1 (layout tree, book-only), 2–5 (excerpts of tinyalu_tb — the
  crate has no figure captions of its own; ch40's numbering is book-defined),
  6 = transcript (owed; same run as Interlude fig 7).
- ch37: ~~book figs 1,2,3 = crate 1,2,3; crate's "Figure 4 is a paragraph"
  stays prose (the hanging-get_response paragraph); book fig 4 = ShopEnv +
  RepairTest (crate captions the test as Figure 5 — .rs caption 5→4); fig 5 =
  transcript (owed).~~ **Void 2026-08-05** — the example was replaced and the
  chapter rewritten; the figure map lives in
  `output/examples/ch37-out-of-order-transaction-testbench-7.1/README.md`.
- ch38: figs 1–5 = crate 1–5 unchanged; fig 6 = transcript (owed). No shifts.
- ch27: transcripts inserted into the sequence shifted the .rs captions.
  Book→crate map: 1→1, 2→2, 3→3, 4=transcript(new), 5→4, 6→5, 7=transcript,
  8→6, 9→7, 10=transcript, 11→8, 12→9, 13=transcript. The .rs captions for
  crate figs 4–9 must become 5,6,8,9,11,12 (line-count-neutral edits).

## Transcripts owed — ALL PAID (2026-07-30, closing session)

All 13 `[TRANSCRIPT NEEDED]` markers are filled verbatim from the rebuilt
READMEs (ch27 ×4, ch28 ×1 + its two excerpt figures upgraded to full
transcripts, ch36 ×1, ch37 ×1, ch38 ×1, ch39 ×3, Interlude ×1 = MaxTest
portion, ch40 ×1 = RandomTest tail — the two halves of one
`rustdv/tinyalu_tb/README.md` run, split to avoid printing the same block
twice). ch15/17/21 path citations repointed off `src/lib.rs`; ch18/19/20
transcripts advanced 5ns per D112 (prose numbers updated: ch18's "125
nanoseconds" → 130). ch16 unaffected (no clock, no embedded paths).

**The `#[component]` sweep (D114) is also done**: 17 files mechanically
updated; ch24 now introduces the bare attribute as the whole rule; ch21's
expansion figure updated; the concrete-vs-erased distinctions in ch31, ch32,
ch36 now hang on the field's *type*, not the attribute's argument.
`#[component(no_factory)]` (also deleted from the code) removed from ch29 —
the one exception named there now is generic components.

The list below is retained for history only.

## Transcripts owed (blocked on Ray rerunning sims) — HISTORICAL

- Interlude fig 7 (and ch40 will need the same): the shipped testbench run,
  `sim/run_rustdv.sh`, verbatim. STATUS.md describes the counts
  (RandomTest 20/0, MaxTest 4/0) but carries no verbatim block.

- ch36: one placeholder (fig 9), the full run of BaseTest/RandomTest/MaxTest.
  README stale (cites src/lib.rs, rustdv-uvm paths).
- ch39: three placeholders (figs 3, 5, 9): AluTest, ParallelTest,
  FibonacciProgramTest portions. README stale (19 lines, old figure set).
- ch37: one placeholder (fig 5), `.../run_sim.sh ch37_repair_desk_testbench_7_1
  playground`.
- ch38: one placeholder (fig 6), `.../run_sim.sh ch38_fibonacci_testbench_7_2
  tinyalu ...`.
- ch27: four placeholders marked `[TRANSCRIPT NEEDED]` in the manuscript —
  MsgTest, MultiMsgTest, GlobalTest, ConflictTest outputs from
  `sim-common/run_sim.sh ch27_configuration playground`. README is stale
  (23 lines, old figure map, cites src/lib.rs which no longer exists).

## Progress

| Chapter | State | Notes |
|---|---|---|
| Tone study | done | this file |
| ch1 | done | checkpoint passed 2026-07-30; listings byte-identical to HEAD |
| Toolkit page | done | new file, in SUMMARY before ch15 |
| Appendix D | done | new file, after Appendix C; chapter column needs per-chapter re-verification |
| ch24 | done | blank-sheet rewrite; figs 1–7 (3 = new tree drawing filling the crate's deliberate gap; .rs captions 4/5/6 unchanged); transcripts verbatim from README; retitled "Components" in SUMMARY |
| ch27 | done, transcripts owed | blank-sheet rewrite from the converted crate (its comments carry the argument); 4 transcript placeholders; retitled "Configuration" |
| ch31 | done | blank-sheet rewrite; README fresh, transcripts verbatim; pipeline is an inline-SVG drawing (fig 9); D83b uniformity + try_put handback + concurrency payoff all landed |
| ch32 | done | blank-sheet rewrite; hub-holds-nothing is the thesis; AnalysisBus name story in prose; imp_decl contrast lands here, two-WriteSink demo deferred to ch34 as promised |
| ch36 | done, transcript owed | blank-sheet rewrite; SVG handshake diagram as fig 1; why-two-calls + finish_item ownership + steering-wheel frame all landed |
| ch39 | done, transcripts owed | blank-sheet rewrite; no-VirtualSequence-trait argument (thesis applied to itself); do_add interface + Fibonacci program; driver-answers env explained in prose |
| ch28 | done, 1 transcript owed (fig 7) | blank-sheet rewrite; toolkit framing (Result variants / dump / trace / expect_error); compile-fail figures stay dead |
| ch29 | done | blank-sheet rewrite; README fresh, transcripts verbatim; new/create distinction as block-author decision; universal registration; ch24 callback landed |
| ch34 | done | blank-sheet rewrite; SVG architecture (fig 1); objection-flush + silent-under-check danger + D82c war story; two-WriteSink scoreboard cashes ch32's promise. ch33 must present crate figs "Chapter 33, Figure 1–6" (Tester, Driver, monitors, Scoreboard, Coverage) and NOT re-show the env |
| ch5 | done (EXEMPLAR, Part I) | dual-audience pass: 5 surgical edits, listings untouched |
| ch23 | done (EXEMPLAR, Part II) | blank-sheet rewrite from converted crate; two front doors; type-name registration; pathless-PASSED observation |
| ch2–ch14 (pass) | done | dual-audience pass was already largely done in the old draft; this pass fixed stale forward refs (Part III/IV/V → chapter numbers; channels → queues/FIFOs; factory-by-closures → makers-in-registry; BFM-via-constructor → ConfigDb) and swept genuinely/honestly. ch5+ch23 are the exemplars |
| Interlude | done, transcript owed (fig 7) | full rewrite from rustdv/tinyalu_tb (the converted crate); figs 1–6 verbatim excerpts; recognition-not-explanation; closing map updated to real chapter numbers |
| ch15–ch21 (pass) | edits done; **READMEs regenerated 2026-07-30 — manuscript needs re-checking against them** | stale Part III/IV refs, tone words, and ch21's reversed no-registration claim fixed (derive now registers — matches D73). The READMEs are now current: `src/lib.rs` citations repointed at the real crate roots, and **ch18/ch19/ch20 transcripts corrected — they were 5ns early throughout** (D112: the DUT self-clocks, so the first edge lands later; same operands, same results). The manuscript's listings and transcripts were copied from the *stale* READMEs, so ch18–20 quoted times are wrong and any ch15–21 path citation wants a look. |
| ch22 | done | prose-only rewrite; the runtime-on-purpose paragraph is the chapter's spine; question list re-answered to the restored design |
| ch25 | done | blank-sheet rewrite; ConfigDb two-line intro + no-singleton-anywhere + cost stated; 'static spawn = ch1's first compiler story landed; generics flagged for ch30's reversal |
| ch26 | done | blank-sheet rewrite; no-path-anywhere lesson; runner-resets-between-tests evidence in the log file |
| ch30 | done | blank-sheet rewrite; compile-time vs run-time variation rule corrects ch25's generics on purpose; identical-transcripts-as-proof |
| ch33 | done | blank-sheet rewrite; one-job-each principle; two-sink scoreboard delivered as promised; getattr-monitor contrast; explicitly cannot run |
| ch35 | done | blank-sheet rewrite; not-an-object opening (Ray's directive); Debug≠convert2string corrected; equality-on-the-transaction; PartialEq/Eq coverage-key bite; Copy false friend |
| ch37 | done, transcript owed | blank-sheet rewrite (repair desk, per crate); file renamed; try_next_item necessity; ticket table; hanging-get_response as prose |
| ch38 | done, transcript owed | blank-sheet rewrite (Fibonacci, per crate); file renamed; no-flush + no-result-monitor arguments; nothing-travels-backwards |
| ch40 | done, transcript owed | blank-sheet rewrite; walk-of-whys (clock paragraph, wait_idle vs magic-20, passive-env empty slot, scoreboard guards + mutation story); template framing |
| Appendix A | done | retitled rows (24, 29, 37, 38); ch41 row removed with a closing sentence |
| Appendix B | done | methodology rows rewritten to restored design |
| Appendix C | done | methodology table rewritten to restored design |
| renumbering-spec.md | done | in book-pdf/; constraints + per-chapter maps + README regeneration list |
| caption pass | **applied 2026-07-30** | code thread ran it: 8 crates, line-count-neutral, crates rebuilt clean. No manuscript `file:line` invalidated |
| `#[component]` sweep | **owed** | D114 removed the attribute's argument; 17 manuscript files still print the old form. ch21 + ch24 need rewriting, not substitution. See "New since the pass closed" |

## Next action

**The prose pass is COMPLETE** (2026-07-30): all 40 chapters, the Interlude,
the Toolkit page, and Appendices A–D are written or passed;
`renumbering-spec.md` is written; mdbook builds green.

### Transcripts are verified on both platforms — DONE 2026-07-30

`output/regression/verify-transcripts.sh` reran all 13 sims and checked every
transcript line in every README against fresh output. **251 lines, green on
Linux/aarch64 (Icarus 14.0 devel) and on Darwin/arm64 (Icarus 13.0 stable).**
Two different simulator versions agreeing character for character, so what you
are about to copy is a property of the framework rather than of one toolchain.

Copy them with confidence. If you ever need to re-check, that one command does it.

### Status of the three closing steps (updated 2026-07-30, code thread)

1. **Transcripts — DONE. All 13 markers are filled (2026-08-04).** A code
   thread ran the sims and pasted the output; there is nothing left to copy.
   The manuscript now carries 549 transcript lines and **every one of them is
   checked against a real run** by `verify-transcripts.sh`, which gates the
   push. Historical detail below. The sims
   were rerun and the READMEs for ch27, ch28, ch36, ch37, ch38 and ch39 were
   rebuilt: each now carries its transcripts verbatim under a "Transcript(s)"
   heading, labelled by figure number. Copy them into the 13
   `[TRANSCRIPT NEEDED]` markers character for character. The Interlude's and
   ch40's are in **`rustdv/tinyalu_tb/README.md`**, a new file written for
   exactly this. Nothing is waiting on Ray.
   **Those READMEs' figure maps were rebuilt too, in the book's numbering** —
   a row number is now the figure number. **ch15–21 were regenerated as well**,
   so no stale README is left: their `src/lib.rs` citations are repointed, and
   ch18/ch19/ch20's transcripts were 5ns early throughout (D112 — the DUT
   self-clocks, so the first edge lands later) and now carry real output. If
   ch18–20 prose quotes a time, re-check it.
2. **The mechanical caption pass — DONE 2026-07-30.** `renumbering-spec.md`
   applied to the `.rs` captions in ch27, ch28, ch31, ch32, ch34, ch36, ch37,
   ch39; every edit line-count-neutral, all eight crates rebuilt clean. Part I
   untouched, as specified. **Every `file:line` the manuscript embeds is still
   valid** — no transcript was invalidated by it.
3. **The end-to-end read — DONE 2026-07-30 (closing session).** mdbook builds
   with no warnings beyond the expected optional-pdf one. Checked against the
   rendered HTML: every chapter's figure sequence is 1..n in order (scripted
   over all captions: `# Figure N`, `// Chapter X, Figure N`,
   `<figcaption>`); no fenced-block leaks, no escaped SVG, no leftover
   markers; no reference to a chapter beyond 40; the `.rs` captions match the
   manuscript's numbering (spot-verified ch27 and ch39 against the applied
   spec); spot-read ch1, the Interlude, ch24 and Appendix D head-to-tail as
   rendered. One repo-side nit found and filed at the end of
   `chapter-notes.md`: ch27's README's last "proves" bullet contradicts its
   own Figure 13 transcript about who wins the conflict (the transcript and
   the book are right).

**Nothing is owed from the writer. The book is closed out:** all transcripts
pasted, the D114 sweep applied (including ch21/ch24's rewrites and the
retirement of `#[component(no_factory)]` from ch29), ch15–21 reconciled with
the regenerated READMEs (paths and the D112 +5ns times, including ch18's
prose "130 nanoseconds"), and the rendered book read. What can still happen
to this manuscript happens in other threads: Ray's own read, and any code
change that moves a transcript again.

### New since the pass closed: the child attribute lost its argument

**D114 (`output/.design-decisions.md` §42), 2026-07-30.**
`#[component(child)]`, `#[component(fifo)]` and `#[component(sequencer)]` are
gone from the code. The attribute is bare `#[component]`. The derive never read
the word — it tested only that one of the three was present — and what a field
becomes is decided by its Rust type. `AnalysisBus` declared `#[component(fifo)]`
was the wart that prompted reading the macro.

**The manuscript still prints the old form in 17 files:** the Interlude and
chapters 21, 24, 25, 26, 27, 28, 29, 30, 31, 32, 34, 36, 37, 38, 39, 40.
Fifteen are substitution inside code blocks. **Two are real work:**

- **ch21** teaches `#[derive(Component)]`. Any passage explaining what the
  argument selects has lost its subject.
- **ch24** introduces the attribute to the reader. `#[component]` marks a field
  as a child in the tree — that is now the whole rule and the whole syntax.

**Do not mention the change. The reader never knew the argument existed.** They
meet `#[component]` for the first time, so there is no before-and-after to
explain. The full note is at the head of Part II in `chapter-notes.md`.

The open items below and the per-chapter routine under Exemplars are the method
for any further prose work.

## Open questions for Ray

- ~~ch24 README stale note~~ **Closed by Ray, 2026-07-30: not an issue, and
  not to be raised again.** ch24's prose defers concurrency to ch31 and is
  true either way.
- ~~ch37/ch38 mismatch~~ **RESOLVED — and my first report was wrong.** The
  crates' .rs sources (the truth) contain exactly what chapter-notes said:
  ch37 = the repair desk (7.1, try_next_item, out-of-order, tickets), ch38 =
  Fibonacci with get_response (7.2). The stale READMEs had described an
  older swap and misled the first check. Per Ray's "code wins, fix the
  names": manuscript files renamed to chapter-37-repair-desk-testbench-7.1.md
  / chapter-38-fibonacci-testbench-7.2.md, SUMMARY retitled ("The Repair
  Desk: Testbench 7.1" / "Fibonacci Testbench: 7.2"), ch36's closing pointer
  fixed. Both chapters written from the crates.
