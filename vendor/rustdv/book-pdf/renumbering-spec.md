# Renumbering spec — reconciling `.rs` captions to the manuscript's figures

> **Historical: this work order was applied and is finished.** It is kept as the
> record of what moved and why, not as a task. One section is also out of date:
> **ch37's example was replaced on 2026-08-05** and its crate is now
> `ch37-out-of-order-transaction-testbench-7.1`, with its own figure map in that
> crate's `README.md`. Take ch37 figures from there, never from the ch37 section
> below.

Written at the end of the prose pass (2026-07-30). For each chapter the pass
changed, this lists the figure numbers the prose now uses, mapped to the code
they correspond to, and flags every `.rs` caption whose number must move.
A mechanical pass applies the caption edits; nothing here touches prose.

## Constraints for whoever runs this

1. **Edits must be in place and line-count-neutral.** Transcripts embed
   `file:line` (e.g. `[ch25-.../src/ch25_uvm_env_testbench_4_0.rs:256]`), so
   a caption edit that adds or removes a line invalidates every transcript in
   that crate. Change digits inside existing comment lines only.
2. **Part I (ch1–14) numbering feeds `manifest.json` and `book-sync`.**
   Nothing in Part I moved: ch1–14 figure numbers are unchanged by this pass,
   so `book-sync` and the manifest need no edits. (ch1's figs 1–2, and all
   Part I listings, are byte-identical to HEAD.)
3. Transcript regeneration must happen **after** any file renames, never
   before (log lines embed paths). The ch15–21 READMEs are already stale in
   this respect — see HANDOFF.md "Transcripts owed."

## Chapters with NO `.rs` caption changes

- ch1 (figs 1–2 unchanged), all of ch2–ch14, Interlude (excerpts carry no
  captions), Toolkit page, ch15–ch22 (listings untouched by this pass),
  ch23 (book figs 1–7 land exactly in the crate's slots; transcripts fill
  2/6/7, tower diagram fills 3), ch25 (figs 1–10 = crate 1–10; transcript
  appended as 11), ch26 (figs 1–7 = crate; transcripts 8–9 appended),
  ch29 (crate left gaps 3/5/9/11/14 for transcripts; book fig 16 appended —
  no existing caption moves), ch30 (figs 1–5 = crate; transcript 6 appended),
  ch33 (crate's "Chapter 33, Figure 1–6" unchanged), ch35 (figs 1–7 = the
  seven bins), ch38 (figs 1–5 = crate; transcript 6 appended), ch40 (the
  tinyalu_tb crate carries no figure captions; the book's numbering is its
  own).

## Chapters whose `.rs` captions must change

### ch24 — `output/examples/ch24-components/src/ch24_components.rs`

No caption *numbers* move (crate 1, 4, 5, 6 keep their numbers; the book's
figs 2, 7 are transcripts and fig 3 is a book-only drawing filling the
crate's deliberate gap). **No edit needed** — listed here only because the
crate's numbering has gaps a future reader might "fix"; do not.

### ch27 — `output/examples/ch27-configuration/src/ch27_configuration.rs`

Book inserted four transcript figures. Crate→book: 1→1, 2→2, 3→3, 4→**5**,
5→**6**, 6→**8**, 7→**9**, 8→**11**, 9→**12**. Edit the captions for crate
figs 4–9 accordingly (book figs 4, 7, 10, 13 are transcripts).

### ch28 — `output/examples/ch28-config-debugging/src/ch28_config_debugging.rs`

Crate→book: 1–6 unchanged, 7→**8**, 8→**9**, 9→**11**. (Book figs 7, 10, 12
are transcripts/dump excerpts.)

### ch31 — `output/examples/ch31-component-communications/src/ch31_component_communications.rs`

Crate→book: 1→1, 2→2, 3→3, 4→**5**, 5→**6**, 6→**7**, 7→**9** (the pipeline
comment block — now rendered as the book's SVG drawing, fig 9), 8→**10**,
9→**11**, 10→**12**, 11→**14**, 12→**16**. (Book figs 4, 8, 13, 15, 17 are
transcripts.)

### ch32 — `output/examples/ch32-analysis-ports/src/ch32_analysis_ports.rs`

Crate→book: 1–4 unchanged, 5→**6**, 6→**8**, 7→**9**. (Book figs 5, 7, 10
are transcripts.)

### ch34 — `output/examples/ch34-connections-testbench-6.0/src/ch34_connections_testbench_6_0.rs`

Only the two "Chapter 34" captions: Figure 1→**2** (AluEnv), Figure 2→**3**
(AluTest). Book fig 1 is the SVG architecture drawing; fig 4 the transcript.
The "Chapter 33" captions in the same file are untouched.

### ch36 — `output/examples/ch36-sequence-testbench-7.0/src/ch36_sequence_testbench_7_0.rs`

All captions shift by one: crate 1→**2**, 2→**3**, 3→**4**, 4→**5**, 5→**6**,
6→**7**, 7→**8**. Book fig 1 is the SVG handshake drawing; fig 9 the
transcript. The uncaptioned AluCommand/AluResult re-show stays uncaptioned.

### ch37 — `output/examples/ch37-repair-desk-testbench-7.1/src/ch37_repair_desk_testbench_7_1.rs`

Crate figs 1, 2, 3 unchanged. Crate "Figure 5" (RepairTest) → **4** (the
book's fig 4 covers ShopEnv + RepairTest). The crate's "Figure 4 is a
paragraph" comment should be reworded to drop the figure number (it is prose
in the book, not a numbered figure) — if that rewording cannot be made
line-count-neutral, leave it and note the discrepancy.

### ch39 — `output/examples/ch39-virtual-sequence-testbench-8.0/src/ch39_virtual_sequence_testbench_8_0.rs`

Presentation reorders the crate. Crate→book: 1→**1**, 2→**4**, 3→**6**,
4→**7**, 5→**8**, 6→**2**. (Book figs 3, 5, 9 are transcripts.)
ParallelTest/FibonacciProgramTest listings remain uncaptioned in the crate
and unlisted in the book.

## Stale READMEs (regeneration list, not caption edits)

ch15–ch21 (cite `src/lib.rs`, transcripts embed pre-rename paths), ch27,
ch36, ch37, ch38, ch39 (old figure maps, missing transcripts). After
regeneration, paste the new transcripts into the manuscript at the
`[TRANSCRIPT NEEDED]` markers and re-copy any transcript whose `file:line`
changed (HANDOFF.md "Transcripts owed" is the checklist).
