use super::context::TrapFrame;
use loongArch64::register::estat::{self, Exception, Trap};

core::arch::global_asm!(
    include_str!("trap.S"),
    trapframe_size = const core::mem::size_of::<TrapFrame>(),
);

fn handle_breakpoint(era: &mut usize) {
    debug!("Exception(Breakpoint) @ {:#x} ", era);
    *era += 4;
}

#[unsafe(no_mangle)]
fn loongarch64_trap_handler(tf: &mut TrapFrame, _from_user: bool) {
    let estat = estat::read();

    match estat.cause() {
        Trap::Exception(Exception::Breakpoint) => handle_breakpoint(&mut tf.era),
        Trap::Interrupt(_) => {
            let irq_num: usize = estat.is().trailing_zeros() as usize;
            handle_trap!(IRQ, irq_num);
        }
        _ => {
            panic!(
                "Unhandled trap {:?} @ {:#x}:\n{:#x?}",
                estat.cause(),
                tf.era,
                tf
            );
        }
    }
}
