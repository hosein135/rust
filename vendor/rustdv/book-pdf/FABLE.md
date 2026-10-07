# Brief for the prose pass
# Notes for RustDV Book.

# Chapter 32 — the code has landed; the prose is yours

> **Done 2026-08-05.** Verified against the manuscript, not this note: no
> `WriteSink`/`on_write`/`analysis_fifo` anywhere in `book-pdf/src`, the tap
> section is ch32 Figures 11–13, and the ch33/ch34 paragraphs use the new
> vocabulary. Kept for the record; nothing below is outstanding.

**The code thread ran on 2026-08-05 (D116/D117). You are not blocked.** Both
checkers are green — `verify-book-listings.py` reports 0 drift and
`verify-transcripts.sh` passes — so **every code block and every transcript in
ch32 is already correct**. Do not re-paste listings; they came from the crate.
What is left is the vocabulary in the running prose around them, which the code
thread deliberately did not touch.

What changed under you, and what it obliges:

* `WriteSink` is now `Subscriber`; `on_write()` is now `subscribe()`. The
  listings say so; the prose does not. Still carrying the retired names in
  ch32: the section headings `## WriteSink: what an arriving item does` and
  `## on_write and connect`, the paragraphs beneath them, and the Summary.
  One paragraph each in ch33 and ch34 too. Rewrite them in the new vocabulary:
  **the plain struct with `write()` is the subscriber; a component hosts it.**
  The parent `connect`s (which stream); the component `subscribe`s (which
  receiver) — keep that distinction sharp, it is the whole reason the two
  names differ. Stop calling the hosting component "the subscriber", and say
  in one sentence that `uvm_subscriber` is a component while rustdv's
  `Subscriber` is plain data — the same lesson the chapter already teaches
  about where storage lives.
* No identifier is named `analysis_fifo`. The crate's bus fields are `bus` and
  Figures 4, 6 and 9 already show it. (`uvm_tlm_analysis_fifo` stays wherever
  it appears — that is UVM's class name, and the contrast is the point.)
* The `TlmFifo` tap demonstration has arrived from ch31 and **is in the crate
  but not yet in the chapter.** It is ch32 Figures 11–13: `TapLog`/`TapWatcher`
  (11), `FifoTapTest` (12), and the transcript ending `tap saw [0, 1, 2]` (13),
  all in `output/examples/ch32-analysis-ports/` — the README figure map lists
  them and its transcript is the real run. This is the one place you are
  writing new prose rather than repairing old. Teach it as the port of
  `uvm_tlm_fifo`'s built-in analysis ports, `TlmFifo::put_ap()` /
  `TlmFifo::get_ap()`: the data path is still a queue — one consumer takes
  each item, the producer blocks when full — while the taps are observation
  alongside; every subscriber sees every item, nothing is consumed, nobody is
  delayed. The section belongs at the end, once subscribers are understood.
  `FifoTapTest` reuses Chapter 31's `Producer` and `Consumer` verbatim; they
  are in the crate **uncaptioned on purpose**, so refer back to Chapter 31
  rather than reprinting put/get inside the analysis chapter.
* ch31 needs nothing. Its tap section is gone, its transcripts were
  regenerated at four tests, and it keeps one forward sentence naming
  `put_ap()`/`get_ap()` and deferring to Chapter 32.

If you change a listing for any reason, re-run
`python3 output/regression/verify-book-listings.py` and
`bash output/regression/verify-transcripts.sh` — they gate the push.


# Final step

The book is ready to go to market.  Make one final complete sweep of the book to make final edits for its publication.

> **Done 2026-08-05.** The sweep ran: TOC titles, appendix cross-references
> and chapter pointers checked against the crates, spelling and
> figure-reference conventions unified. Regression, both book checkers and
> the mdBook HTML build green on Linux; macOS confirmation is Ray's.


---

## The absolute rule: you change no code

You edit **`book-pdf/src/*.md`**, `book-pdf/src/SUMMARY.md`, and the planning
files this brief asks you to create. Nothing else.

Off limits, no exceptions: `rustdv/`, `output/`, `sim/`, `skills/`,
`toolchain-drop/`, any `.rs`, `.toml`, `.json`, `.sh`, `.py`, or HDL file.

**When the code and the chapter disagree, the code wins.** The examples are
verified running; the manuscript is known stale. If an example looks wrong, or
a chapter needs a figure that does not exist — **stop and tell Ray.** Do not
edit the example to match your prose. That is the failure this whole project
exists to undo: prose driving design.

**Transcripts are copied, never composed.** Take them verbatim from the
chapter's `output/examples/*/README.md`. Do not retype, reformat, tidy, invent,
or regenerate them, and do not run a simulator. A transcript that looks wrong
is something you report.


---

## Voice

The model is *Python for RTL Verification*, in `../rustdv-reference/salemi_books/`.
Read enough of it to absorb how it teaches, and write like that. What follows is
not a style guide — you have the source — it is the two or three places this book
must differ from it.

**The reader.** That book was written for Python engineers. This one addresses a
verification engineer who knows the UVM from *either* SystemVerilog or Python,
with reading ability in both and no prior book. So nothing may assume the reader
has run cocotb or pyuvm, or has read the earlier books. Naming them as sources is
fine; leaning on shared memory of them is not.

**The reader has no history with rustdv, and rustdv has no history worth
telling.** Never explain the present design by contrast with an earlier design,
an earlier draft, an earlier name, or "a previous version of the testbench"
(the reader's own TB 2.0–8.0 climb is fine — that is their history). Explain
what happens now, plainly, as if it were always so.

**Comparisons are foils, not nostalgia.** "In Python, a typo'd attribute is a
runtime `AttributeError`" is useful. "As you saw in the Python book" is not.
Prefer two sharp comparisons to none — a dual audience means both foils, not no
foil — and where the dialects diverge, one parenthetical, SystemVerilog first:
*(SV: `uvm_config_db#(int)::set`; pyuvm: `ConfigDB().set`)*.

**SystemVerilog quotations** come only from `../rustdv-reference/uvmprimer-master/`,
run ≤15 lines, carry a source label, never appear with simulation output, and
total a handful across the whole book.

**Do not celebrate the compiler.** No "promise kept", no "the seam at work", no
selling types. State what a mechanism does and move on; where a compile error
is real, show it and let it speak.

**Say the point plainly.** "Honestly", "genuinely" and "straightforward" read as
persuasion rather than statement.

**Answer, then stop.** When Ray asks a question, give the verdict and the one
or two reasons that decide it. No essays, no surveys of alternatives he did not
ask for, no narration between steps of a task.


## When you are unsure

Ask Ray. Do not invent an answer and write it as settled — that is precisely how
the original design failed. This applies to anything in `chapter-notes.md`
marked **ASK RAY**, to the caption question above, and to any place a chapter
needs a fact the examples do not contain.

Reference material — cocotb, pyuvm, four releases of the SystemVerilog UVM, and
the example code from both earlier books — is outside the repo at
`../rustdv-reference`, read-only. Use it to check what the UVM actually does
rather than what a comment says it does.
