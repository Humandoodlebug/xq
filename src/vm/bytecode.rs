use crate::vm::{Address, Result, ScopeId, ScopedSlot, Value};
use std::fmt::{Debug, Formatter};

#[derive(Clone)]
pub struct NamedFunction<F: Clone + ?Sized> {
    pub name: &'static str,
    pub func: F,
}

#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct Closure(pub(crate) Address);

pub type NamedFn1 = NamedFunction<Box<fn(Value) -> Result<Value>>>;
pub type NamedFn2 = NamedFunction<Box<fn(Value, Value) -> Result<Value>>>;

impl<F: Clone> Debug for NamedFunction<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Intrinsic {}", self.name))
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ByteCode {
    Unreachable,
    PlaceHolder,
    Nop,
    Push(Value),
    Pop,
    Dup,
    Swap,
    Const(Value),
    Load(ScopedSlot),
    Store(ScopedSlot),
    PushClosure(Closure),
    StoreClosure(ScopedSlot),
    Object,
    Append(ScopedSlot),
    Fork {
        fork_pc: Address,
    },
    ForkTryBegin {
        catch_pc: Option<Address>,
    },
    ForkTryEnd,
    ForkAlt,
    ForkLabel,
    Backtrack,
    Jump(Address),
    JumpUnless(Address),
    CallClosure {
        slot: ScopedSlot,
        return_address: Address,
    },
    Call {
        function: Address,
        return_address: Address,
    },
    // TODO: Tail recursion
    // CallRec,
    PushPC,
    CallPC,
    NewScope {
        id: ScopeId,
        variable_cnt: usize,
        closure_cnt: usize,
    },
    Ret,
    Output,
    Each,
    ExpBegin,
    ExpEnd,
    PathBegin,
    PathEnd,
    Intrinsic1(NamedFn1),
    Intrinsic2(NamedFn2),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub(crate) code: Vec<ByteCode>,
    pub(crate) entry_point: Address,
}

impl Program {
    pub(crate) fn fetch_code(&self, pc: Address) -> Option<&ByteCode> {
        self.code.get(pc.0)
    }
}
