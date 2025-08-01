const BUILTIN_PLATFORMS: &[&str] = &[
    "aarch64-bsta1000b",
    "aarch64-qemu-virt",
    "aarch64-raspi4",
    "loongarch64-qemu-virt",
    "riscv64-qemu-virt",
    "x86_64-pc-oslab",
    "x86_64-qemu-q35",
    "riscv64-vf2",
];

const BUILTIN_PLATFORM_FAMILIES: &[&str] = &[
    "aarch64-bsta1000b",
    "aarch64-phytium-pi",
    "aarch64-qemu-virt",
    "aarch64-raspi",
    "loongarch64-qemu-virt",
    "riscv64-qemu-virt",
    "riscv64-vf2",
    "x86-pc",
];

fn make_cfg_values(str_list: &[&str]) -> String {
    str_list
        .iter()
        .map(|s| format!("{s:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn main() {
    let platform = axconfig::PLATFORM;

    println!("cargo:rustc-cfg=platform=\"{platform}\"");
    println!(
        "cargo:rustc-cfg=platform_family=\"{}\"",
        axconfig::plat::FAMILY
    );
    println!(
        "cargo::rustc-check-cfg=cfg(platform, values({}))",
        make_cfg_values(BUILTIN_PLATFORMS)
    );
    println!(
        "cargo::rustc-check-cfg=cfg(platform_family, values({}))",
        make_cfg_values(BUILTIN_PLATFORM_FAMILIES)
    );
}
