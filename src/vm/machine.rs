use crate::{
    data_structure::{PStack, PVector},
    vm::{
        bytecode::NamedFunction,
        error::{RecoverableError, UnrecoverableError},
        intrinsic::truthy,
        Address, ByteCode, Program, Result, ScopeId, ScopedSlot, Value,
    },
};

#[derive(Debug, Clone)]
struct Scope {
    slots: PVector<Option<Value>>,
    closure_slots: PVector<Option<Address>>,
}

impl Scope {
    fn new(variable_cnt: usize, closure_cnt: usize) -> Self {
        Self {
            slots: std::iter::repeat(None).take(variable_cnt).collect(),
            closure_slots: std::iter::repeat(None).take(closure_cnt).collect(),
        }
    }
}

#[derive(Debug)]
struct Machine {
    env: Environment,
}

#[derive(Debug)]
struct Environment {
    program: Program,
    forks: Vec<State>,
}

#[derive(Debug, Clone)]
struct State {
    pc: Address,
    stack: PStack<Value>,

    scopes: PVector<(Option<Scope>, PStack<Scope>)>,
    scope_stack: PStack<(Address, ScopeId)>, // (pc, pushed_scope)
}

impl Environment {
    fn push_fork(&mut self, state: &State, new_pc: Address) {
        let mut new_state = state.clone();
        new_state.pc = new_pc;
        self.forks.push(new_state);
    }

    fn pop_fork(&mut self) -> Option<State> {
        self.forks.pop()
    }
}

impl State {
    fn new(pc: Address) -> Self {
        State {
            pc,
            stack: Default::default(),
            scopes: Default::default(),
            scope_stack: Default::default(),
        }
    }

    fn push(&mut self, item: Value) {
        self.stack.push(item)
    }

    fn pop(&mut self) -> Result<Value, UnrecoverableError> {
        self.stack.pop().ok_or(UnrecoverableError::PopEmptyStack)
    }

    fn dup(&mut self) -> Result<(), UnrecoverableError> {
        let value = self.pop()?;
        self.push(value.clone());
        self.push(value);
        Ok(())
    }

    fn slot(&mut self, scoped_slot: &ScopedSlot) -> Result<&mut Option<Value>, UnrecoverableError> {
        let scope = self
            .scopes
            .get_mut(scoped_slot.0 .0)
            .and_then(|(x, _)| x.as_mut())
            .ok_or(UnrecoverableError::UninitializedScope)?;
        scope
            .slots
            .get_mut(scoped_slot.1)
            .ok_or(UnrecoverableError::UnknownSlot)
    }

    fn push_scope(
        &mut self,
        scope_id: ScopeId,
        variable_cnt: usize,
        closure_cnt: usize,
        pc: Address,
    ) {
        if self.scopes.len() <= scope_id.0 {
            self.scopes.extend(
                std::iter::repeat_with(|| (None, PStack::new()))
                    .take(scope_id.0 - self.scopes.len() + 1),
            )
        }
        let (scope, stack) = &mut self.scopes[scope_id.0];
        if let Some(prev_scope) = scope.replace(Scope::new(variable_cnt, closure_cnt)) {
            stack.push(prev_scope);
        }
        self.scope_stack.push((pc, scope_id));
    }

    fn pop_scope(&mut self) -> Result<Address, UnrecoverableError> {
        let (pc, scope_id) = self
            .scope_stack
            .pop()
            .ok_or(UnrecoverableError::PopEmptyScope)?;
        let (current, prev) = self
            .scopes
            .get_mut(scope_id.0)
            .ok_or(UnrecoverableError::PopUnknownScope)?;
        if current.is_none() {
            return Err(UnrecoverableError::PopUnknownScope);
        }
        *current = prev.pop();
        Ok(pc)
    }
}

impl Machine {
    fn new(program: Program) -> Self {
        let state = State::new(program.entry_point);
        Self {
            env: Environment {
                program,
                forks: vec![state],
            },
        }
    }
}

impl Iterator for Machine {
    type Item = Result<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        match run_code(&mut self.env) {
            Ok(item) => item,
            Err(e) => Some(Err(e.into())), // TODO: fuse
        }
    }
}

fn run_code(env: &mut Environment) -> Result<Option<Result<Value>>, UnrecoverableError> {
    let mut state = if let Some(state) = env.pop_fork() {
        state
    } else {
        return Ok(None);
    };
    let err: Option<RecoverableError> = None;
    let mut call_pc: Option<Address> = None;
    while let Some(code) = env.program.fetch_code(state.pc) {
        use ByteCode::*;
        match code {
            Unreachable => {}
            PlaceHolder => {}
            Nop => {}
            Push(v) => {
                state.push(v.clone());
            }
            Pop => {
                state.pop()?;
            }
            Dup => {
                state.dup()?;
            }
            Const(v) => {
                state.pop()?;
                state.push(v.clone())
            }
            Load(scoped_slot) => {
                let value = state
                    .slot(scoped_slot)?
                    .as_ref()
                    .ok_or(UnrecoverableError::UninitializedSlot)?
                    .clone();
                state.push(value);
            }
            Store(scoped_slot) => {
                let value = state.pop()?;
                state.slot(scoped_slot)?.replace(value);
            }
            StoreClosure(_) => todo!("Implement {:?}", code),
            Object => todo!("Implement {:?}", code),
            Append(scoped_slot) => {
                let value = state.pop()?;
                let slot_item = state
                    .slot(scoped_slot)?
                    .as_mut()
                    .ok_or(UnrecoverableError::UninitializedSlot)?;
                match slot_item {
                    Value::Array(v) => {
                        v.push_back(value);
                    }
                    _ => {
                        return Err(UnrecoverableError::TypeMismatch(
                            "expected a array to append to, but was not an array",
                        ));
                    }
                }
            }
            Fork { fork_pc } => {
                let fork_pc = *fork_pc;
                env.push_fork(&state, fork_pc);
            }
            ForkTryBegin => todo!("Implement {:?}", code),
            ForkTryEnd => todo!("Implement {:?}", code),
            ForkAlt => todo!("Implement {:?}", code),
            ForkLabel => todo!("Implement {:?}", code),
            Backtrack => todo!("Implement {:?}", code),
            Jump(address) => {
                state.pc = *address;
                continue;
            }
            JumpUnless(address) => {
                let value = state.pop()?;
                if !truthy(value) {
                    state.pc = *address;
                    continue;
                }
            }
            PushClosure(_) => todo!("Implement {:?}", code),
            CallClosure => todo!("Implement {:?}", code),
            Call {
                function,
                return_address,
            } => {
                assert_eq!(call_pc.replace(*return_address), None);
                state.pc = *function;
                continue;
            }
            PushPC => todo!("Implement {:?}", code),
            CallPC => todo!("Implement {:?}", code),
            NewScope {
                id,
                variable_cnt,
                closure_cnt,
            } => {
                let return_address = call_pc
                    .take()
                    .expect("NewScope should be called after Call");
                state.push_scope(*id, *variable_cnt, *closure_cnt, return_address);
            }
            Ret => {}
            Output => {
                return if let Some(err) = err {
                    Ok(Some(Err(err.into())))
                } else {
                    let value = state.pop()?;
                    Ok(Some(Ok(value)))
                }
            }
            Each => todo!("Implement {:?}", code),
            ExpBegin => todo!("Implement {:?}", code),
            ExpEnd => todo!("Implement {:?}", code),
            PathBegin => todo!("Implement {:?}", code),
            PathEnd => todo!("Implement {:?}", code),
            Intrinsic1(NamedFunction { name: _name, func }) => {
                let arg = state.pop()?;
                state.push(func(arg));
            }
            Intrinsic2(NamedFunction { name: _name, func }) => {
                let rhs = state.pop()?;
                let lhs = state.pop()?;
                state.push(func(lhs, rhs));
            }
        }
        state.pc.next();
    }
    Ok(None)
}
