use core::fmt::Write;
use esp_idf_sys::esp_efuse_mac_get_default;
use heapless::String;

pub fn uuid() -> String<16> {
    // Step 1: make a 6-byte buffer
    let mut mac_bytes: [u8; 6] = [0; 6];

    // Step 2: call the C function to fill it
    // SAFETY: must be called after ESP system is up
    let err = unsafe { esp_efuse_mac_get_default(mac_bytes.as_mut_ptr()) };
    if err != esp_idf_sys::ESP_OK {
        // Handling error by returning zeros
        let mut s = String::new();
        write!(s, "000000000000").unwrap();
        return s;
    }

    // Step 3: pack those bytes into a u64
    // bitwise operators to pack 6 separate bytes into one 48-bit int stored in a 64-bit int
    let mac_raw: u64 = (mac_bytes[0] as u64) << 40
        | (mac_bytes[1] as u64) << 32
        | (mac_bytes[2] as u64) << 24
        | (mac_bytes[3] as u64) << 16
        | (mac_bytes[4] as u64) << 8
        | (mac_bytes[5] as u64);

    // Step 4: split high/low for formatting
    let hi = (mac_raw >> 32) as u16;
    let lo = mac_raw as u32;

    let mut s = String::<16>::new();
    // this gives you a 12-hex-digit uppercase string, e.g. "A1B2C3D4E5F6"
    write!(s, "{:04X}{:08X}", hi, lo).unwrap();
    s
}
