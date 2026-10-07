# Renaming the project: name candidates and impact analysis

Prepared 2026-07-08, before any changes. Two distinct names are being
replaced: **rustuvm** (the repo/project) and **rustvm** (the framework the
book describes — which collides with existing Rust virtual-machine projects
and, per this analysis, invites "VM = virtual machine" misreading).

---

## Part 1 — Name candidates

### How Rust projects get named (the landscape)

Four patterns dominate: **oxidation metaphors** (Oxide, redox, oxidized —
chemistry of rust), **iron/metallurgy words** (ferrous, ferrite, corrosion,
Ferris the mascot), **rust- prefixes / -rs suffixes** (rust-analyzer,
tokio-rs), and **plain coined names**. One caution specific to this project:
the phrase "Rust verification" is already owned by a crowded field of tools
that formally verify Rust *software* — Verus (Microsoft), Kani (AWS),
Creusot, Prusti, Crucible/crux-mir (Galois). A name should signal *hardware*
DV — bench/DV/RTL — or it will be shelved with the wrong tools.

A second caution, learned from rustvm itself: avoid a bare `vm` suffix.
UVM's "VM" means Verification Methodology, but the software world reads
"virtual machine" every time.

### Checked and rejected

| Name | Why not |
|---|---|
| **rvm** (your suggestion) | Ruby Version Manager — rvm.io, a Wikipedia-notable tool that has owned the `rvm` command since ~2009. Unusable. Also reads as "Rust VM." |
| rustvm | Existing Rust virtual-machine repos; VM misreading |
| patina | Microsoft-backed Rust UEFI firmware (crates.io v14.x, active) |
| assay | Rust testing-macro crate, 74K downloads |
| hematite | Piston's Minecraft client (30 releases) |
| crucible | Galois's verification framework — direct domain collision |
| rustbench | Benchmarking crate (2025) |
| quench | A programming language (crates.io) |
| attest | Test framework, actively published 2026 |
| sequent | Discrete-event simulation library — domain-adjacent |
| vermilion | Squatted on crates.io (ironically, for a rust-paint pun) |
| alloy, verdi, calibre, verus, kani | Existing EDA/formal-verification tools — never reuse these |

### The ten candidates (all free on crates.io as of 2026-07-08)

| # | Name | Reading | Notes |
|---|---|---|---|
| 1 | **rustdv** | Rust + DV (design verification) | **Recommended.** DV is *the* industry term — your audience parses it instantly. Short, pronounceable ("rust-dee-vee"), no VM confusion, no formal-verification confusion, honest about being Rust. |
| 2 | **veriron** | VERIfication + IRON | Brandable, evokes hardware + rust chemistry. Risk: could scan as "very-ron." |
| 3 | **oxidv** | OXIde + DV | The oxidation pattern applied to DV. Pronunciation ambiguous ("ox-id-vee"? "oxi-dee-vee"?). |
| 4 | **ferrodv** | FERRO (iron) + DV | Clear on both axes; slightly long. |
| 5 | **rusttb** | Rust + TB (testbench) | TB is as recognizable as DV to this audience; double-t seam is the only wart. |
| 6 | **veribench** | VERIfication + testBENCH | Fully self-describing; longest of the set; "bench" alone can read as benchmarking. |
| 7 | **oxbench** | OXide + BENCH | Punchy; same benchmarking ambiguity. |
| 8 | **veriforge** | VERIfy + FORGE | Forge = where iron is worked; "forge" is a crowded metaphor generally but this compound is free. |
| 9 | **verox** | VERification + OXide | Sleek and brandable, but a small AI startup (verox-ai/verox on GitHub) surfaced in 2025-26; not disqualifying, not clean. |
| 10 | **verist** | "one devoted to truth/realism" (a real word) | Beautiful meaning for verification; same caveat — verist-ai exists on GitHub. |

Checks performed: crates.io registry API (authoritative for the future
crate name), web/GitHub search for the finalists. Before committing, spend
five minutes confirming the winner: the GitHub repo name search, a plain
web search, and — if you ever want them — domain/social handles. Consider
publishing a 0.0.1 placeholder crate immediately after choosing, to reserve
the name.

---

## Part 2 — Implications of removing "uvm" from the codebase

### The two renames, and what stays

- **rustuvm → new name** (repo identity): the GitHub repo (not yet created —
  ideal timing; no redirects to manage), your local folder name, README
  badge URLs and Codespaces link (the `OWNER/rustuvm` placeholders), the
  devcontainer display name, the `docker build -t rustuvm` instruction.
- **rustvm → new name** (the framework the book teaches): this is the big
  one — **35 occurrences across 10 of 14 chapters** of the manuscript, not
  just the two preview figures.
- **What stays:** UVM as acknowledged inspiration (10 mentions) and pyuvm
  references (35) in the book prose — "inspired by UVM" is history, not
  branding. The `reference/` directory is untouched. Structural vocabulary
  (env, component, test, sequence, driver, monitor, scoreboard) is generic
  verification language, not UVM property — keep it, without `uvm_` prefixes.

### Ripple effects, in dependency order

1. **Book source** (`book-pdf/src/`): 35 rustvm occurrences. Two figure
   *titles* change ("The shape of rustvm's Component trait", "The shape of
   rustvm's driver") — retitling figures changes downstream names.
2. **Examples tree**: two fragment *filenames* embed the name
   (`fig04_shape_rustvm_s_component.rs`, `fig06_shape_rustvm_s_driver.rs`)
   plus their contents/headers; `manifest.json` titles and file paths;
   ch10/ch11 README tables. Regenerate Playground links after.
3. **Regression system**: `regress.json` comment, `TESTING.md`,
   `tests/example-template/test.json` (its example uses package name
   `rustvm` and path `output/rustvm`), sim-lint test comment. No goldens
   change (fragments have none) and no golden re-bless is needed.
4. **Docs/infra**: root README (framework mentions + repo placeholders),
   `sim/README.md`, `docker/Dockerfile` comment, devcontainer name.
5. **Planning docs**: `output/.design-doc.md` and `output/book-outline.md`
   both reference rustvm throughout.
6. **Rebuilds**: `mdbook build` and PDF regeneration after the manuscript
   edit.

### The safety net (why this rename is low-risk)

The regression system polices its own rename: `book-sync` fails on any
figure where book and example disagree, the coverage test fails if a
file path in the manifest goes stale, and the titles test catches renamed
figures. Procedure: change book + examples + manifest together, run
`regress.py --suite book-sync` until green, then sweep docs with a
case-insensitive grep for `rustvm|rustuvm` (expected survivors: UVM/pyuvm
history mentions and this file).

### Operational cautions

- **Rename the local folder last** (or create the GitHub repo under the new
  name and re-clone): this Cowork session's folder connection points at the
  `rustuvm` path, and the folder name itself is the last uvm trace.
- The pre-push git hook uses `git rev-parse --show-toplevel`, so it survives
  a folder rename untouched.
- Choose the GitHub repo name = crate name = folder name; one name
  everywhere.

### Open questions

- Chapter 10/11 preview-figure prose says things like "the shape of
  <name>'s Component trait" — sentences read fine with any candidate
  substituted, but you may want to re-voice them once the name is real.
- Whether the book's *title-page framing* ("we build <name>, inspired by
  UVM but rethought for Rust") deserves a paragraph of its own is an
  authorial call this analysis doesn't make.
