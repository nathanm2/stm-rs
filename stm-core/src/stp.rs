#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
pub enum StpVersion {
    STPv1 = 1,
    STPv2_1, // also covers STPv2.0
    STPv2_2,
}

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
pub enum OpCode {
    NULL = 0x0,
    M8 = 0x1,
    MERR = 0x2,
    C8 = 0x3,
    D8 = 0x4,
    D16 = 0x5,
    D32 = 0x6,
    D64 = 0x7,
    D8MTS = 0x8,
    D16MTS = 0x9,
    D32MTS = 0xA,
    D64MTS = 0xB,
    D4 = 0xC,
    D4MTS = 0xD,
    FLAG_TS = 0xE,
    M16 = 0xF1,
    GERR = 0xF2,
    C16 = 0xF3,
    D8TS = 0xF4,
    D16TS = 0xF5,
    D32TS = 0xF6,
    D64TS = 0xF7,
    D8M = 0xF8,
    D16M = 0xF9,
    D32M = 0xFA,
    D64M = 0xFB,
    D4TS = 0xFC,
    D4M = 0xFD,
    FLAG = 0xFE,
    VERSION = 0xF00,
}

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
pub enum TimestampType {
    STPv1 = 1,
    STPv2NATDELTA = 2,
    STPv2NAT = 3,
    STPv2GRAY = 4,
}

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
pub enum Packet {
    NULL,
    M8 {
        master: u8,
    },
    C8 {
        channel: u8,
    },
    VERSION {
        ts_type: TimestampType,
        is_le: bool, // Is little endian ?
        version: StpVersion,
    },
}
