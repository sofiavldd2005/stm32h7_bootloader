use serde::{Deserialize, Serialize};

/// 256 bytes of flash data.
///
/// Serde derives only support `[T; N]` up to N=32, so we split into
/// `[[u8; 32]; 8]` internally and expose a flat byte-slice interface.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct FlashData([[u8; 32]; 8]);

impl FlashData {
    pub const MAX_LEN: usize = 256;

    pub fn from_bytes(src: &[u8]) -> Self {
        let mut d = Self([[0u8; 32]; 8]);
        let len = src.len().min(Self::MAX_LEN);
        for (i, &b) in src[..len].iter().enumerate() {
            d.0[i / 32][i % 32] = b;
        }
        d
    }

    /// Raw view of all 256 bytes.
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: [[u8; 32]; 8] = 256 bytes, contiguous, all initialized.
        unsafe { core::slice::from_raw_parts(self.0.as_ptr() as *const u8, Self::MAX_LEN) }
    }
}

/// Commands sent from host to bootloader.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum HostCommand {
    /// Ping the bootloader — expects Ack reply.
    Ping,
    /// Erase all CM4 flash sectors.
    EraseAll,
    /// Write data to a flash address.
    /// `addr` must be 4-byte aligned.
    /// `len` is the number of valid bytes in the 256-byte payload.
    Write {
        addr: u32,
        data: FlashData,
        len: u16,
    },
    /// Request CRC-32/MPEG2 of a flash region.
    Crc32 {
        addr: u32,
        len: u32,
    },
    /// Boot the validated firmware — triggers system reset.
    Boot,
}

/// Replies from bootloader to host.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum HostReply {
    /// Command accepted.
    Ack,
    /// Command rejected with error code.
    Nak(u8),
    /// Response to Crc32 request.
    CrcResult(u32),
    /// Bootloader ready for next command.
    Ready,
}

/// Protocol error codes.
pub mod error {
    /// Address not aligned or out of range.
    pub const BAD_ADDRESS: u8 = 0x01;
    /// Data length exceeds maximum.
    pub const BAD_LENGTH: u8 = 0x02;
    /// Flash erase/program operation failed.
    pub const FLASH_ERROR: u8 = 0x03;
    /// Final CRC verification failed.
    pub const CRC_MISMATCH: u8 = 0x04;
    /// Bootloader is busy with prior operation.
    pub const BUSY: u8 = 0x05;
}
