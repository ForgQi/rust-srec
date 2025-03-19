use std::io;

use byteorder::{BigEndian, ReadBytesExt};
use bytes::Bytes;
use bytes_util::BytesCursorExt;
use h264::AVCDecoderConfigurationRecord;

/// AVC Packet
#[derive(Debug, Clone, PartialEq)]
pub enum AvcPacket {
    /// AVC NALU
    Nalu { composition_time: u32, data: Bytes },
    /// AVC Sequence Header
    SequenceHeader(AVCDecoderConfigurationRecord),
    /// AVC End of Sequence
    EndOfSequence,
    /// AVC Unknown (we don't know how to parse it)
    Unknown {
        avc_packet_type: AvcPacketType,
        composition_time: u32,
        data: Bytes,
    },
}

impl AvcPacket {
    pub fn demux(reader: &mut io::Cursor<Bytes>) -> io::Result<Self> {
        let avc_packet_type = AvcPacketType::try_from(reader.read_u8()?)?;
        let composition_time = reader.read_u24::<BigEndian>()?;

        match avc_packet_type {
            AvcPacketType::SeqHdr => Ok(Self::SequenceHeader(
                AVCDecoderConfigurationRecord::parse(reader)?,
            )),
            AvcPacketType::Nalu => Ok(Self::Nalu {
                composition_time,
                data: reader.extract_remaining(),
            }),
            AvcPacketType::EndOfSequence => Ok(Self::EndOfSequence),
            _ => Ok(Self::Unknown {
                avc_packet_type,
                composition_time,
                data: reader.extract_remaining(),
            }),
        }
    }
}

/// FLV AVC Packet Type
/// Defined in the FLV specification. Chapter 1 - AVCVIDEODATA
/// The AVC packet type is used to determine if the video data is a sequence
/// header or a NALU.
#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AvcPacketType {
    SeqHdr = 0,
    Nalu = 1,
    EndOfSequence = 2,
    Unknown = 255,
}

impl TryFrom<u8> for AvcPacketType {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::SeqHdr),
            1 => Ok(Self::Nalu),
            2 => Ok(Self::EndOfSequence),
            _ => Ok(Self::Unknown),
        }
    }
}
