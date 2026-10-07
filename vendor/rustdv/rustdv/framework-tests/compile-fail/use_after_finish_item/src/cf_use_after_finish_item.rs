//! Using a transaction after `finish_item` has taken it must not compile.
//!
//! The book makes this claim twice, and it is the one a reader is most
//! likely to walk into: in SystemVerilog and in Python the sequence still
//! holds a reference to the item after handing it over, so reading it back
//! is legal and the answer is whatever the driver has done to it since.
//! Chapter 37's answer to "what if I want the item afterwards" is *clone it
//! before you hand it over*, and that answer only makes sense if this is a
//! compile error.
//!
//! `finish_item(cmd)` takes the transaction **by value**. After the move
//! there is no `cmd` to read.

use rustdv::prelude::*;

#[derive(Clone, Debug, Default)]
struct AluCommand {
    a: u8,
    b: u8,
}

#[derive(Clone, Debug, Default)]
struct AluResult {
    result: u16,
}

#[derive(Default)]
struct BadSeq;

impl Sequence for BadSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        let mut cmd = AluCommand { a: 1, b: 2 };
        ctx.start_item(&mut cmd).await?;
        cmd.a = 7;
        ctx.finish_item(cmd).await?;

        // The transaction is the driver's now. Reading it here would be
        // reading a value someone else owns and may already have changed.
        //
        // It has to be a real read: `let _ = cmd.a;` compiles, because `let _`
        // does not evaluate a place expression at all. That is a genuine Rust
        // subtlety and it is why this line prints instead of discarding.
        println!("the operand was {}", cmd.a);
        Ok(())
    }
}
