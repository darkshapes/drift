use rand_core::RngCore;
use rand_core::TryRngCore;

pub struct CryptoOsRng;

impl RngCore for CryptoOsRng {
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        rand::rngs::OsRng.try_fill_bytes(dest).expect("OsRng failed");
    }

    fn next_u32(&mut self) -> u32 {
        rand::rngs::OsRng.try_next_u32().expect("OsRng failed")
    }

    fn next_u64(&mut self) -> u64 {
        rand::rngs::OsRng.try_next_u64().expect("OsRng failed")
    }
}

impl Default for CryptoOsRng {
    fn default() -> Self {
        Self
    }
}

impl CryptoOsRng {
    pub fn new() -> Self {
        Self
    }
}

impl rand_core::CryptoRng for CryptoOsRng {}