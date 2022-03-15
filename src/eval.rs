use monoasm::DestLabel;

use super::hir::SsaReg;
use super::*;

pub struct Evaluator {
    codegen: Codegen,
    mir_context: MirContext,
    mcir_context: McIrContext,
    jit_state: HashMap<usize, JitState>,
}

enum JitState {
    Fail,
    Success(DestLabel, Type),
}

macro_rules! value_op {
    ($lhs:ident, $rhs:ident, $op:ident) => {{
        match ($lhs, $rhs) {
            (Value::Integer($lhs), Value::Integer($rhs)) => $lhs.$op(&$rhs),
            (Value::Integer($lhs), Value::Float($rhs)) => ($lhs as f64).$op(&$rhs),
            (Value::Float($lhs), Value::Integer($rhs)) => $lhs.$op(&($rhs as f64)),
            (Value::Float($lhs), Value::Float($rhs)) => $lhs.$op(&$rhs),
            _ => unreachable!(),
        }
    }};
}

impl Evaluator {
    fn new() -> Self {
        Self {
            codegen: Codegen::new(),
            mir_context: MirContext::new(),
            mcir_context: McIrContext::new(),
            jit_state: HashMap::default(),
        }
    }

    fn jit_compile(
        &mut self,
        hir_context: &HirContext,
        hir_id: usize,
        args: &[Value],
    ) -> Option<(DestLabel, Type)> {
        let hir_func = &hir_context.functions[hir_id];
        let args = hir_func
            .args
            .iter()
            .zip(args.iter())
            .map(|(name, val)| (name.clone(), val.ty()))
            .collect();
        let mir_id =
            match self
                .mir_context
                .new_func_from_ast(hir_func.name.clone(), args, &hir_func.ast)
            {
                Ok(id) => id,
                Err(err) => {
                    dbg!(err);
                    self.mir_context.functions.pop().unwrap();
                    return None;
                }
            };
        dbg!(&self.mir_context);
        let func_id = self
            .mcir_context
            .from_mir(&self.mir_context.functions[mir_id]);
        Some(
            self.codegen
                .compile_func(&self.mcir_context.functions[func_id], func_id),
        )
    }

    pub fn eval_toplevel(hir_context: &HirContext) -> Value {
        let mut eval = Self::new();
        eval.eval_function(hir_context, 0, &[])
    }

    fn eval_function(&mut self, hir_context: &HirContext, cur_fn: usize, args: &[Value]) -> Value {
        match self.jit_state.get(&cur_fn) {
            None => {
                if let Some((dest, ty)) = self.jit_compile(hir_context, cur_fn, args) {
                    eprintln!("JIT success");
                    self.jit_state.insert(cur_fn, JitState::Success(dest, ty));
                    eprintln!("call JIT");
                    return self.codegen.run(dest, ty, args);
                } else {
                    eprintln!("JIT failed");
                    self.jit_state.insert(cur_fn, JitState::Fail);
                }
            }
            Some(JitState::Fail) => {}
            Some(JitState::Success(dest, ty)) => {
                eprintln!("call JIT");
                return self.codegen.run(*dest, *ty, args);
            }
        }

        let func = &hir_context.functions[cur_fn];
        let locals_num = func.locals.len();
        let mut locals = vec![Value::Nil; locals_num];
        locals[0..args.len()].clone_from_slice(args);
        let register_num = func.register_num();
        let mut eval = FuncContext {
            ssareg: vec![Value::Nil; register_num],
            locals,
            cur_bb: func.entry_bb,
            prev_bb: 0,
            pc: 0,
        };
        loop {
            let bb = &hir_context[eval.cur_bb];
            let op = &bb.insts[eval.pc];
            eval.pc += 1;
            if let Some(val) = self.eval(&mut eval, hir_context, op) {
                return val;
            }
        }
    }

    fn eval(
        &mut self,
        ctx: &mut FuncContext,
        hir_context: &HirContext,
        hir: &Hir,
    ) -> Option<Value> {
        match hir {
            Hir::Integer(ret, i) => {
                ctx[*ret] = Value::Integer(*i);
            }
            Hir::Float(ret, f) => {
                ctx[*ret] = Value::Float(*f);
            }
            Hir::Nil(ret) => {
                ctx[*ret] = Value::Nil;
            }
            Hir::Neg(op) => {
                let src = ctx.eval_operand(&op.src);
                ctx[op.ret] = match src {
                    Value::Integer(i) => Value::Integer(-i),
                    Value::Float(f) => Value::Float(-f),
                    _ => unreachable!(),
                };
            }
            Hir::Add(op) => {
                let lhs = ctx.eval_operand(&op.lhs);
                let rhs = ctx.eval_operand(&op.rhs);
                ctx[op.ret] = match (lhs, rhs) {
                    (Value::Integer(lhs), Value::Integer(rhs)) => Value::Integer(lhs + rhs),
                    (Value::Integer(lhs), Value::Float(rhs)) => Value::Float(lhs as f64 + rhs),
                    (Value::Float(lhs), Value::Integer(rhs)) => Value::Float(lhs + rhs as f64),
                    (Value::Float(lhs), Value::Float(rhs)) => Value::Float(lhs + rhs),
                    _ => unreachable!(),
                };
            }
            Hir::Sub(op) => {
                let lhs = ctx.eval_operand(&op.lhs);
                let rhs = ctx.eval_operand(&op.rhs);
                ctx[op.ret] = match (lhs, rhs) {
                    (Value::Integer(lhs), Value::Integer(rhs)) => Value::Integer(lhs - rhs),
                    (Value::Integer(lhs), Value::Float(rhs)) => Value::Float(lhs as f64 - rhs),
                    (Value::Float(lhs), Value::Integer(rhs)) => Value::Float(lhs - rhs as f64),
                    (Value::Float(lhs), Value::Float(rhs)) => Value::Float(lhs - rhs),
                    _ => unreachable!(),
                };
            }
            Hir::Mul(op) => {
                let lhs = ctx.eval_operand(&op.lhs);
                let rhs = ctx.eval_operand(&op.rhs);
                ctx[op.ret] = match (lhs, rhs) {
                    (Value::Integer(lhs), Value::Integer(rhs)) => Value::Integer(lhs * rhs),
                    (Value::Integer(lhs), Value::Float(rhs)) => Value::Float(lhs as f64 * rhs),
                    (Value::Float(lhs), Value::Integer(rhs)) => Value::Float(lhs * rhs as f64),
                    (Value::Float(lhs), Value::Float(rhs)) => Value::Float(lhs * rhs),
                    _ => unreachable!(),
                };
            }
            Hir::Div(op) => {
                let lhs = ctx.eval_operand(&op.lhs);
                let rhs = ctx.eval_operand(&op.rhs);
                ctx[op.ret] = match (lhs, rhs) {
                    (Value::Integer(lhs), Value::Integer(rhs)) => Value::Integer(lhs / rhs),
                    (Value::Integer(lhs), Value::Float(rhs)) => Value::Float(lhs as f64 / rhs),
                    (Value::Float(lhs), Value::Integer(rhs)) => Value::Float(lhs / rhs as f64),
                    (Value::Float(lhs), Value::Float(rhs)) => Value::Float(lhs / rhs),
                    _ => unreachable!(),
                };
            }
            Hir::Cmp(kind, op) => {
                let lhs = ctx.eval_operand(&op.lhs);
                let rhs = ctx.eval_operand(&op.rhs);
                ctx[op.ret] = Value::Bool(match kind {
                    CmpKind::Eq => value_op!(lhs, rhs, eq),
                    CmpKind::Ne => value_op!(lhs, rhs, ne),
                    CmpKind::Lt => value_op!(lhs, rhs, lt),
                    CmpKind::Gt => value_op!(lhs, rhs, gt),
                    CmpKind::Le => value_op!(lhs, rhs, le),
                    CmpKind::Ge => value_op!(lhs, rhs, ge),
                });
            }
            Hir::CmpBr(kind, lhs, rhs, then_, else_) => {
                let lhs = ctx[*lhs].clone();
                let rhs = ctx.eval_operand(rhs);
                let b = match kind {
                    CmpKind::Eq => value_op!(lhs, rhs, eq),
                    CmpKind::Ne => value_op!(lhs, rhs, ne),
                    CmpKind::Lt => value_op!(lhs, rhs, lt),
                    CmpKind::Gt => value_op!(lhs, rhs, gt),
                    CmpKind::Le => value_op!(lhs, rhs, le),
                    CmpKind::Ge => value_op!(lhs, rhs, ge),
                };
                let next_bb = if b { then_ } else { else_ };
                ctx.goto(*next_bb);
            }
            Hir::Ret(lhs) => return Some(ctx.eval_operand(lhs)),
            Hir::LocalStore(ret, ident, rhs) => {
                ctx.locals[*ident] = ctx[*rhs].clone();
                if let Some(ret) = ret {
                    ctx[*ret] = ctx[*rhs].clone();
                }
            }
            Hir::LocalLoad(ident, lhs) => {
                ctx[*lhs] = ctx.locals[*ident].clone();
            }
            Hir::Call(id, ret, args) => {
                let args = args
                    .iter()
                    .map(|op| ctx.eval_operand(op))
                    .collect::<Vec<Value>>();
                if let Some(ret) = *ret {
                    ctx[ret] = self.eval_function(hir_context, *id, &args)
                }
            }
            Hir::Br(next_bb) => {
                ctx.goto(*next_bb);
            }
            Hir::CondBr(cond_, then_, else_) => {
                let next_bb = if ctx[*cond_] == Value::Bool(false) {
                    else_
                } else {
                    then_
                };
                ctx.goto(*next_bb);
            }
            Hir::Phi(ret, phi) => {
                let reg = phi.iter().find(|(bb, _)| ctx.prev_bb == *bb).unwrap().1;
                ctx[*ret] = ctx[reg].clone();
            }
        }
        None
    }
}

struct FuncContext {
    ssareg: Vec<Value>,
    locals: Vec<Value>,
    cur_bb: usize,
    prev_bb: usize,
    pc: usize,
}

impl std::ops::Index<SsaReg> for FuncContext {
    type Output = Value;

    fn index(&self, i: SsaReg) -> &Value {
        &self.ssareg[i.to_usize()]
    }
}

impl std::ops::IndexMut<SsaReg> for FuncContext {
    fn index_mut(&mut self, i: SsaReg) -> &mut Value {
        &mut self.ssareg[i.to_usize()]
    }
}

impl FuncContext {
    fn goto(&mut self, bb: usize) {
        self.prev_bb = self.cur_bb;
        self.cur_bb = bb;
        self.pc = 0;
    }

    fn eval_operand(&self, op: &HirOperand) -> Value {
        match op {
            HirOperand::Const(c) => c.clone(),
            HirOperand::Reg(r) => self[*r].clone(),
        }
    }
}
