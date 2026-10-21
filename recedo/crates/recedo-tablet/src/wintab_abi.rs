//! Wintab 1.4 data structures and constants (from the public `wintab.h` / `pktdef.h`).
//!
//! Kept free of Win32 calls so the layout can be tested on any OS. Packets are decoded with
//! [`PacketLayout`], which computes field offsets from the packet-data mask the context was
//! actually opened with, because drivers may refuse optional fields and the packet shape
//! changes with the mask.

#![allow(non_camel_case_types, non_snake_case)]

pub type WTPKT = u32;
pub type FIX32 = u32;

// --- WTInfo categories and indices -------------------------------------------------------
pub const WTI_INTERFACE: u32 = 1;
pub const IFC_WINTABID: u32 = 1;
pub const IFC_SPECVERSION: u32 = 2;
pub const IFC_IMPLVERSION: u32 = 3;
pub const IFC_NDEVICES: u32 = 4;
pub const IFC_NCURSORS: u32 = 5;
pub const IFC_NCONTEXTS: u32 = 6;

pub const WTI_STATUS: u32 = 2;
pub const WTI_DEFCONTEXT: u32 = 3;
pub const WTI_DEFSYSCTX: u32 = 4;
pub const WTI_DEVICES: u32 = 100;
pub const WTI_CURSORS: u32 = 200;
pub const WTI_DDCTXS: u32 = 400;
pub const WTI_DSCTXS: u32 = 500;

pub const DVC_NAME: u32 = 1;
pub const DVC_HARDWARE: u32 = 2;
pub const DVC_NCSRTYPES: u32 = 3;
pub const DVC_FIRSTCSR: u32 = 4;
pub const DVC_PKTRATE: u32 = 5;
pub const DVC_PKTDATA: u32 = 6;
pub const DVC_PKTMODE: u32 = 7;
pub const DVC_CSRDATA: u32 = 8;
pub const DVC_X: u32 = 12;
pub const DVC_Y: u32 = 13;
pub const DVC_Z: u32 = 14;
pub const DVC_NPRESSURE: u32 = 15;
pub const DVC_TPRESSURE: u32 = 16;
pub const DVC_ORIENTATION: u32 = 17;
pub const DVC_ROTATION: u32 = 18;
pub const DVC_PNPID: u32 = 19;

pub const CSR_NAME: u32 = 1;
pub const CSR_ACTIVE: u32 = 2;
pub const CSR_PKTDATA: u32 = 3;
pub const CSR_BUTTONS: u32 = 4;
pub const CSR_BUTTONBITS: u32 = 5;
pub const CSR_PHYSID: u32 = 15;
pub const CSR_MODE: u32 = 16;
pub const CSR_CAPABILITIES: u32 = 19;
pub const CSR_TYPE: u32 = 20;

// --- Packet data bits (order = order of fields in the packet) ----------------------------
pub const PK_CONTEXT: WTPKT = 0x0001;
pub const PK_STATUS: WTPKT = 0x0002;
pub const PK_TIME: WTPKT = 0x0004;
pub const PK_CHANGED: WTPKT = 0x0008;
pub const PK_SERIAL_NUMBER: WTPKT = 0x0010;
pub const PK_CURSOR: WTPKT = 0x0020;
pub const PK_BUTTONS: WTPKT = 0x0040;
pub const PK_X: WTPKT = 0x0080;
pub const PK_Y: WTPKT = 0x0100;
pub const PK_Z: WTPKT = 0x0200;
pub const PK_NORMAL_PRESSURE: WTPKT = 0x0400;
pub const PK_TANGENT_PRESSURE: WTPKT = 0x0800;
pub const PK_ORIENTATION: WTPKT = 0x1000;
pub const PK_ROTATION: WTPKT = 0x2000;

/// What Recedo asks for first.
pub const PACKETDATA_FULL: WTPKT = PK_STATUS
    | PK_TIME
    | PK_CHANGED
    | PK_SERIAL_NUMBER
    | PK_CURSOR
    | PK_BUTTONS
    | PK_X
    | PK_Y
    | PK_Z
    | PK_NORMAL_PRESSURE
    | PK_TANGENT_PRESSURE
    | PK_ORIENTATION;
/// The set Qt and Krita use; known to open on common drivers.
pub const PACKETDATA_QT: WTPKT =
    PK_X | PK_Y | PK_BUTTONS | PK_TIME | PK_NORMAL_PRESSURE | PK_TANGENT_PRESSURE | PK_ORIENTATION | PK_CURSOR | PK_Z;
/// Last resort: position, pressure, buttons.
pub const PACKETDATA_MIN: WTPKT = PK_X | PK_Y | PK_BUTTONS | PK_NORMAL_PRESSURE | PK_CURSOR | PK_TIME;

// --- Context options and status ---------------------------------------------------------
pub const CXO_SYSTEM: u32 = 0x0001;
pub const CXO_PEN: u32 = 0x0002;
pub const CXO_MESSAGES: u32 = 0x0004;
pub const CXO_CSRMESSAGES: u32 = 0x0008;
pub const CXO_MGNINSIDE: u32 = 0x4000;
pub const CXO_MARGIN: u32 = 0x8000;

pub const TPS_PROXIMITY: u32 = 0x0001;
pub const TPS_QUEUE_ERR: u32 = 0x0002;
pub const TPS_MARGIN: u32 = 0x0004;
pub const TPS_GRAB: u32 = 0x0008;
pub const TPS_INVERT: u32 = 0x0010;

pub const TU_NONE: u32 = 0;
pub const TU_INCHES: u32 = 1;
pub const TU_CENTIMETERS: u32 = 2;
pub const TU_CIRCLE: u32 = 3;

// --- Messages ------------------------------------------------------------------------------
pub const WT_DEFBASE: u32 = 0x7FF0;
pub const WT_MAXOFFSET: u32 = 0xF;
pub const WT_PACKET: u32 = 0;
pub const WT_CTXOPEN: u32 = 1;
pub const WT_CTXCLOSE: u32 = 2;
pub const WT_CTXUPDATE: u32 = 3;
pub const WT_CTXOVERLAP: u32 = 4;
pub const WT_PROXIMITY: u32 = 5;
pub const WT_INFOCHANGE: u32 = 6;
pub const WT_CSRCHANGE: u32 = 7;
pub const WT_PACKETEXT: u32 = 8;

pub const LCNAMELEN: usize = 40;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AXIS {
    pub axMin: i32,
    pub axMax: i32,
    pub axUnits: u32,
    pub axResolution: FIX32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ORIENTATION {
    pub orAzimuth: i32,
    pub orAltitude: i32,
    pub orTwist: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LOGCONTEXTW {
    pub lcName: [u16; LCNAMELEN],
    pub lcOptions: u32,
    pub lcStatus: u32,
    pub lcLocks: u32,
    pub lcMsgBase: u32,
    pub lcDevice: u32,
    pub lcPktRate: u32,
    pub lcPktData: WTPKT,
    pub lcPktMode: WTPKT,
    pub lcMoveMask: WTPKT,
    pub lcBtnDnMask: u32,
    pub lcBtnUpMask: u32,
    pub lcInOrgX: i32,
    pub lcInOrgY: i32,
    pub lcInOrgZ: i32,
    pub lcInExtX: i32,
    pub lcInExtY: i32,
    pub lcInExtZ: i32,
    pub lcOutOrgX: i32,
    pub lcOutOrgY: i32,
    pub lcOutOrgZ: i32,
    pub lcOutExtX: i32,
    pub lcOutExtY: i32,
    pub lcOutExtZ: i32,
    pub lcSensX: FIX32,
    pub lcSensY: FIX32,
    pub lcSensZ: FIX32,
    pub lcSysMode: i32,
    pub lcSysOrgX: i32,
    pub lcSysOrgY: i32,
    pub lcSysExtX: i32,
    pub lcSysExtY: i32,
    pub lcSysSensX: FIX32,
    pub lcSysSensY: FIX32,
}

impl Default for LOGCONTEXTW {
    fn default() -> Self {
        // SAFETY: plain-old-data struct, all-zero is a valid value.
        unsafe { std::mem::zeroed() }
    }
}

impl LOGCONTEXTW {
    pub fn name(&self) -> String {
        utf16_until_nul(&self.lcName)
    }

    pub fn set_name(&mut self, name: &str) {
        self.lcName = [0; LCNAMELEN];
        for (dst, src) in self.lcName.iter_mut().zip(name.encode_utf16().take(LCNAMELEN - 1)) {
            *dst = src;
        }
    }
}

const _: () = assert!(std::mem::size_of::<AXIS>() == 16);
const _: () = assert!(std::mem::size_of::<ORIENTATION>() == 12);
// 40 WCHARs + 33 four-byte fields.
const _: () = assert!(std::mem::size_of::<LOGCONTEXTW>() == 80 + 33 * 4);

pub fn utf16_until_nul(s: &[u16]) -> String {
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    String::from_utf16_lossy(&s[..end])
}

/// FIX32 (16.16 fixed point) to f64.
pub fn fix32_to_f64(v: FIX32) -> f64 {
    (v >> 16) as f64 + (v & 0xFFFF) as f64 / 65536.0
}

/// One decoded packet. Fields not present in the context's mask are zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RawPacket {
    pub status: u32,
    pub time: u32,
    pub changed: u32,
    pub serial: u32,
    pub cursor: u32,
    pub buttons: u32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub normal_pressure: u32,
    pub tangent_pressure: u32,
    pub orientation: ORIENTATION,
    pub rotation: [i32; 3],
}

/// Field offsets of a packet for a given packet-data mask.
///
/// Fields appear in the order of their bit in the mask, low to high, with natural
/// alignment. On 64-bit Windows `HCTX` (`PK_CONTEXT`) is 8 bytes, everything else 4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacketLayout {
    pub mask: WTPKT,
    pub size: usize,
    offsets: [Option<usize>; 14],
}

const FIELD_SIZES: [(WTPKT, usize); 14] = [
    (PK_CONTEXT, 8),
    (PK_STATUS, 4),
    (PK_TIME, 4),
    (PK_CHANGED, 4),
    (PK_SERIAL_NUMBER, 4),
    (PK_CURSOR, 4),
    (PK_BUTTONS, 4),
    (PK_X, 4),
    (PK_Y, 4),
    (PK_Z, 4),
    (PK_NORMAL_PRESSURE, 4),
    (PK_TANGENT_PRESSURE, 4),
    (PK_ORIENTATION, 12),
    (PK_ROTATION, 12),
];

impl PacketLayout {
    pub fn new(mask: WTPKT) -> PacketLayout {
        let mut offsets = [None; 14];
        let mut off = 0usize;
        let mut max_align = 4usize;
        for (i, &(bit, size)) in FIELD_SIZES.iter().enumerate() {
            if mask & bit != 0 {
                let align = if bit == PK_CONTEXT { 8 } else { 4 };
                max_align = max_align.max(align);
                off = off.next_multiple_of(align);
                offsets[i] = Some(off);
                off += size;
            }
        }
        PacketLayout { mask, size: off.next_multiple_of(max_align).max(4), offsets }
    }

    fn offset(&self, bit: WTPKT) -> Option<usize> {
        let i = FIELD_SIZES.iter().position(|&(b, _)| b == bit)?;
        self.offsets[i]
    }

    pub fn has(&self, bit: WTPKT) -> bool {
        self.mask & bit != 0
    }

    /// Decodes one packet. `bytes` must be at least [`PacketLayout::size`] long.
    pub fn decode(&self, bytes: &[u8]) -> RawPacket {
        assert!(bytes.len() >= self.size, "packet buffer too short");
        let rd = |bit: WTPKT, extra: usize| -> u32 {
            match self.offset(bit) {
                Some(o) => u32::from_le_bytes(bytes[o + extra..o + extra + 4].try_into().unwrap()),
                None => 0,
            }
        };
        RawPacket {
            status: rd(PK_STATUS, 0),
            time: rd(PK_TIME, 0),
            changed: rd(PK_CHANGED, 0),
            serial: rd(PK_SERIAL_NUMBER, 0),
            cursor: rd(PK_CURSOR, 0),
            buttons: rd(PK_BUTTONS, 0),
            x: rd(PK_X, 0) as i32,
            y: rd(PK_Y, 0) as i32,
            z: rd(PK_Z, 0) as i32,
            normal_pressure: rd(PK_NORMAL_PRESSURE, 0),
            tangent_pressure: rd(PK_TANGENT_PRESSURE, 0),
            orientation: ORIENTATION {
                orAzimuth: rd(PK_ORIENTATION, 0) as i32,
                orAltitude: rd(PK_ORIENTATION, 4) as i32,
                orTwist: rd(PK_ORIENTATION, 8) as i32,
            },
            rotation: [rd(PK_ROTATION, 0) as i32, rd(PK_ROTATION, 4) as i32, rd(PK_ROTATION, 8) as i32],
        }
    }
}

/// Human-readable names of the bits in a packet-data mask.
pub fn describe_mask(mask: WTPKT) -> String {
    const NAMES: [(WTPKT, &str); 14] = [
        (PK_CONTEXT, "context"),
        (PK_STATUS, "status"),
        (PK_TIME, "time"),
        (PK_CHANGED, "changed"),
        (PK_SERIAL_NUMBER, "serial"),
        (PK_CURSOR, "cursor"),
        (PK_BUTTONS, "buttons"),
        (PK_X, "x"),
        (PK_Y, "y"),
        (PK_Z, "z"),
        (PK_NORMAL_PRESSURE, "pressure"),
        (PK_TANGENT_PRESSURE, "tangent-pressure"),
        (PK_ORIENTATION, "orientation"),
        (PK_ROTATION, "rotation"),
    ];
    let names: Vec<&str> = NAMES.iter().filter(|(b, _)| mask & b != 0).map(|(_, n)| *n).collect();
    format!("0x{mask:04X} [{}]", names.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_layout_matches_spec_order() {
        // Hand-computed from pktdef.h with PACKETDATA_FULL: status, time, changed, serial,
        // cursor, buttons, x, y, z, normal, tangent (11 x 4 bytes), orientation (12 bytes).
        let l = PacketLayout::new(PACKETDATA_FULL);
        assert_eq!(l.size, 56);
        assert_eq!(l.offset(PK_STATUS), Some(0));
        assert_eq!(l.offset(PK_TIME), Some(4));
        assert_eq!(l.offset(PK_CHANGED), Some(8));
        assert_eq!(l.offset(PK_SERIAL_NUMBER), Some(12));
        assert_eq!(l.offset(PK_CURSOR), Some(16));
        assert_eq!(l.offset(PK_BUTTONS), Some(20));
        assert_eq!(l.offset(PK_X), Some(24));
        assert_eq!(l.offset(PK_Y), Some(28));
        assert_eq!(l.offset(PK_Z), Some(32));
        assert_eq!(l.offset(PK_NORMAL_PRESSURE), Some(36));
        assert_eq!(l.offset(PK_TANGENT_PRESSURE), Some(40));
        assert_eq!(l.offset(PK_ORIENTATION), Some(44));
        assert_eq!(l.offset(PK_CONTEXT), None);
    }

    #[test]
    fn qt_layout() {
        // time, cursor, buttons, x, y, z, normal, tangent, orientation.
        let l = PacketLayout::new(PACKETDATA_QT);
        assert_eq!(l.size, 8 * 4 + 12);
        assert_eq!(l.offset(PK_TIME), Some(0));
        assert_eq!(l.offset(PK_CURSOR), Some(4));
        assert_eq!(l.offset(PK_X), Some(12));
        assert_eq!(l.offset(PK_ORIENTATION), Some(32));
    }

    #[test]
    fn context_handle_is_pointer_sized() {
        let l = PacketLayout::new(PK_CONTEXT | PK_STATUS | PK_X);
        assert_eq!(l.offset(PK_CONTEXT), Some(0));
        assert_eq!(l.offset(PK_STATUS), Some(8));
        assert_eq!(l.offset(PK_X), Some(12));
        assert_eq!(l.size, 16);
        // An odd number of 4-byte fields after the handle pads the struct to 8.
        assert_eq!(PacketLayout::new(PK_CONTEXT | PK_STATUS).size, 16);
    }

    #[test]
    fn decode_reads_little_endian_fields() {
        let l = PacketLayout::new(PACKETDATA_FULL);
        let mut b = vec![0u8; l.size];
        let put = |b: &mut Vec<u8>, off: usize, v: i32| b[off..off + 4].copy_from_slice(&v.to_le_bytes());
        put(&mut b, 0, TPS_INVERT as i32);
        put(&mut b, 4, 123_456);
        put(&mut b, 12, 77);
        put(&mut b, 16, 2);
        put(&mut b, 20, 0b101);
        put(&mut b, 24, 25_400);
        put(&mut b, 28, 14_000);
        put(&mut b, 36, 4095);
        put(&mut b, 44, 1800);
        put(&mut b, 48, -450);
        put(&mut b, 52, 90);
        let p = l.decode(&b);
        assert_eq!(p.status, TPS_INVERT);
        assert_eq!(p.time, 123_456);
        assert_eq!(p.serial, 77);
        assert_eq!(p.cursor, 2);
        assert_eq!(p.buttons, 0b101);
        assert_eq!((p.x, p.y), (25_400, 14_000));
        assert_eq!(p.normal_pressure, 4095);
        assert_eq!(p.orientation, ORIENTATION { orAzimuth: 1800, orAltitude: -450, orTwist: 90 });
    }

    #[test]
    fn context_name_roundtrip() {
        let mut c = LOGCONTEXTW::default();
        c.set_name("Recedo");
        assert_eq!(c.name(), "Recedo");
        let long = "x".repeat(100);
        c.set_name(&long);
        assert_eq!(c.name().len(), LCNAMELEN - 1);
    }

    #[test]
    fn fix32() {
        assert_eq!(fix32_to_f64(3600 << 16), 3600.0);
        assert_eq!(fix32_to_f64((1 << 16) | 0x8000), 1.5);
    }

    #[test]
    fn mask_description() {
        assert_eq!(describe_mask(PK_X | PK_Y), "0x0180 [x, y]");
    }
}
