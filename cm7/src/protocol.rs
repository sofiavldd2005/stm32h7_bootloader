use crate::crc;
use crate::flash;
use crate::uart;
use shared::protocol::{error, HostCommand, HostReply};

/// Transport abstraction — any byte stream can implement this.
pub trait Transport {
    /// Read one byte, blocking.
    fn read_byte(&mut self) -> u8;
    /// Write one byte.
    fn write_byte(&mut self, b: u8);
}

/// UART transport implementation.
pub struct UartTransport;

impl Transport for UartTransport {
    fn read_byte(&mut self) -> u8 {
        uart::getc()
    }
    fn write_byte(&mut self, b: u8) {
        uart::putc(b)
    }
}

/// Max frame size: 256 payload + ~2 COBS overhead + 1 delimiter.
const BUF_SIZE: usize = 512;

/// COBS-encode `input` into `output`.
///
/// Returns the number of bytes written (includes the trailing `0x00` delimiter).
fn cobs_encode(input: &[u8], output: &mut [u8]) -> usize {
    let mut code = 1u8;
    let mut code_pos = 0usize;
    let mut out_pos = 1;

    for (i, &byte) in input.iter().enumerate() {
        if byte == 0 {
            output[code_pos] = (i - code_pos + 1) as u8;
            code_pos = out_pos;
            out_pos += 1;
            code = 1;
        } else {
            output[out_pos] = byte;
            out_pos += 1;
            code += 1;
            if code == 0xFF {
                output[code_pos] = 0xFF;
                code_pos = out_pos;
                out_pos += 1;
                code = 1;
            }
        }
    }
    output[code_pos] = code;
    output[out_pos] = 0x00;
    out_pos + 1
}

/// COBS-decode a frame (everything before the `0x00` delimiter) into `output`.
///
/// `input` must start at the beginning of the COBS-encoded data and include
/// all bytes up to (but not including) the `0x00` delimiter.
///
/// Returns the number of decoded bytes, or `None` on invalid encoding.
fn cobs_decode(input: &[u8], output: &mut [u8]) -> Option<usize> {
    let mut out_pos = 0;
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
            output[out_pos] = input[i];
            out_pos += 1;
            i += 1;
        }

        if block_end < input.len() {
            output[out_pos] = 0;
            out_pos += 1;
        }
    }
    Some(out_pos)
}

/// Receive one complete COBS frame from the transport.
///
/// Returns the decoded payload bytes.
fn recv_frame(tx: &mut dyn Transport) -> Option<[u8; BUF_SIZE]> {
    // COBS-decoded output lands here
    let mut raw: [u8; BUF_SIZE] = [0u8; BUF_SIZE];
    // Accumulate COBS-encoded bytes (everything before 0x00)
    let mut buf: [u8; BUF_SIZE] = [0u8; BUF_SIZE];
    let mut pos = 0;

    // Wait for first non-zero byte (beginning of frame)
    loop {
        let b = tx.read_byte();
        if b != 0 {
            buf[pos] = b;
            pos += 1;
            break;
        }
    }

    // Read until 0x00 delimiter (or full buffer)
    loop {
        let b = tx.read_byte();
        if b == 0 {
            break;
        }
        if pos >= buf.len() {
            return None;
        }
        buf[pos] = b;
        pos += 1;
    }

    cobs_decode(&buf[..pos], &mut raw)?;
    Some(raw)
}

/// Send a payload as a COBS frame.
fn send_frame(tx: &mut dyn Transport, payload: &[u8]) {
    let mut buf = [0u8; BUF_SIZE];
    let len = cobs_encode(payload, &mut buf);
    for b in &buf[..len] {
        tx.write_byte(*b);
    }
}

/// Dispatch a single command and return the reply.
fn dispatch(cmd: &HostCommand) -> HostReply {
    match cmd {
        HostCommand::Ping => HostReply::Ack,

        HostCommand::EraseAll => {
            let result = unsafe { flash::erase_all() };
            match result {
                Ok(()) => HostReply::Ack,
                Err(()) => HostReply::Nak(error::FLASH_ERROR),
            }
        }

        HostCommand::Write { addr, data, len } => {
            if *addr & 3 != 0 || *addr < 0x0810_0000 || *addr >= 0x0820_0000 {
                return HostReply::Nak(error::BAD_ADDRESS);
            }
            let word_len = (*len as usize + 3) / 4;
            let bytes = data.as_bytes();

            // Program the flash 32-bit words at a time
            for i in 0..word_len {
                let w_addr = addr + (i as u32 * 4);
                let word_bytes = &bytes[i * 4..core::cmp::min(i * 4 + 4, bytes.len())];
                let mut word = 0u32;
                for (j, &b) in word_bytes.iter().enumerate() {
                    word |= (b as u32) << (j * 8);
                }
                let result = unsafe { flash::program_word(w_addr, word) };
                if result.is_err() {
                    return HostReply::Nak(error::FLASH_ERROR);
                }
            }
            HostReply::Ack
        }

        HostCommand::Crc32 { addr, len } => {
            let crc = unsafe { crc::compute_region(*addr, *len) };
            match crc {
                Some(crc_val) => HostReply::CrcResult(crc_val),
                None => HostReply::Nak(error::BAD_ADDRESS),
            }
        }

        HostCommand::Boot => {
            // Set FW_APPROVED so CM4 will boot the new firmware
            unsafe {
                core::ptr::write_volatile(
                    shared::mem::FW_APPROVED,
                    0xDEAD_BEEF,
                );
            }
            HostReply::Ack
        }
    }
}

/// Main protocol loop — never returns.
pub fn protocol_loop(tx: &mut dyn Transport) -> ! {
    loop {
        if let Some(raw) = recv_frame(tx) {
            if let Ok(cmd) = postcard::from_bytes::<HostCommand>(&raw) {
                let reply = dispatch(&cmd);

                let mut reply_buf = [0u8; BUF_SIZE];
                if let Ok(serialized) = postcard::to_slice::<HostReply>(&reply, &mut reply_buf) {
                    send_frame(tx, serialized);
                }

                // If Boot was acknowledged, trigger system reset.
                if matches!(cmd, HostCommand::Boot) {
                    // SCB_AIRCR: SYSRESETREQ
                    unsafe {
                        core::ptr::write_volatile(
                            0xE000_ED0C as *mut u32,
                            0x5FA0_0004,
                        );
                    }
                    loop {}
                }
            } else {
                let nak = HostReply::Nak(error::BAD_ADDRESS);
                let mut buf = [0u8; BUF_SIZE];
                if let Ok(serialized) = postcard::to_slice::<HostReply>(&nak, &mut buf) {
                    send_frame(tx, serialized);
                }
            }
        }
    }
}
