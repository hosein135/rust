# Framework simulator tests

Run all Icarus tests with `bash rustdv/framework-tests/run.sh`, or select a
prefix such as `sig_`. Verilator's real/string and unpacked aggregate tests run
with:

```sh
SIM=verilator bash rustdv/framework-tests/run.sh sv_types_
```

The runner enables the `verilator-types` Cargo feature for Verilator. These tests
require a Verilator build that exposes unpacked structs and unions through VPI
(`vpiStructVar`, `vpiUnionVar`, and `vpiMember`); the older implementation that
only exposes scalar variables cannot run them. The regression entry is
`custom/sim-sv-types-verilator`.

## Typed variable access

`HierarchyHandle` provides `real(name)`, `string(name)`, `aggregate(name)`, and
`signal(name)`. Both hierarchy and aggregate handles provide `child(name)`.
Missing names return `HandleError`. `SimHandle` distinguishes `Real`, `String`, `Struct`, and `Union`,
with `as_hierarchy()`, `as_signal()`, `as_real()`, `as_string()`, and
`as_aggregate()` conversions returning `Result<H, HandleError>`. Wrong kinds
return `HandleError::WrongKind` with the name, expected kind, and actual kind.
`SimHandle::Other` has no object metadata, so its error uses `<unknown>` as the name.

```rust,ignore
let dut = ctx.dut();
let gain = dut.real("real_sig")?;
let label = dut.string("string_sig")?;
let record = dut.aggregate("record_sig")?;
gain.set(1.25);
label.set("ready")?;
record.child("number")?.as_signal()?.set_u32(42);
record.child("fraction")?.as_real()?.set(0.5);
record.child("text")?.as_string()?.set("member")?;
read_write().await;
assert_eq!(gain.get()?, 1.25);
assert_eq!(label.get()?, "ready");
```

Real and string handles have `get()`, scheduled `set()`, and immediate
`set_now()`. Scheduled writes own their data, preserve the order of distinct
handles, and keep the last value written to a handle. Writes during ReadWrite
apply immediately; both write paths reject ReadOnly. Real values use `f64`.
Strings are copied from VPI into owned Rust strings (invalid UTF-8 is replaced);
embedded NUL bytes are rejected before writes.

Structs and unions expose `child(name)` and `children()` for members, including
nested aggregates. Values are read and written through the member's typed
handle. Union members share storage. Packed aggregates exposed by the simulator
as `vpiReg` or `vpiBitVar` continue to use `LogicHandle`.

The `sv_types_` tests cover scalar type recognition and wrong-kind rejection,
f64 precision, empty/long/UTF-8 strings, owned reads, NUL rejection, deferred and
immediate writes, last-write-wins, all four ReadOnly guards, member enumeration,
nested members, mixed member types, missing members, and union aliasing.
