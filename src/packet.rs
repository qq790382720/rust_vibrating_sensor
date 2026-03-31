// src/packet.rs

pub const DAT_PACK_SIZE: usize = 116;
pub const PACKET_HEADER_SIZE: usize = 24; // 4+4+4+4+4+4
pub const PACKET_DATA_SIZE: usize = DAT_PACK_SIZE * 3 * 4; // f32 × 3 × 116
pub const PACKET_END_SIZE: usize = 4;
pub const PACKET_TOTAL_SIZE: usize = PACKET_HEADER_SIZE + PACKET_DATA_SIZE + PACKET_END_SIZE; // 1416 bytes

#[derive(Debug, Clone)]
pub struct DataPacket {
    pub pack_type: [u8; 4],
    pub sensor_id: u32,
    pub fs: f32,
    pub temperature: f32,
    pub pack_counter: u32,
    pub _reserved: u32,
    pub data: [[f32; 3]; DAT_PACK_SIZE],
    pub _pack_end: [u8; 4],
}

impl DataPacket {
    pub const PACK_END_MARKER: [u8; 4] = *b"PEND";

    pub fn parse(buf: &[u8]) -> Option<Self> {
        if buf.len() < PACKET_TOTAL_SIZE {
            return None;
        }

        let pack_type = <[u8; 4]>::try_from(&buf[0..4]).ok()?;
        let sensor_id = u32::from_le_bytes(<[u8; 4]>::try_from(&buf[4..8]).ok()?);
        let fs = f32::from_le_bytes(<[u8; 4]>::try_from(&buf[8..12]).ok()?);
        let temperature = f32::from_le_bytes(<[u8; 4]>::try_from(&buf[12..16]).ok()?);
        let pack_counter = u32::from_le_bytes(<[u8; 4]>::try_from(&buf[16..20]).ok()?);
        let reserved = u32::from_le_bytes(<[u8; 4]>::try_from(&buf[20..24]).ok()?);

        let mut data = [[0.0f32; 3]; DAT_PACK_SIZE];
        let mut offset = 24;
        for i in 0..DAT_PACK_SIZE {
            for j in 0..3 {
                let bytes = <[u8; 4]>::try_from(&buf[offset..offset + 4]).ok()?;
                data[i][j] = f32::from_le_bytes(bytes);
                offset += 4;
            }
        }

        let pack_end = <[u8; 4]>::try_from(&buf[offset..offset + 4]).ok()?;
        if pack_end != Self::PACK_END_MARKER {
            return None;
        }

        Some(DataPacket {
            pack_type,
            sensor_id,
            fs,
            temperature,
            pack_counter,
            _reserved:reserved,
            data,
            _pack_end:pack_end,
        })
    }

    pub fn is_data_packet(&self) -> bool {
        self.pack_type == *b"PUDT"
    }


}


