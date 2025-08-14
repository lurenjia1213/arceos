use lazyinit::LazyInit;
use loongArch64::time::Time;

static NANOS_PER_TICK: LazyInit<u64> = LazyInit::new();

/// RTC wall time offset in nanoseconds at monotonic time base.
static mut RTC_EPOCHOFFSET_NANOS: u64 = 0;

/// Returns the current clock time in hardware ticks.
#[inline]
pub fn current_ticks() -> u64 {
    Time::read() as _
}

/// Return epoch offset in nanoseconds (wall time offset to monotonic clock start).
#[inline]
pub fn epochoffset_nanos() -> u64 {
    unsafe { RTC_EPOCHOFFSET_NANOS }
}

/// Converts hardware ticks to nanoseconds.
#[inline]
pub fn ticks_to_nanos(ticks: u64) -> u64 {
    ticks * *NANOS_PER_TICK
}

/// Converts nanoseconds to hardware ticks.
#[inline]
pub fn nanos_to_ticks(nanos: u64) -> u64 {
    nanos / *NANOS_PER_TICK
}

/// Set a one-shot timer.
///
/// A timer interrupt will be triggered at the specified monotonic time deadline (in nanoseconds).
///
/// LoongArch64 TCFG CSR: <https://loongson.github.io/LoongArch-Documentation/LoongArch-Vol1-EN.html#timer-configuration>
#[cfg(feature = "irq")]
pub fn set_oneshot_timer(deadline_ns: u64) {
    use loongArch64::register::tcfg;

    let ticks_now = current_ticks();
    let ticks_deadline = nanos_to_ticks(deadline_ns);
    let init_value = ticks_deadline - ticks_now;
    tcfg::set_init_val(init_value as _);
    tcfg::set_en(true);
}

pub(super) fn init_percpu() {
    #[cfg(feature = "irq")]
    {
        use loongArch64::register::tcfg;
        tcfg::set_init_val(0);
        tcfg::set_periodic(false);
        tcfg::set_en(true);
        super::irq::set_enable(super::irq::TIMER_IRQ_NUM, true);
    }
}

pub(super) fn init_primary() {
    NANOS_PER_TICK
        .init_once(crate::time::NANOS_PER_SEC / loongArch64::time::get_timer_freq() as u64);
}
//Ls7a RTC
//0x100d0100
//https://elixir.bootlin.com/linux/v6.14/source/drivers/rtc/rtc-loongson.c
#[cfg(feature = "rtc")]
const TOY_READ0_REG: usize = 0x2c; /* TOY low 32-bits value (read-only) */
const TOY_READ1_REG: usize = 0x30;
const RTC_CTRL_REG: usize = 0x40;
/* bitmask of RTC_CTRL_REG */
const TOY_ENABLE: u32 = 1 << 11;
const OSC_ENABLE: u32 = 1 << 8;
const TOY_ENABLE_MASK: u32 = TOY_ENABLE | OSC_ENABLE;
//https://elixir.bootlin.com/linux/v6.14/source/drivers/rtc/rtc-loongson.c#L37
pub(super) fn init_early() {
    #[cfg(feature = "rtc")]
    if axconfig::devices::RTC_PADDR != 0 {
        use crate::mem::phys_to_virt;
        use memory_addr::PhysAddr;

        const RTC_PADDR: PhysAddr = pa!(axconfig::devices::RTC_PADDR);
        // Get the current time in microseconds since the epoch (1970-01-01) from the riscv RTC.
        // Subtract the timer ticks to get the actual time when ArceOS was booted.
        let vaddr = phys_to_virt(RTC_PADDR).as_usize();
        unsafe { ((vaddr + RTC_CTRL_REG) as *mut u32).write_volatile(TOY_ENABLE_MASK) }
        let value = unsafe { ((vaddr + TOY_READ0_REG) as *const u32).read_volatile() };
        let year = unsafe { ((vaddr + TOY_READ1_REG) as *const u32).read_volatile() };
        /// GENMASK
        fn field_extract(reg: u32, high: u32, low: u32) -> u32 {
            assert!(high < 32 && low <= high);
            let mask = (!0u32 >> (31 - high)) & (!0u32 << low);
            (reg & mask) >> low
        }

        use chrono::{TimeZone, Timelike, Utc};
        let time = Utc
            .with_ymd_and_hms(
                1900 + year as i32,
                field_extract(value, 31, 26), //TOY_WRITE0_REG，不是TOY_MATCH0/1/2_REG！！
                field_extract(value, 25, 21),
                field_extract(value, 20, 16),
                field_extract(value, 15, 10),
                field_extract(value, 9, 4),
            )
            .unwrap()
            .with_nanosecond(field_extract(value, 3, 0) * crate::time::NANOS_PER_MILLIS as u32)
            .unwrap();
        let epoch_time_nanos = time.timestamp_nanos_opt().unwrap();
        unsafe {
            RTC_EPOCHOFFSET_NANOS = epoch_time_nanos as u64 - ticks_to_nanos(current_ticks());
        }
    }
}
