use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    ax_println!("{}", info);
    axhal::misc::terminate()
}
