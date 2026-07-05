use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand};
use serialport::SerialPort;
use shared::protocol::{error, FlashData, HostCommand, HostReply};

// ---------------------------------------------------------------------------
// CRC-32/MPEG2 (non-reflected, poly 0x04C11DB7) — same algorithm as build.rs
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// COBS framing (mirrors cm7/src/protocol.rs)
// ---------------------------------------------------------------------------

const MAX_FRAME: usize = 512;

fn cobs_encode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() + 2);
    let mut code = 1u8;
    let mut code_pos = 0usize;

    out.push(0); // placeholder

    for &byte in input {
        if byte == 0 {
            out[code_pos] = (out.len() - code_pos) as u8;
            code_pos = out.len();
            out.push(0);
            code = 1;
        } else {
            out.push(byte);
            code += 1;
            if code == 0xFF {
                out[code_pos] = 0xFF;
                code_pos = out.len();
                out.push(0);
                code = 1;
            }
        }
    }
    out[code_pos] = code;
    out.push(0x00);
    out
}

fn cobs_decode(input: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;

    while i < input.len() {
        let code = input[i];
        if code == 0 {
            return None;
        }
        i += 1;

        let block_end = i + (code as usize) - 1;
        let end = block_end.min(input.len());

        while i < end {
            if input[i] == 0 {
                return None;
            }
            out.push(input[i]);
            i += 1;
        }

        if block_end < input.len() {
            out.push(0);
        }
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// UART transport
// ---------------------------------------------------------------------------

struct UartConnection {
    port: Box<dyn SerialPort>,
}

impl UartConnection {
    fn open(path: &str, baud: u32) -> Result<Self, String> {
        let port = serialport::new(path, baud)
            .timeout(Duration::from_secs(5))
            .open()
            .map_err(|e| format!("Failed to open {}: {}", path, e))?;
        Ok(Self { port })
    }

    fn read_byte(&mut self) -> u8 {
        let mut buf = [0u8; 1];
        loop {
            if self.port.read(&mut buf).is_ok() {
                return buf[0];
            }
        }
    }

    fn write_all(&mut self, data: &[u8]) -> Result<(), String> {
        self.port
            .write_all(data)
            .map_err(|e| format!("Write error: {}", e))
    }

    /// Send a command and receive the reply.
    fn send_command(&mut self, cmd: &HostCommand) -> Result<HostReply, String> {
        let serialized = postcard::to_allocvec(cmd).map_err(|e| format!("Serialize error: {}", e))?;
        let frame = cobs_encode(&serialized);
        self.write_all(&frame)?;

        // Read reply frame
        let mut raw = Vec::new();
        // Wait for first non-zero byte
        loop {
            let b = self.read_byte();
            if b != 0 {
                raw.push(b);
                break;
            }
        }
        // Read until 0x00
        loop {
            let b = self.read_byte();
            if b == 0 {
                break;
            }
            raw.push(b);
        }

        let decoded = cobs_decode(&raw).ok_or("COBS decode failed")?;
        let reply: HostReply =
            postcard::from_bytes(&decoded).map_err(|e| format!("Deserialize error: {}", e))?;
        Ok(reply)
    }

    fn read_binary(&self, path: &PathBuf) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))
    }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "bootloader-host", about = "STM32H755ZI-Q firmware update tool")]
struct Cli {
    /// Serial port (e.g. /dev/ttyACM0)
    #[arg(short, long, default_value = "/dev/ttyACM0")]
    port: String,

    /// Baud rate
    #[arg(short, long, default_value_t = 115200)]
    baud: u32,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check if bootloader is alive
    Ping,
    /// Read binary, flash, verify, and boot
    Flash {
        /// Path to CM4 firmware binary
        binary: PathBuf,
        /// Base address for flash (default: 0x0810_0000)
        #[arg(long, default_value_t = 0x0810_0000)]
        addr: u32,
    },
    /// Erase all CM4 flash
    Erase,
    /// Request CRC of a flash region
    Crc {
        /// Start address
        addr: u32,
        /// Length in bytes
        len: u32,
    },
    /// Send boot command
    Boot,
}

fn format_reply(reply: &HostReply) -> String {
    match reply {
        HostReply::Ack => "ACK".into(),
        HostReply::Nak(code) => format!("NAK(0x{code:02X})"),
        HostReply::CrcResult(val) => format!("CRC = 0x{val:08X}"),
        HostReply::Ready => "READY".into(),
    }
}

fn cmd_ping(conn: &mut UartConnection) -> Result<(), String> {
    let reply = conn.send_command(&HostCommand::Ping)?;
    println!("{}", format_reply(&reply));
    Ok(())
}

fn cmd_erase(conn: &mut UartConnection) -> Result<(), String> {
    println!("Erasing CM4 flash...");
    let reply = conn.send_command(&HostCommand::EraseAll)?;
    println!("{}", format_reply(&reply));
    Ok(())
}

fn cmd_boot(conn: &mut UartConnection) -> Result<(), String> {
    println!("Booting...");
    let reply = conn.send_command(&HostCommand::Boot)?;
    println!("{}", format_reply(&reply));
    Ok(())
}

fn cmd_crc(conn: &mut UartConnection, addr: u32, len: u32) -> Result<(), String> {
    let reply = conn.send_command(&HostCommand::Crc32 { addr, len })?;
    println!("{}", format_reply(&reply));
    Ok(())
}

fn cmd_flash(conn: &mut UartConnection, path: &PathBuf, base_addr: u32) -> Result<(), String> {
    let binary = conn.read_binary(path)?;
    let host_crc = crc32_mpeg2(&binary);
    println!(
        "Binary: {} ({} bytes, CRC = 0x{host_crc:08X})",
        path.display(),
        binary.len()
    );

    // Ping to verify bootloader is alive
    println!("Pinging bootloader...");
    let reply = conn.send_command(&HostCommand::Ping)?;
    if !matches!(reply, HostReply::Ack) {
        return Err(format!("Ping failed: {}", format_reply(&reply)));
    }
    println!("Bootloader alive");

    // Erase
    println!("Erasing CM4 flash...");
    let reply = conn.send_command(&HostCommand::EraseAll)?;
    if !matches!(reply, HostReply::Ack) {
        return Err(format!("Erase failed: {}", format_reply(&reply)));
    }
    println!("Erase complete");

    // Write in 256-byte chunks
    let chunk_size = 256;
    let total_chunks = (binary.len() + chunk_size - 1) / chunk_size;
    println!("Writing {} chunks...", total_chunks);

    for (chunk_idx, chunk) in binary.chunks(chunk_size).enumerate() {
        let mut data = [0u8; 256];
        let len = chunk.len() as u16;
        data[..chunk.len()].copy_from_slice(chunk);

        let cmd = HostCommand::Write {
            addr: base_addr + (chunk_idx as u32 * chunk_size as u32),
            data: FlashData::from_bytes(&data),
            len,
        };

        let reply = conn.send_command(&cmd)?;
        if !matches!(reply, HostReply::Ack) {
            return Err(format!(
                "Write failed at chunk {}/{}: {}",
                chunk_idx + 1,
                total_chunks,
                format_reply(&reply)
            ));
        }

        if (chunk_idx + 1) % 100 == 0 || chunk_idx + 1 == total_chunks {
            println!("  {}/{} chunks written", chunk_idx + 1, total_chunks);
        }
    }
    println!("Write complete");

    // Verify CRC
    println!("Verifying CRC...");
    let reply = conn.send_command(&HostCommand::Crc32 {
        addr: base_addr,
        len: binary.len() as u32,
    })?;

    match reply {
        HostReply::CrcResult(device_crc) => {
            println!(
                "Host CRC:   0x{host_crc:08X}\nDevice CRC: 0x{device_crc:08X}"
            );
            if host_crc != device_crc {
                return Err("CRC mismatch — not booting".into());
            }
            println!("CRC match!");
        }
        other => {
            return Err(format!("CRC command failed: {}", format_reply(&other)));
        }
    }

    // Boot
    println!("Booting...");
    let reply = conn.send_command(&HostCommand::Boot)?;
    println!("{}", format_reply(&reply));

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let mut conn = match UartConnection::open(&cli.port, cli.baud) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let result = match &cli.command {
        Commands::Ping => cmd_ping(&mut conn),
        Commands::Flash { binary, addr } => cmd_flash(&mut conn, binary, *addr),
        Commands::Erase => cmd_erase(&mut conn),
        Commands::Crc { addr, len } => cmd_crc(&mut conn, *addr, *len),
        Commands::Boot => cmd_boot(&mut conn),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
