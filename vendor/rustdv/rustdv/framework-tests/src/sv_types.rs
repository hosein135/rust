//! Verilator VPI tests for real/string variables and unpacked aggregates.
use rustdv::prelude::*;
use rustdv::{SimHandle, ValueError};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[rustdv::test]
async fn sv_types_scalar_round_trip(ctx: RustdvCtx) -> Result<(), TestError> {
    let dut = ctx.dut();
    check!(
        matches!(dut.child("real_sig")?, SimHandle::Real(_)),
        "real type"
    );
    check!(
        matches!(dut.child("string_sig")?, SimHandle::String(_)),
        "string type"
    );
    check!(dut.signal("real_sig").is_err(), "real accepted as logic");
    check!(dut.real("string_sig").is_err(), "string accepted as real");
    let hierarchy = SimHandle::Hierarchy(dut).as_hierarchy()?;
    let real_object = hierarchy.child("real_sig")?;
    for (error, expected, actual) in [
        (real_object.as_hierarchy().err(), "module", "real"),
        (real_object.as_signal().err(), "signal", "real"),
        (real_object.as_string().err(), "string", "real"),
        (real_object.as_aggregate().err(), "struct or union", "real"),
        (
            hierarchy.child("string_sig")?.as_real().err(),
            "real",
            "string",
        ),
    ] {
        check!(
            matches!(error, Some(HandleError::WrongKind { name, expected: got, actual: kind })
            if name.ends_with("_sig") && got == expected && kind == actual),
            "wrong-kind conversion lost error details"
        );
    }
    check!(
        matches!(
            SimHandle::Other.as_signal(),
            Err(HandleError::WrongKind { .. })
        ),
        "Other converted to signal"
    );
    let real = dut.child("real_sig")?.as_real()?;
    let text = dut.child("string_sig")?.as_string()?;
    let precise = 1.0 + 2.0_f64.powi(-40);
    real.set_now(precise);
    check!(real.get()? == precise, "real lost f64 precision");
    let long_text = "long text λ ".repeat(100);
    text.set_now(&long_text)?;
    check!(text.get()? == long_text, "long UTF-8 string was truncated");
    real.set_now(-1.25);
    text.set_now("initial")?;
    check!(real.get()? == -1.25, "immediate real write");
    check!(text.get()? == "initial", "immediate string write");
    let saved = text.get()?;
    real.set(123.5);
    real.set(-0.0625);
    text.set(&"long text λ ".repeat(100))?;
    text.set("final λ")?;
    check!(
        real.get()? == -1.25 && text.get()? == "initial",
        "scheduled writes applied early"
    );
    read_write().await;
    check!(
        real.get()? == -0.0625 && text.get()? == "final λ",
        "last scheduled write did not win"
    );
    check!(saved == "initial", "string read did not own its data");
    text.set("")?;
    check!(text.get()?.is_empty(), "ReadWrite write was not immediate");
    check!(
        text.set("bad\0text") == Err(ValueError::InteriorNul),
        "scheduled NUL accepted"
    );
    check!(
        text.set_now("bad\0text") == Err(ValueError::InteriorNul),
        "immediate NUL accepted"
    );
    check!(text.get()?.is_empty(), "invalid string modified value");
    Ok(())
}

#[rustdv::test]
async fn sv_types_aggregate_members(ctx: RustdvCtx) -> Result<(), TestError> {
    let dut = ctx.dut();
    check!(
        matches!(dut.child("record_sig")?, SimHandle::Struct(_)),
        "struct type"
    );
    check!(
        matches!(dut.child("union_sig")?, SimHandle::Union(_)),
        "union type"
    );
    let record = dut.child("record_sig")?.as_aggregate()?;
    check!(record.child("missing").is_err(), "missing member found");
    check!(
        record.child("fraction")?.as_signal().is_err(),
        "real member accepted as logic"
    );
    let members = record.children();
    check!(
        members.len() == 3,
        "expected three struct members, got {}",
        members.len()
    );
    check!(
        members
            .iter()
            .filter(|h| matches!(h, SimHandle::Real(_)))
            .count()
            == 1,
        "real member missing"
    );
    check!(
        members
            .iter()
            .filter(|h| matches!(h, SimHandle::String(_)))
            .count()
            == 1,
        "string member missing"
    );
    let number = record.child("number")?.as_signal()?;
    let fraction = record.child("fraction")?.as_real()?;
    let text = record.child("text")?.as_string()?;
    number.set_u32(42);
    fraction.set(2.75);
    text.set("member value")?;
    let nested = dut
        .aggregate("nested_sig")?
        .child("inner")?
        .as_aggregate()?;
    nested.child("number")?.as_signal()?.set_u32(91);
    let union = dut.child("union_sig")?.as_aggregate()?;
    check!(union.children().len() == 2, "union members missing");
    let first = union.child("first")?.as_signal()?;
    let second = union.child("second")?.as_signal()?;
    first.set_u32(0x12345678);
    read_write().await;
    check!(number.get_u32()? == 42, "integer struct member");
    check!(fraction.get()? == 2.75, "real struct member");
    check!(text.get()? == "member value", "string struct member");
    check!(
        nested.child("number")?.as_signal()?.get_u32()? == 91,
        "nested member"
    );
    check!(
        second.get_u32()? == 0x12345678,
        "union members do not alias"
    );
    second.set_u32_now(0x87654321);
    check!(first.get_u32()? == 0x87654321, "reverse union alias");
    Ok(())
}

#[rustdv::test]
async fn sv_types_real_scheduled_readonly_rejected(ctx: RustdvCtx) -> Result<(), TestError> {
    let h = ctx.dut().real("real_sig")?;
    read_only().await;
    check!(
        catch_unwind(AssertUnwindSafe(|| h.set(1.0))).is_err(),
        "scheduled real write allowed in ReadOnly"
    );
    Ok(())
}
#[rustdv::test]
async fn sv_types_real_immediate_readonly_rejected(ctx: RustdvCtx) -> Result<(), TestError> {
    let h = ctx.dut().real("real_sig")?;
    read_only().await;
    check!(
        catch_unwind(AssertUnwindSafe(|| h.set_now(1.0))).is_err(),
        "immediate real write allowed in ReadOnly"
    );
    Ok(())
}
#[rustdv::test]
async fn sv_types_string_scheduled_readonly_rejected(ctx: RustdvCtx) -> Result<(), TestError> {
    let h = ctx.dut().string("string_sig")?;
    read_only().await;
    check!(
        catch_unwind(AssertUnwindSafe(|| h.set("illegal"))).is_err(),
        "scheduled string write allowed in ReadOnly"
    );
    Ok(())
}
#[rustdv::test]
async fn sv_types_string_immediate_readonly_rejected(ctx: RustdvCtx) -> Result<(), TestError> {
    let h = ctx.dut().string("string_sig")?;
    read_only().await;
    check!(
        catch_unwind(AssertUnwindSafe(|| h.set_now("illegal"))).is_err(),
        "immediate string write allowed in ReadOnly"
    );
    Ok(())
}
