use std::io;

/// The `VideoFormat` is a nutype enum for `video_format` as defined in
/// ISO/IEC-14496-10-2022 - E.2.1 Table E-2.
///
/// Defaults to 5 (unspecified).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VideoFormat {
    /// The video type is component.
    Component = 0,

    /// The video type is PAL.
    PAL = 1,

    /// The video type is NTSC.
    NTSC = 2,

    /// The video type is SECAM.
    SECAM = 3,

    /// The video type is MAC.
    MAC = 4,

    /// The video type is Unspecified.
    Unspecified = 5,

    /// The video type is Reserved.
    Reserved1 = 6,

    /// The video type is Reserved.
    Reserved2 = 7,
}

impl TryFrom<u8> for VideoFormat {
    type Error = io::Error;
    /// Converts a u8 value to a `VideoFormat` enum.
    /// Returns an error if the value is not a valid `VideoFormat`.
    ///
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(VideoFormat::Component),
            1 => Ok(VideoFormat::PAL),
            2 => Ok(VideoFormat::NTSC),
            3 => Ok(VideoFormat::SECAM),
            4 => Ok(VideoFormat::MAC),
            5 => Ok(VideoFormat::Unspecified),
            6 => Ok(VideoFormat::Reserved1),
            7 => Ok(VideoFormat::Reserved2),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid video format: {}", value),
            )),
        }
    }
}
