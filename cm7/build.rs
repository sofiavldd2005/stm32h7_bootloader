use object::{Object, ObjectSegment};

fn main() {
    // --- Linker script setup
    println!("cargo:rustc-link-arg=-Tlink.x");
    println!("cargo:rustc-link-search={}", env!("CARGO_MANIFEST_DIR"));
    println!("cargo:rerun-if-changed=link.x");
    println!("cargo:rerun-if-changed=memory.x");

    // Locate the CM4 ELF binary.
    // When building with `--manifest-path cm7/Cargo.toml`,
    // the CM4 binary is at ../cm4/target/thumbv7em-none-eabihf/<profile>/cm4.
    let binding = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&binding).parent().unwrap();

    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let cm4_elf = workspace_root
        .join("cm4/target/thumbv7em-none-eabihf")
        .join(&profile)
        .join("cm4");
    // Read ELF data and extract loadable content into a 1 MB buffer
    let golden = if let Ok(data) = std::fs::read(&cm4_elf) {
        if let Ok(obj) = object::File::parse(&*data) {
            // Build a buffer representing the CM4 flash region [0x08100000, 0x08200000)
            let flash_base: u64 = 0x0810_0000;
            let flash_size: usize = 1024 * 1024; // 1 MB
            let mut buf = vec![0xFFu8; flash_size]; // unprogrammed = 0xFF
            for segment in obj.segments() {
                if let Ok(data) = segment.data() {
                    let addr = segment.address();
                    if addr >= flash_base && addr < flash_base + flash_size as u64 {
                        let offset = (addr - flash_base) as usize;
                        let end = core::cmp::min(offset + data.len(), flash_size - 4); // skip CRC slot
                        let copy_len = core::cmp::min(data.len(), end - offset);
                        buf[offset..offset + copy_len].copy_from_slice(&data[..copy_len]);
                    }
                }
            }

            // CRC the first 1M-4 bytes (skip the CRC slot at the end)
            let crc = crc32_mpeg2(&buf[..flash_size - 4]);
            Some(crc)
        } else {
            None
        }
    } else {
        None
    };

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest = std::path::Path::new(&out_dir).join("crc_golden.rs");

    match golden {
        Some(crc) => {
            std::fs::write(
                &dest,
                format!("pub const CRC_GOLDEN: u32 = 0x{:08X};\n", crc),
            )
            .unwrap();
            eprintln!("CM4 CRC32 = 0x{:08X}", crc);
        }
        None => {
            // Fallback: emit 0xFFFF_FFFF so runtime will fail visibly
            std::fs::write(&dest, "pub const CRC_GOLDEN: u32 = 0xFFFF_FFFF;\n").unwrap();
            eprintln!(
                "cargo:warning=CM4 ELF not found at {:?}. CRC validation will fail at runtime.",
                cm4_elf
            );
        }
    }
    println!("cargo:rerun-if-changed={}", cm4_elf.display());
}

fn crc32_mpeg2(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= (byte as u32) << 24;
        for _ in 0..8 {
            if crc & 0x8000_0000 != 0 {
                crc = (crc << 1) ^ 0x04C1_1DB7;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}
