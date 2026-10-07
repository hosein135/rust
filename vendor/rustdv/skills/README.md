# rustdv Verification Skills

Three skills that turn "here's a spec and some RTL — verify it" into a
repeatable AI workflow. They chain:

1. **rtl-spec-analysis** — spec + RTL in, approved verification plan out.
2. **rustdv-testbench** — plan in, complete running rustdv testbench out.
3. **rustdv-verify-cover** — testbench in, mutation-checked verification
   and coverage report out.

Each directory holds a `SKILL.md`; the `*.skill` files are the same
directories zipped for one-click install (Claude: Settings > Capabilities,
or the "Save skill" button when presented in chat). Distilled from the
implementation of rustdv and *Rust for RTL Verification* — the gotchas in
these files (the two-idle-edge wait_idle rule, the macOS linking trio, the
mutation-check acceptance gate) were all learned the expensive way.
