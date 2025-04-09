use memory_addr::VirtAddr;

use crate::mem::virt_to_phys;

/// The maximum number of bytes that can be read at once.
const MAX_RW_SIZE: usize = 256;

/// Writes a byte to the console.
pub fn putchar(c: u8) {
    sbi_rt::console_write_byte(c);
}

/// Tries to write bytes to the console from input u8 slice.
/// Returns the number of bytes written.
fn try_write_bytes(bytes: &[u8]) -> usize {
    sbi_rt::console_write(sbi_rt::Physical::new(
        // A maximum of 256 bytes can be written at a time
        // to prevent SBI from disabling IRQs for too long.
        bytes.len().min(MAX_RW_SIZE),
        virt_to_phys(VirtAddr::from_ptr_of(bytes.as_ptr())).as_usize(),
        0,
    ))
    .value
}

/// Writes bytes to the console from input u8 slice.
pub fn write_bytes(bytes: &[u8]) {
    // If the address is from userspace, we need to copy the bytes to kernel space.
    #[cfg(feature = "uspace")]
    if bytes.as_ptr() as usize & (1 << 63) == 0 {
        // Check if the address is valid.
        let kernel_bytes = bytes.to_vec();
        let mut write_len = 0;
        while write_len < kernel_bytes.len() {
            let len = try_write_bytes(&kernel_bytes[write_len..]);
            if len == 0 {
                break;
            }
            write_len += len;
        }
        return;
    }
    let mut write_len = 0;
    while write_len < bytes.len() {
        let len = try_write_bytes(&bytes[write_len..]);
        if len == 0 {
            break;
        }
        write_len += len;
    }
}

/// Reads bytes from the console into the given mutable slice.
/// Returns the number of bytes read.
// pub fn read_bytes(bytes: &mut [u8]) -> usize {
//     sbi_rt::console_read(sbi_rt::Physical::new(
//         bytes.len().min(MAX_RW_SIZE),
//         virt_to_phys(VirtAddr::from_mut_ptr_of(bytes.as_mut_ptr())).as_usize(),
//         0,
//     ))
//     .value
// }
pub fn read_bytes(bytes: &mut [u8]) -> usize {
    // If the address is from userspace, we need to use a kernel buffer
    #[cfg(feature = "uspace")]
    if bytes.as_ptr() as usize & (1 << 63) == 0 {
        let mut total_read = 0;
        while total_read < bytes.len() {
            let chunk_size = (bytes.len() - total_read).min(MAX_RW_SIZE);
            let mut kernel_buf = [0u8; MAX_RW_SIZE];
            let read_len = sbi_rt::console_read(sbi_rt::Physical::new(
                chunk_size,
                virt_to_phys(VirtAddr::from_mut_ptr_of(kernel_buf.as_mut_ptr())).as_usize(),
                0,
            )).value.min(chunk_size); // 防御性截断
            if read_len == 0 { break; } // 无更多数据
            bytes[total_read..total_read + read_len].copy_from_slice(&kernel_buf[..read_len]);
            total_read += read_len;
        }
        return total_read;
    }    
    // Direct read for kernel-space buffers
    sbi_rt::console_read(sbi_rt::Physical::new(
        bytes.len().min(MAX_RW_SIZE),
        virt_to_phys(VirtAddr::from_mut_ptr_of(bytes.as_mut_ptr())).as_usize(),
        0,
    ))
    .value
}
