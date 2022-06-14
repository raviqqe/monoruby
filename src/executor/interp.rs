use super::compiler::Codegen;
use super::*;

///
/// Program counter base.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub(crate) struct BcPcBase(*const u64);

impl std::ops::Add<usize> for BcPcBase {
    type Output = BcPc;
    fn add(self, rhs: usize) -> BcPc {
        BcPc(unsafe { self.0.offset(rhs as isize) })
    }
}

impl std::ops::Add<InstId> for BcPcBase {
    type Output = BcPc;
    fn add(self, rhs: InstId) -> BcPc {
        BcPc(unsafe { self.0.offset(rhs.0 as isize) })
    }
}

impl BcPcBase {
    pub(super) fn new(func: &NormalFuncInfo) -> Self {
        BcPcBase(&func.bytecode()[0] as *const _)
    }
}

///
/// Program counter
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BcPc(*const u64);

impl std::ops::Sub<BcPcBase> for BcPc {
    type Output = usize;
    fn sub(self, rhs: BcPcBase) -> usize {
        let offset = unsafe { self.0.offset_from(rhs.0) };
        assert!(offset >= 0);
        offset as usize
    }
}

impl std::ops::Sub<BcPc> for BcPc {
    type Output = usize;
    fn sub(self, rhs: BcPc) -> usize {
        let offset = unsafe { self.0.offset_from(rhs.0) };
        assert!(offset >= 0);
        offset as usize
    }
}

impl std::ops::AddAssign<i32> for BcPc {
    fn add_assign(&mut self, offset: i32) {
        unsafe {
            *self = BcPc(self.0.offset(offset as isize));
        }
    }
}

impl std::default::Default for BcPc {
    fn default() -> Self {
        Self(std::ptr::null())
    }
}

/*fn conv(reg: u16) -> i64 {
    reg as i64 * 8 + 16
}*/

///
/// Bytecode interpreter.
///
pub struct Interp {
    pub codegen: Codegen,
}

impl Interp {
    fn new() -> Self {
        Self {
            codegen: Codegen::new(),
        }
    }

    pub fn jit_exec_toplevel(globals: &mut Globals) -> Result<Value> {
        let mut eval = Self::new();
        let f = eval.codegen.exec_toplevel(globals);
        let res = f(&mut eval, globals);
        res.ok_or_else(|| std::mem::take(&mut globals.error).unwrap())
    }

    pub fn eval_toplevel(globals: &mut Globals) -> Result<Value> {
        let main_id = globals.get_main_func();
        let mut eval = Self::new();

        let entry = eval.codegen.construct_vm();
        let vm_entry = eval.codegen.jit.get_label_address(eval.codegen.vm_entry);
        eval.codegen.precompile(&mut globals.func, vm_entry);

        let addr: fn(&mut Interp, &mut Globals, FuncId) -> Option<Value> =
            unsafe { std::mem::transmute(entry.as_ptr()) };
        match addr(&mut eval, globals, main_id) {
            Some(val) => Ok(val),
            None => Err(std::mem::take(&mut globals.error).unwrap()),
        }
    }
}
