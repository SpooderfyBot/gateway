#![allow(unused)]

pub type OpCode = usize;

pub const OP_STATS_UPDATE: OpCode = 0;
pub const OP_MESSAGE: OpCode = 5;
pub const OP_LIVE_READY: OpCode = 2;