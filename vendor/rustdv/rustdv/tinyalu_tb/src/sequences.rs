//! Sequences: the stimulus, as a program.
//!
//! A sequence is not a component — no place in the tree, no path, no phases.
//! One method, `body`, and a `SeqCtx` to run it against.
//!
//! The gap between `start_item` and `finish_item` is the point: `start_item`
//! returns when the sequencer has granted this item its turn and the driver is
//! blocked waiting for its contents, so everything in between happens with the
//! driver committed and holding still. That is where late stimulus setting
//! lives, and it is why the rendezvous has two calls rather than one.
//!
//! All three sequences differ only in how they fill the operands, so the walk
//! over the operations lives in one function and each sequence supplies a
//! `set_operands`. `BaseSeq` is the type a test names; the factory substitutes
//! one of the others for it (D80/D96).

use rustdv::prelude::*;

use crate::alu_item::{AluCommand, AluResult, Ops};

/// How the operands get filled, once the driver is committed.
trait Operands {
    fn set_operands(&mut self, rng: &mut Rng, cmd: &mut AluCommand);
}

/// Every operation, `n` times each — the walk all three sequences share.
async fn all_ops<S: Operands>(
    seq: &mut S,
    ctx: &mut SeqCtx<AluCommand, AluResult>,
    n: usize,
) -> Result<(), SeqError> {
    let mut rng = ctx.rng();
    for _ in 0..n {
        for op in Ops::ALL {
            let mut cmd = AluCommand { a: 0, b: 0, op };
            ctx.start_item(&mut cmd).await?;
            // Late generation: the driver is waiting, so decide now.
            seq.set_operands(&mut rng, &mut cmd);
            // Ownership moves to the driver here. A sequence that needed the
            // command afterward would clone it first; this one does not.
            ctx.finish_item(cmd).await?;
        }
    }
    Ok(())
}

/// The type a test asks for. On its own it drives zeros, which is a legal
/// stimulus and a poor one — its job is to be the name the factory overrides.
#[derive(Default)]
pub struct BaseSeq;

impl Operands for BaseSeq {
    fn set_operands(&mut self, _rng: &mut Rng, _cmd: &mut AluCommand) {}
}

impl Sequence for BaseSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        all_ops(self, ctx, 1).await
    }
}

/// Random operands across every operation, five times each. Coverage is
/// guaranteed by construction rather than hoped for.
#[derive(Default)]
pub struct RandomSeq;

impl Operands for RandomSeq {
    fn set_operands(&mut self, rng: &mut Rng, cmd: &mut AluCommand) {
        cmd.a = rng.u8();
        cmd.b = rng.u8();
    }
}

impl Sequence for RandomSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        all_ops(self, ctx, 5).await
    }
}

/// Maximum operands for every operation: `0xff op 0xff`, once each. The corner
/// the random test is unlikely to reach on its own.
#[derive(Default)]
pub struct MaxSeq;

impl Operands for MaxSeq {
    fn set_operands(&mut self, _rng: &mut Rng, cmd: &mut AluCommand) {
        cmd.a = 0xFF;
        cmd.b = 0xFF;
    }
}

impl Sequence for MaxSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        all_ops(self, ctx, 1).await
    }
}
