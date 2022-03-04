use std::io::Write;

use monoasm::*;
use monoasm_macro::monoasm;

use super::*;

///
/// Physical registers for general purpose.
///
enum GeneralPhysReg {
    /// General purpose register (r8-r11)
    Reg(u64),
    /// On stack with offset from rbp.
    Stack(i64),
}

///
/// Physical registers for double-precision floating point numbers.
///
enum FloatPhysReg {
    /// Xmm registers (xmm1-xmm15)
    Xmm(u64),
    /// On stack with offset from rbp.
    Stack(i64),
}

///
/// Code generator
///
/// This generates x86-64 machine code from McIR into heap memory .
///
pub struct Codegen {
    jit: JitMemory,
    g_offset: usize,
    f_offset: usize,
    block_labels: Vec<DestLabel>,
    func_labels: Vec<DestLabel>,
}

impl Codegen {
    pub fn new() -> Self {
        Self {
            jit: JitMemory::new(),
            g_offset: 0,
            f_offset: 0,
            block_labels: vec![],
            func_labels: vec![],
        }
    }

    ///
    /// Allocate general register to physical register.
    ///
    /// Currently, first 4 registers are allocated to R8-R11 registers.
    ///
    fn g_phys_reg(&self, reg: GReg) -> GeneralPhysReg {
        let reg = reg.to_usize();
        if reg < 5 {
            GeneralPhysReg::Reg(reg as u64 + 8)
        } else {
            GeneralPhysReg::Stack(((reg - 4 + self.g_offset) * 8) as i64 + 8)
        }
    }

    ///
    /// Allocate general register to physical register.
    ///
    /// Currently, first 14 registers are allocated to xmm1-xmm15 registers.
    ///
    fn f_phys_reg(&self, reg: FReg) -> FloatPhysReg {
        let reg = reg.to_usize();
        if reg < 15 {
            FloatPhysReg::Xmm((reg + 1) as u64)
        } else {
            FloatPhysReg::Stack(((reg - 14 + self.f_offset) * 8) as i64 + 8)
        }
    }

    fn emit_jcc(&mut self, kind: CmpKind, br: DestLabel) {
        match kind {
            CmpKind::Eq => {
                monoasm!(self.jit, jeq br;);
            }
            CmpKind::Ne => {
                monoasm!(self.jit, jne br;);
            }
            CmpKind::Gt => {
                monoasm!(self.jit, jgt br;);
            }
            CmpKind::Ge => {
                monoasm!(self.jit, jge br;);
            }
            CmpKind::Lt => {
                monoasm!(self.jit, jlt br;);
            }
            CmpKind::Le => {
                monoasm!(self.jit, jle br;);
            }
        }
    }

    fn emit_fjcc(&mut self, kind: CmpKind, br: DestLabel) {
        match kind {
            CmpKind::Eq => {
                monoasm!(self.jit, jeq br;);
            }
            CmpKind::Ne => {
                monoasm!(self.jit, jne br;);
            }
            CmpKind::Gt => {
                monoasm!(self.jit, ja br;);
            }
            CmpKind::Ge => {
                monoasm!(self.jit, jae br;);
            }
            CmpKind::Lt => {
                monoasm!(self.jit, jb br;);
            }
            CmpKind::Le => {
                monoasm!(self.jit, jbe br;);
            }
        }
    }

    fn emit_setcc(&mut self, kind: CmpKind, dest: &GeneralPhysReg) {
        match dest {
            GeneralPhysReg::Reg(dest) => match kind {
                CmpKind::Eq => {
                    monoasm!( self.jit, seteq R(*dest); );
                }
                CmpKind::Ne => {
                    monoasm!( self.jit, setne R(*dest); );
                }
                CmpKind::Gt => {
                    monoasm!( self.jit, setgt R(*dest); );
                }
                CmpKind::Ge => {
                    monoasm!( self.jit, setge R(*dest); );
                }
                CmpKind::Lt => {
                    monoasm!( self.jit, setlt R(*dest); );
                }
                CmpKind::Le => {
                    monoasm!( self.jit, setle R(*dest); );
                }
            },
            GeneralPhysReg::Stack(dest) => match kind {
                CmpKind::Eq => {
                    monoasm!( self.jit, seteq [rbp-(*dest)]; );
                }
                CmpKind::Ne => {
                    monoasm!( self.jit, setne [rbp-(*dest)]; );
                }
                CmpKind::Gt => {
                    monoasm!( self.jit, setgt [rbp-(*dest)]; );
                }
                CmpKind::Ge => {
                    monoasm!( self.jit, setge [rbp-(*dest)]; );
                }
                CmpKind::Lt => {
                    monoasm!( self.jit, setlt [rbp-(*dest)]; );
                }
                CmpKind::Le => {
                    monoasm!( self.jit, setle [rbp-(*dest)]; );
                }
            },
        };
    }

    fn emit_fsetcc(&mut self, kind: CmpKind, dest: &GeneralPhysReg) {
        match dest {
            GeneralPhysReg::Reg(dest) => match kind {
                CmpKind::Eq => {
                    monoasm!( self.jit, seteq R(*dest); );
                }
                CmpKind::Ne => {
                    monoasm!( self.jit, setne R(*dest); );
                }
                CmpKind::Gt => {
                    monoasm!( self.jit, seta R(*dest); );
                }
                CmpKind::Ge => {
                    monoasm!( self.jit, setae R(*dest); );
                }
                CmpKind::Lt => {
                    monoasm!( self.jit, setb R(*dest); );
                }
                CmpKind::Le => {
                    monoasm!( self.jit, setbe R(*dest); );
                }
            },
            GeneralPhysReg::Stack(dest) => match kind {
                CmpKind::Eq => {
                    monoasm!( self.jit, seteq [rbp-(*dest)]; );
                }
                CmpKind::Ne => {
                    monoasm!( self.jit, setne [rbp-(*dest)]; );
                }
                CmpKind::Gt => {
                    monoasm!( self.jit, seta [rbp-(*dest)]; );
                }
                CmpKind::Ge => {
                    monoasm!( self.jit, setae [rbp-(*dest)]; );
                }
                CmpKind::Lt => {
                    monoasm!( self.jit, setb [rbp-(*dest)]; );
                }
                CmpKind::Le => {
                    monoasm!( self.jit, setbe [rbp-(*dest)]; );
                }
            },
        };
    }
}

macro_rules! integer_ops {
    ($self: ident, $op: ident, $lhs:ident, $rhs:ident) => {{
        let lhs = $self.g_phys_reg(*$lhs);
        match &$rhs {
            McGeneralOperand::Reg(rhs) => {
                let rhs = $self.g_phys_reg(*rhs);
                match (lhs, rhs) {
                    (GeneralPhysReg::Reg(lhs), GeneralPhysReg::Reg(rhs)) => {
                        monoasm!($self.jit, $op  R(lhs), R(rhs););
                    }
                    (GeneralPhysReg::Reg(lhs), GeneralPhysReg::Stack(rhs)) => {
                        monoasm!($self.jit, $op  R(lhs), [rbp-(rhs)];);
                    }
                    (GeneralPhysReg::Stack(lhs), GeneralPhysReg::Reg(rhs)) => {
                        monoasm!($self.jit, $op  [rbp-(lhs)], R(rhs););
                    }
                    (GeneralPhysReg::Stack(lhs), GeneralPhysReg::Stack(rhs)) => {
                        monoasm!($self.jit,
                          movq  rax, [rbp-(rhs)];
                          $op  [rbp-(lhs)], rax;
                        );
                    }
                };
            }
            McGeneralOperand::Integer(rhs) => {
                let lhs = $self.g_phys_reg(*$lhs);
                match lhs {
                    GeneralPhysReg::Reg(lhs) => {
                        monoasm!($self.jit, $op  R(lhs), (*rhs as i64););
                    }
                    GeneralPhysReg::Stack(lhs) => {
                        monoasm!($self.jit, $op  [rbp-(lhs)], (*rhs as i64););
                    }
                };
            }
        }
    }};
}

macro_rules! float_ops {
    ($self: ident, $op: ident, $lhs:ident, $rhs:ident) => {{
        let lhs = $self.f_phys_reg(*$lhs);
        match &$rhs {
            McFloatOperand::Reg(rhs) => {
                let rhs = $self.f_phys_reg(*rhs);
                match (lhs, rhs) {
                    (FloatPhysReg::Xmm(lhs), FloatPhysReg::Xmm(rhs)) => {
                        monoasm!($self.jit,
                          $op    xmm(lhs), xmm(rhs);
                        );
                    }
                    (FloatPhysReg::Xmm(lhs), FloatPhysReg::Stack(rhs)) => {
                        monoasm!($self.jit,
                          movsd  xmm0, [rbp-(rhs)];
                          $op    xmm(lhs), xmm0;
                        );
                    }
                    (FloatPhysReg::Stack(lhs), FloatPhysReg::Xmm(rhs)) => {
                        monoasm!($self.jit,
                          movsd  xmm0, [rbp-(lhs)];
                          $op    xmm0, xmm(rhs);
                          movsd  [rbp-(lhs)], xmm0;
                        );
                    }
                    (FloatPhysReg::Stack(lhs), FloatPhysReg::Stack(rhs)) => {
                        monoasm!($self.jit,
                          movsd  xmm0, [rbp-(lhs)];
                          $op    xmm0, [rbp-(rhs)];
                          movsd  [rbp-(lhs)], xmm0;
                        );
                    }
                }
            }
            McFloatOperand::Float(rhs) => {
                let lhs = $self.f_phys_reg(*$lhs);
                match lhs {
                    FloatPhysReg::Xmm(lhs) => {
                        let label = $self.jit.const_f64(*rhs);
                        monoasm!($self.jit,
                            movq   xmm0, [rip + label];
                            $op    xmm(lhs), xmm0;
                        );
                    }
                    FloatPhysReg::Stack(lhs) => {
                        let label = $self.jit.const_f64(*rhs);
                        monoasm!($self.jit,
                          movsd  xmm0, [rbp-(lhs)];
                          $op    xmm0, [rip + label];
                          movsd  [rbp-(lhs)], xmm0;
                        );
                    }
                }
            }
        };
    }}
}

impl Codegen {
    pub fn compile_and_run(&mut self, mcir_context: &McIrContext) -> Value {
        for _ in &mcir_context.blocks {
            self.block_labels.push(self.jit.label());
        }

        for _ in &mcir_context.functions {
            self.func_labels.push(self.jit.label());
        }

        for cur_fn in 0..mcir_context.functions.len() {
            self.compile_func(mcir_context, cur_fn);
        }

        let main_func = &mcir_context.functions[0];
        let ret_ty = main_func.ret_ty;
        let func_label = self.func_labels[0];
        self.jit.finalize::<*mut u64, i64>();
        let res = match ret_ty {
            Type::Integer => {
                let func = self.jit.get_label_addr::<(), i64>(func_label);
                let i = func(());
                Value::Integer(i as i32)
            }
            Type::Float => {
                let func = self.jit.get_label_addr::<(), f64>(func_label);
                let f = func(());
                Value::Float(f)
            }
            Type::Bool => {
                let func = self.jit.get_label_addr::<(), u8>(func_label);
                let f = func(());
                Value::Bool(f != 0)
            }
        };
        // dump local variables.
        /*for (name, (i, ty)) in local_map {
            match ty {
                Type::Integer => {
                    eprintln!("{} [{}: i64]", name, locals[*i] as i64);
                }
                Type::Float => {
                    eprintln!(
                        "{} [{}: f64]",
                        name,
                        f64::from_ne_bytes(locals[*i].to_ne_bytes())
                    );
                }
            }
        }*/
        #[cfg(debug_assertions)]
        self.dump_code();
        res
    }

    fn compile_func(&mut self, mcir_context: &McIrContext, cur_fn: usize) {
        let func = &mcir_context.functions[cur_fn];
        let ret_ty = func.ret_ty;
        let g_spill = match func.g_regs {
            i if i < 5 => 0,
            i => i - 4,
        };
        let f_spill = match func.f_regs {
            i if i < 15 => 0,
            i => i - 14,
        };
        let locals_num = func.locals.len();
        self.g_offset = locals_num;
        self.f_offset = locals_num + g_spill;
        let func_label = self.func_labels[cur_fn];
        self.jit.bind_label(func_label);
        self.prologue(locals_num + g_spill + f_spill);

        for bbi in &func.bbs {
            self.compile_bb(mcir_context, *bbi, ret_ty);
        }
    }

    fn compile_bb(&mut self, mcir_context: &McIrContext, bbi: usize, ret_ty: Type) {
        let bb = &mcir_context.blocks[bbi];
        let label = self.block_labels[bbi];
        self.jit.bind_label(label);
        for op in &bb.insts {
            match op {
                McIR::Integer(reg, i) => match self.g_phys_reg(*reg) {
                    GeneralPhysReg::Reg(reg) => {
                        monoasm!(self.jit, movq  R(reg), (*i););
                    }
                    GeneralPhysReg::Stack(ofs) => {
                        monoasm!(self.jit, movq  [rbp-(ofs)], (*i););
                    }
                },
                McIR::Float(reg, f) => {
                    let label = self.jit.const_f64(*f);
                    let f = u64::from_ne_bytes(f.to_ne_bytes()) as i64;
                    match self.f_phys_reg(*reg) {
                        FloatPhysReg::Xmm(reg) => {
                            monoasm!(self.jit,
                              movq   xmm(reg), [rip + label];
                            );
                        }
                        FloatPhysReg::Stack(ofs) => {
                            monoasm!(self.jit,
                              movq  [rbp-(ofs)], (f);
                            );
                        }
                    }
                }
                McIR::IAdd(lhs, rhs) => integer_ops!(self, addq, lhs, rhs),
                McIR::ISub(lhs, rhs) => integer_ops!(self, subq, lhs, rhs),
                McIR::IMul(lhs, rhs) => {
                    fn emit_rhs(mut jit: &mut JitMemory, rhs: GeneralPhysReg) {
                        match rhs {
                            GeneralPhysReg::Reg(rhs) => {
                                monoasm!(jit, imul  rax, R(rhs); );
                            }
                            GeneralPhysReg::Stack(rhs) => {
                                monoasm!(jit, imul  rax, [rbp-(rhs)]; );
                            }
                        }
                    }
                    let lhs = self.g_phys_reg(*lhs);
                    let rhs = self.g_phys_reg(*rhs);
                    match lhs {
                        GeneralPhysReg::Reg(lhs) => {
                            monoasm!(self.jit,
                              movq  rax, R(lhs);
                            );
                            emit_rhs(&mut self.jit, rhs);
                            monoasm!(self.jit,
                              movq  R(lhs), rax;
                            );
                        }
                        GeneralPhysReg::Stack(lhs) => {
                            monoasm!(self.jit,
                              movq  rax, [rbp-(lhs)];
                            );
                            emit_rhs(&mut self.jit, rhs);
                            monoasm!(self.jit,
                              movq  [rbp-(lhs)], rax;
                            );
                        }
                    };
                }
                McIR::IDiv(lhs, rhs) => {
                    fn emit_rhs(mut jit: &mut JitMemory, rhs: GeneralPhysReg) {
                        match rhs {
                            GeneralPhysReg::Reg(rhs) => {
                                monoasm!(jit, idiv  R(rhs););
                            }
                            GeneralPhysReg::Stack(rhs) => {
                                monoasm!(jit, idiv  [rbp-(rhs)];);
                            }
                        }
                    }
                    let lhs = self.g_phys_reg(*lhs);
                    let rhs = self.g_phys_reg(*rhs);
                    match lhs {
                        GeneralPhysReg::Reg(lhs) => {
                            monoasm!(self.jit,
                              movq  rax, R(lhs);
                              cqo;
                            );
                            emit_rhs(&mut self.jit, rhs);
                            monoasm!(self.jit,
                              movq  R(lhs), rax;
                            );
                        }
                        GeneralPhysReg::Stack(lhs) => {
                            monoasm!(self.jit,
                              movq  rax, [rbp-(lhs)];
                              cqo;
                            );
                            emit_rhs(&mut self.jit, rhs);
                            monoasm!(self.jit,
                              movq  [rbp-(lhs)], rax;
                            );
                        }
                    };
                }
                McIR::ICmp(kind, lhs, rhs) => {
                    let lhs = self.g_phys_reg(*lhs);
                    match rhs {
                        McGeneralOperand::Reg(rhs) => {
                            let rhs = self.g_phys_reg(*rhs);
                            match (&lhs, &rhs) {
                                (GeneralPhysReg::Reg(lhs_), rhs) => match rhs {
                                    GeneralPhysReg::Reg(rhs) => {
                                        monoasm!(self.jit, cmpq  R(*lhs_), R(*rhs););
                                    }
                                    GeneralPhysReg::Stack(rhs) => {
                                        monoasm!(self.jit, cmpq  R(*lhs_), [rbp-(rhs)];);
                                    }
                                },
                                (GeneralPhysReg::Stack(lhs_), rhs) => match rhs {
                                    GeneralPhysReg::Reg(rhs) => {
                                        monoasm!(self.jit, cmpq  [rbp-(lhs_)], R(*rhs););
                                    }
                                    GeneralPhysReg::Stack(rhs) => {
                                        monoasm!(self.jit,
                                            movq  rax, [rbp-(rhs)];
                                            cmpq  [rbp-(lhs_)], rax;
                                        );
                                    }
                                },
                            };
                        }
                        McGeneralOperand::Integer(rhs) => {
                            match &lhs {
                                GeneralPhysReg::Reg(lhs_) => {
                                    monoasm!(self.jit, cmpq  R(*lhs_), (*rhs););
                                }
                                GeneralPhysReg::Stack(lhs_) => {
                                    monoasm!(self.jit, cmpq  [rbp-(lhs_)], (*rhs););
                                }
                            };
                        }
                    }
                    self.emit_setcc(*kind, &lhs);
                }
                McIR::FCmp(kind, ret, lhs, rhs) => {
                    let lhs = self.f_phys_reg(*lhs);
                    let rhs = self.f_phys_reg(*rhs);
                    let ret = self.g_phys_reg(*ret);
                    match &lhs {
                        FloatPhysReg::Xmm(lhs) => match rhs {
                            FloatPhysReg::Xmm(rhs) => {
                                monoasm!(self.jit, ucomisd  xmm(*lhs), xmm(rhs););
                            }
                            FloatPhysReg::Stack(rhs) => {
                                monoasm!(self.jit, ucomisd  xmm(*lhs), [rbp-(rhs)];);
                            }
                        },
                        FloatPhysReg::Stack(lhs) => match rhs {
                            FloatPhysReg::Xmm(rhs) => {
                                monoasm!(self.jit,
                                    movq  xmm0, [rbp-(*lhs)];
                                    ucomisd  xmm0, xmm(rhs);
                                );
                            }
                            FloatPhysReg::Stack(rhs) => {
                                monoasm!(self.jit,
                                    movq  xmm0, [rbp-(*lhs)];
                                    ucomisd  xmm0, [rbp-(rhs)];
                                );
                            }
                        },
                    };
                    self.emit_fsetcc(*kind, &ret);
                }
                McIR::ICmpJmp(kind, lhs, rhs, dest) => {
                    let lhs = self.g_phys_reg(*lhs);
                    //let rhs = self.g_phys_reg(*rhs);
                    let label = self.block_labels[*dest];
                    match (lhs, rhs) {
                        (GeneralPhysReg::Reg(lhs), rhs) => match rhs {
                            McGeneralOperand::Integer(rhs) => {
                                monoasm!(self.jit, cmpq  R(lhs), (*rhs););
                            }
                            McGeneralOperand::Reg(rhs) => {
                                let rhs = self.g_phys_reg(*rhs);
                                match rhs {
                                    GeneralPhysReg::Reg(rhs) => {
                                        monoasm!(self.jit, cmpq  R(lhs), R(rhs););
                                    }
                                    GeneralPhysReg::Stack(rhs) => {
                                        monoasm!(self.jit, cmpq  R(lhs), [rbp-(rhs)];);
                                    }
                                }
                            }
                        },
                        (GeneralPhysReg::Stack(lhs), rhs) => match rhs {
                            McGeneralOperand::Integer(rhs) => {
                                monoasm!(self.jit, cmpq  [rbp-(lhs)], (*rhs););
                            }
                            McGeneralOperand::Reg(rhs) => {
                                let rhs = self.g_phys_reg(*rhs);
                                match rhs {
                                    GeneralPhysReg::Reg(rhs) => {
                                        monoasm!(self.jit, cmpq  [rbp-(lhs)], R(rhs););
                                    }
                                    GeneralPhysReg::Stack(rhs) => {
                                        monoasm!(self.jit,
                                            movq  rax, [rbp-(rhs)];
                                            cmpq  [rbp-(lhs)], rax;
                                        );
                                    }
                                }
                            }
                        },
                    };
                    self.emit_jcc(*kind, label);
                }
                McIR::FCmpJmp(kind, lhs, rhs, dest) => {
                    let lhs = self.f_phys_reg(*lhs);
                    let rhs = self.f_phys_reg(*rhs);
                    let label = self.block_labels[*dest];
                    match (lhs, rhs) {
                        (FloatPhysReg::Xmm(lhs), rhs) => match rhs {
                            FloatPhysReg::Xmm(rhs) => {
                                monoasm!(self.jit, ucomisd xmm(lhs), xmm(rhs););
                            }
                            FloatPhysReg::Stack(rhs) => {
                                monoasm!(self.jit, ucomisd xmm(lhs), [rbp-(rhs)];);
                            }
                        },
                        (FloatPhysReg::Stack(lhs), rhs) => match rhs {
                            FloatPhysReg::Xmm(rhs) => {
                                monoasm!(self.jit,
                                    movq  xmm0, [rbp-(lhs)];
                                    ucomisd xmm0, xmm(rhs);
                                );
                            }
                            FloatPhysReg::Stack(rhs) => {
                                monoasm!(self.jit,
                                    movq  xmm0, [rbp-(lhs)];
                                    ucomisd xmm0, [rbp-(rhs)];
                                );
                            }
                        },
                    };
                    self.emit_fjcc(*kind, label);
                }
                McIR::FAdd(lhs, rhs) => float_ops!(self, addsd, lhs, rhs),
                McIR::FSub(lhs, rhs) => float_ops!(self, subsd, lhs, rhs),
                McIR::FMul(lhs, rhs) => float_ops!(self, mulsd, lhs, rhs),
                McIR::FDiv(lhs, rhs) => float_ops!(self, divsd, lhs, rhs),

                McIR::CastIntFloat(dst, src) => match src {
                    &McGeneralOperand::Reg(src) => {
                        let src = self.g_phys_reg(src);
                        let dst = self.f_phys_reg(*dst);
                        match (src, dst) {
                            (GeneralPhysReg::Reg(src), FloatPhysReg::Xmm(dst)) => {
                                monoasm!(self.jit,
                                  cvtsi2sdq xmm(dst), R(src);
                                );
                            }
                            (GeneralPhysReg::Reg(src), FloatPhysReg::Stack(dst)) => {
                                monoasm!(self.jit,
                                  cvtsi2sdq xmm0, R(src);
                                  movsd  [rbp-(dst)], xmm0;
                                );
                            }
                            (GeneralPhysReg::Stack(src), FloatPhysReg::Xmm(dst)) => {
                                monoasm!(self.jit,
                                  cvtsi2sdq xmm(dst), [rbp-(src)];
                                );
                            }
                            (GeneralPhysReg::Stack(src), FloatPhysReg::Stack(dst)) => {
                                monoasm!(self.jit,
                                  cvtsi2sdq xmm0, [rbp-(src)];
                                  movsd  [rbp-(dst)], xmm0;
                                );
                            }
                        }
                    }
                    &McGeneralOperand::Integer(n) => {
                        let dst = self.f_phys_reg(*dst);
                        match dst {
                            FloatPhysReg::Xmm(dst) => {
                                monoasm!(self.jit,
                                  movq      rax, (n as i64);
                                  cvtsi2sdq xmm(dst), rax;
                                );
                            }
                            FloatPhysReg::Stack(dst) => {
                                monoasm!(self.jit,
                                  movq      rax, (n as i64);
                                  cvtsi2sdq xmm0, rax;
                                  movsd     [rbp-(dst)], xmm0;
                                );
                            }
                        }
                    }
                },
                McIR::FRet(lhs) => {
                    match lhs {
                        McFloatOperand::Float(f) => {
                            let n = i64::from_ne_bytes(f.to_le_bytes());
                            monoasm!(self.jit,
                              movq rax, (n);
                              movq xmm0, rax;
                            );
                        }
                        McFloatOperand::Reg(lhs) => match self.f_phys_reg(*lhs) {
                            FloatPhysReg::Xmm(lhs) => {
                                monoasm!(self.jit,
                                  movsd xmm0, xmm(lhs);
                                );
                            }
                            FloatPhysReg::Stack(ofs) => {
                                monoasm!(self.jit,
                                  movsd xmm0, [rbp-(ofs)];
                                );
                            }
                        },
                    }
                    self.epilogue();
                    match ret_ty {
                        Type::Float => {}
                        _ => panic!("Return type mismatch {:?} {:?}.", ret_ty, Type::Float),
                    }
                }
                McIR::IRet(lhs, ty) => {
                    match lhs {
                        McGeneralOperand::Integer(i) => {
                            monoasm!(self.jit,
                              movq rax, (*i as i64);
                            );
                        }
                        McGeneralOperand::Reg(lhs) => {
                            match self.g_phys_reg(*lhs) {
                                GeneralPhysReg::Reg(reg) => {
                                    monoasm!(self.jit,
                                      movq rax, R(reg);
                                    );
                                }
                                GeneralPhysReg::Stack(lhs) => {
                                    monoasm!(self.jit,
                                      movq rax, [rbp-(lhs)];
                                    );
                                }
                            };
                        }
                    }
                    self.epilogue();
                    if ret_ty != *ty {
                        panic!("Return type mismatch {:?} {:?}.", ret_ty, ty)
                    }
                }
                McIR::INeg(reg) => {
                    match self.g_phys_reg(*reg) {
                        GeneralPhysReg::Reg(reg) => {
                            monoasm!(self.jit, negq R(reg););
                        }
                        GeneralPhysReg::Stack(lhs) => {
                            monoasm!(self.jit, negq [rbp-(lhs)];);
                        }
                    };
                }
                McIR::FNeg(reg) => {
                    let n = i64::from_ne_bytes((0.0f64).to_le_bytes());
                    match self.f_phys_reg(*reg) {
                        FloatPhysReg::Xmm(reg) => {
                            monoasm!(self.jit,
                              movq  rax, (n);
                              movq  xmm0, rax;
                              subsd xmm0, xmm(reg);
                              movsd xmm(reg), xmm0;
                            );
                        }
                        FloatPhysReg::Stack(lhs) => {
                            monoasm!(self.jit,
                              movq  rax, (n);
                              movq  xmm0, rax;
                              subsd xmm0, [rbp-(lhs)];
                              movsd [rbp-(lhs)], xmm0;
                            );
                        }
                    };
                }
                McIR::LocalStore(ofs, reg) => {
                    let ofs = (ofs * 8) as i64;
                    match reg {
                        McReg::GReg(reg) => {
                            match self.g_phys_reg(*reg) {
                                GeneralPhysReg::Reg(reg) => {
                                    monoasm!(self.jit,
                                      movq [rbp-(ofs)], R(reg);
                                    );
                                }
                                GeneralPhysReg::Stack(lhs) => {
                                    monoasm!(self.jit,
                                      movq rax, [rbp-(lhs)];
                                      movq [rbp-(ofs)], rax;
                                    );
                                }
                            };
                        }
                        McReg::FReg(reg) => {
                            match self.f_phys_reg(*reg) {
                                FloatPhysReg::Xmm(reg) => {
                                    monoasm!(self.jit,
                                      movq [rbp-(ofs)], xmm(reg);
                                    );
                                }
                                FloatPhysReg::Stack(lhs) => {
                                    monoasm!(self.jit,
                                      movq rax, [rbp-(lhs)];
                                      movq [rbp-(ofs)], rax;
                                    );
                                }
                            };
                        }
                    };
                }
                McIR::LocalLoad(ofs, reg) => {
                    let ofs = (ofs * 8) as i64;
                    match reg {
                        McReg::GReg(reg) => {
                            match self.g_phys_reg(*reg) {
                                GeneralPhysReg::Reg(reg) => {
                                    monoasm!(self.jit,
                                      movq R(reg), [rbp-(ofs)];
                                    );
                                }
                                GeneralPhysReg::Stack(lhs) => {
                                    monoasm!(self.jit,
                                      movq rax, [rbp-(ofs)];
                                      movq [rbp-(lhs)], rax;
                                    );
                                }
                            };
                        }
                        McReg::FReg(reg) => {
                            match self.f_phys_reg(*reg) {
                                FloatPhysReg::Xmm(reg) => {
                                    monoasm!(self.jit,
                                      movq xmm(reg), [rbp-(ofs)];
                                    );
                                }
                                FloatPhysReg::Stack(lhs) => {
                                    monoasm!(self.jit,
                                      movq rax, [rbp-(ofs)];
                                      movq [rbp-(lhs)], rax;
                                    );
                                }
                            };
                        }
                    };
                }
                McIR::Jmp(dest) => {
                    if bbi + 1 != *dest {
                        let label = self.block_labels[*dest];
                        monoasm!(self.jit,
                          jmp label;
                        );
                    }
                }
                McIR::CondJmp(cond_, dest) => {
                    // cond_ must be Type::Bool.
                    let label = self.block_labels[*dest];
                    match cond_ {
                        McReg::GReg(reg) => {
                            match self.g_phys_reg(*reg) {
                                GeneralPhysReg::Reg(reg) => {
                                    monoasm!(self.jit,
                                      cmpb R(reg), 0;
                                    );
                                }
                                GeneralPhysReg::Stack(lhs) => {
                                    monoasm!(self.jit,
                                      cmpb [rbp-(lhs)], 0;
                                    );
                                }
                            };
                        }
                        _ => unreachable!(),
                    };
                    monoasm!(self.jit,
                      jeq label;
                    );
                }
                McIR::In(dest) => {
                    match dest {
                        McReg::GReg(reg) => {
                            match self.g_phys_reg(*reg) {
                                GeneralPhysReg::Reg(reg) => {
                                    monoasm!(self.jit,
                                      movq R(reg), rax;
                                    );
                                }
                                GeneralPhysReg::Stack(lhs) => {
                                    monoasm!(self.jit,
                                      movq [rbp-(lhs)], rax;
                                    );
                                }
                            };
                        }
                        McReg::FReg(reg) => {
                            match self.f_phys_reg(*reg) {
                                FloatPhysReg::Xmm(reg) => {
                                    monoasm!(self.jit,
                                      movq xmm(reg), rax;
                                    );
                                }
                                FloatPhysReg::Stack(lhs) => {
                                    monoasm!(self.jit,
                                      movq [rbp-(lhs)], rax;
                                    );
                                }
                            };
                        }
                    };
                }
                McIR::Out(dest) => {
                    match dest {
                        McReg::GReg(reg) => {
                            match self.g_phys_reg(*reg) {
                                GeneralPhysReg::Reg(reg) => {
                                    monoasm!(self.jit,
                                      movq rax, R(reg);
                                    );
                                }
                                GeneralPhysReg::Stack(lhs) => {
                                    monoasm!(self.jit,
                                      movq rax, [rbp-(lhs)];
                                    );
                                }
                            };
                        }
                        McReg::FReg(reg) => {
                            match self.f_phys_reg(*reg) {
                                FloatPhysReg::Xmm(reg) => {
                                    monoasm!(self.jit,
                                      movq rax, xmm(reg);
                                    );
                                }
                                FloatPhysReg::Stack(lhs) => {
                                    monoasm!(self.jit,
                                      movq rax, [rbp-(lhs)];
                                    );
                                }
                            };
                        }
                    };
                }
            }
        }
    }

    /// Dump generated code.
    fn dump_code(&self) {
        use std::fs::File;
        use std::process::Command;
        let asm = self.jit.to_vec();
        let mut file = File::create("tmp.bin").unwrap();
        file.write_all(&asm).unwrap();

        let output = Command::new("objdump")
            .args(&[
                "-D",
                "-Mintel,x86-64",
                "-b",
                "binary",
                "-m",
                "i386",
                "tmp.bin",
            ])
            .output();
        let asm = match &output {
            Ok(output) => std::str::from_utf8(&output.stdout).unwrap().to_string(),
            Err(err) => err.to_string(),
        };
        eprintln!("{}", asm);
    }

    fn prologue(&mut self, locals: usize) {
        monoasm!(self.jit,
            pushq rbp;
            movq rbp, rsp;
        );
        if locals != 0 {
            monoasm!(self.jit,
                subq rsp, ((locals + locals % 2) * 8);
            );
        }
    }

    fn epilogue(&mut self) {
        monoasm!(self.jit,
            movq rsp, rbp;
            popq rbp;
            ret;
        );
    }
}
