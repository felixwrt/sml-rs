//! High-level APIs for SML

use core::{fmt::Display, time::Duration};

use crate::parser::{
    common::Time,
    streaming::{self, MessageBody, MessageStart, ParseEvent, Parser},
    OctetStr, ParseError,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Wrapper type for a number of seconds.
///
/// Typically, the value is the number of seconds the meter has been running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SecIndex {
    secs: u32,
}

impl SecIndex {
    /// Create a new `SecIndex` from a number.
    pub fn new(secs: u32) -> Self {
        SecIndex { secs }
    }

    /// Return the number of seconds.
    pub fn as_u32(&self) -> u32 {
        self.secs
    }

    /// Converts the `SecIndex` into a `Duration`.
    pub fn as_duration(&self) -> Duration {
        Duration::from_secs(self.secs as u64)
    }
}

impl From<Time> for SecIndex {
    fn from(value: Time) -> Self {
        SecIndex::from(&value)
    }
}

impl From<&Time> for SecIndex {
    fn from(value: &Time) -> Self {
        match value {
            Time::SecIndex(idx) => SecIndex::new(*idx),
        }
    }
}

impl From<SecIndex> for Duration {
    fn from(value: SecIndex) -> Self {
        value.as_duration()
    }
}

impl From<&SecIndex> for Duration {
    fn from(value: &SecIndex) -> Self {
        value.as_duration()
    }
}

/// Units as defined in [DLMS/COSEM][dlms] or [IEC 62056][iec]
///
/// This type only implements the units relevant for (and used by) power meters.
///
/// Specification of the units taken from this [pdf][dlmspdf] ([archive.org][dlmsarchive]).
/// See table on page 47.
///
/// [dlms]: https://www.dlms.com/dlms-cosem
/// [iec]: https://en.wikipedia.org/wiki/IEC_62056
/// [dlmspdf]: https://www.dlms.com/files/Blue-Book-Ed-122-Excerpt.pdf
/// [dlmsarchive]: https://web.archive.org/web/20211130052659/https://www.dlms.com/files/Blue-Book-Ed-122-Excerpt.pdf
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
#[non_exhaustive]
pub enum Unit {
    /// active power `[W]`
    Watt,
    /// active energy `[Wh]`
    WattHour,
    /// voltage `[V]`
    Volt,
    /// current `[A]`
    Ampere,
    /// (phase) angle `[°]`
    Degree,
    /// frequency `[Hz]`
    Hertz,
}

impl Unit {
    /// Returns a string describing the unit (e.g. `"W"` for `Unit::Watt`)
    pub fn as_str(&self) -> &'static str {
        match self {
            Unit::Watt => "W",
            Unit::WattHour => "Wh",
            Unit::Volt => "V",
            Unit::Ampere => "A",
            Unit::Degree => "°",
            Unit::Hertz => "Hz",
        }
    }

    /// Creates a `Unit` instance from a DLMN/COSEM unit number.
    ///
    /// Returns `None` if the given unit number doesn't match one of the supported units.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            8 => Some(Unit::Degree),
            27 => Some(Unit::Watt),
            30 => Some(Unit::WattHour),
            33 => Some(Unit::Ampere),
            35 => Some(Unit::Volt),
            44 => Some(Unit::Hertz),
            _ => None,
        }
    }
}

impl Display for Unit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A physical quantity built from a `value`, a `scaler` and a `unit`.
///
/// Calculation of the quantity: `value * 10 ^ scaler`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub struct Value {
    pub value: i64,
    pub unit: Unit,
    pub scaler: i8,
}

impl Display for Value {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let value = i128::from(self.value);
        if self.scaler >= 0 {
            write!(
                f,
                "{} {}",
                value * 10i128.pow(self.scaler as u32),
                self.unit
            )
        } else {
            let num_a = value / 10i128.pow((-self.scaler) as u32);
            let num_b = value.abs() % 10i128.pow((-self.scaler) as u32);
            write!(
                f,
                "{}.{:0width$} {}",
                num_a,
                num_b,
                self.unit,
                width = (-self.scaler) as usize
            )
        }
    }
}

/// A code as defined in [OBIS][obis]
///
/// See [here][obiscode] for a description of OBIS Codes.
///
/// [obis]: https://de.wikipedia.org/wiki/OBIS-Kennzahlen
/// [obiscode]: https://onemeter.com/docs/device/obis/
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObisCode {
    inner: [u8; 5],
}

impl Display for ObisCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}-{}:{}.{}.{}",
            self.inner[0], self.inner[1], self.inner[2], self.inner[3], self.inner[4]
        )
    }
}

impl ObisCode {
    /// Tries to parse an octet string into an obis code.
    pub fn from_octet_str(value: OctetStr<'_>) -> Option<Self> {
        if value.len() != 6 || value[5] != 255 {
            return None;
        }
        let Ok(vals) = value[..5].try_into() else {
            return None;
        };
        Some(ObisCode { inner: vals })
    }
}

/// Error type used by the application layer
#[derive(Debug, PartialEq)]
pub enum AppError {
    /// Expected another message in the SML transmission but encountered an EOF
    UnexpectedEof,
    /// Found message type that wasn't expected
    UnexpectedMessage,
    /// Error from the underlying parser
    ParseError(ParseError),
    /// An expected value wasn't found
    ValueNotFound,
}

impl From<ParseError> for AppError {
    fn from(value: ParseError) -> Self {
        AppError::ParseError(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
/// WIP
pub struct PowerMeterTransmission2<const N: usize> {
    /// identification of the server
    // pub server_id: Vec<u8>,
    /// time information (optional)
    pub time: Option<SecIndex>,
    /// vector of obis codes and their values
    pub values: [Value; N],
}

impl<const N: usize> PowerMeterTransmission2<N> {
    fn expect_next_message<'i>(parser: &'i mut Parser) -> Result<MessageBody<'i>, AppError> {
        let evt = parser.next().ok_or(AppError::UnexpectedEof)??;
        let ParseEvent::MessageStart(MessageStart { message_body, ..}) = evt else {
            return Err(AppError::UnexpectedMessage);
        };
        Ok(message_body)
    }

    /// Parse a slice of bytes into a `PowerMeterTransmission`
    // TODO: possible improvement: add warnings when values are omitted
    // TODO: possible improvement: add warnings for cases that are currently debug asserts
    pub fn from_bytes(bytes: &[u8], obis_codes: [ObisCode; N]) -> Result<Self, AppError> {
        let mut parser = streaming::Parser::new(bytes);

        let MessageBody::OpenResponse(or) = Self::expect_next_message(&mut parser)? else {
            return Err(AppError::UnexpectedMessage);
        };

        // let server_id = or.server_id.to_vec();

        // time information can be contained in three different spots:
        // - OpenResponse::ref_time
        // - GetListResponse::act_sensor_time
        // - ListEntry::val_time
        // Use `act_sensor_time` if available since that's the most commonly used
        // attribute. If it's not available, try to use `ref_time`. If that's also
        // not available, use `val_time`.
        let ref_time = or.ref_time.map(SecIndex::from);

        let MessageBody::GetListResponse(glr) = Self::expect_next_message(&mut parser)? else {
            return Err(AppError::UnexpectedMessage);
        };

        // server_id in OpenResponse and GetListResponse match
        // debug_assert_eq!(server_id, glr.server_id);

        let act_sensor_time = glr.act_sensor_time.map(SecIndex::from);
        // assert that if both `act_sensor_time` and `ref_time` are set, they contain the same value
        if let (Some(t1), Some(t2)) = (act_sensor_time, ref_time) {
            debug_assert_eq!(t1, t2)
        }

        const DEFAULT: Option<Value> = None;
        let mut values = [DEFAULT; N];
        let mut val_time = None;
        loop {
            let evt = parser.next().ok_or(AppError::UnexpectedEof)??;
            match evt {
                ParseEvent::ListEntry(le) => {
                    let curr_val_time = le.val_time.map(SecIndex::from).filter(|x| x.as_u32() != 0);
                    // assert that all `val_time`s are equal
                    if let (Some(t1), Some(t2)) = (val_time, curr_val_time) {
                        debug_assert_eq!(t1, t2)
                    }
                    val_time = val_time.or(curr_val_time);

                    // ignore values of type Bool, Bytes or List
                    let Some(val) = le.value.as_i64() else {
                        continue;
                    };

                    let Some(obis_code) = ObisCode::from_octet_str(le.obj_name) else {
                        continue;
                    };

                    // continue if the elements' obis code is not in the array of expected ones
                    let Some(idx) = obis_codes.iter().position(|x| *x == obis_code) else {
                        continue;
                    };

                    let Some(unit) = le.unit.and_then(Unit::from_u8) else {
                        continue;
                    };

                    values[idx] = Some(Value {
                        value: val,
                        unit,
                        scaler: le.scaler.unwrap_or(0),
                    });
                }
                ParseEvent::GetListResponseEnd(_) => {
                    break;
                }
                ParseEvent::MessageStart(_) => unreachable!(),
            }
        }
        if values.iter().any(|x| x.is_none()) {
            return Err(AppError::ValueNotFound);
        }
        let values = values.map(|x| x.unwrap());

        let MessageBody::CloseResponse(_) = Self::expect_next_message(&mut parser)? else {
            return Err(AppError::UnexpectedMessage);
        };

        if parser.next().is_some() {
            return Err(AppError::UnexpectedMessage);
        }

        let time = act_sensor_time.or(ref_time).or(val_time);

        Ok(PowerMeterTransmission2 {
            // server_id,
            time,
            values,
        })
    }
}

/// High-Level data structure containing data received from a power meter.
///
/// This data structure is designed for ease-of-use, containing only information
/// that's used by usual power meters. It should cover most use cases.
///
/// The `parser` module provides lower-level data structures that can be used
/// to access data not exposed by this API.
///
/// *This function is available only if sml-rs is built with the `"alloc"` feature.*
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PowerMeterTransmission {
    /// identification of the server
    pub server_id: Vec<u8>,
    /// time information (optional)
    pub time: Option<SecIndex>,
    /// vector of obis codes and their values
    pub values: Vec<(ObisCode, Value)>,
}

#[cfg(feature = "alloc")]
impl Display for PowerMeterTransmission {
    /// **Hint:** The output format used is unstable and may change at any time.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "PowerMeterTransmission:")?;
        writeln!(f, "  server_id: {:?}", &self.server_id)?;
        writeln!(f, "  time: {:?}", self.time.map(|x| x.as_u32()))?;
        writeln!(f, "  values:")?;
        for (obis_code, val) in &self.values {
            writeln!(f, "    {} = {}", obis_code, val)?;
        }
        Ok(())
    }
}

#[cfg(feature = "alloc")]
impl PowerMeterTransmission {
    fn expect_next_message<'i>(parser: &'i mut Parser) -> Result<MessageBody<'i>, AppError> {
        let evt = parser.next().ok_or(AppError::UnexpectedEof)??;
        let ParseEvent::MessageStart(MessageStart { message_body, ..}) = evt else {
            return Err(AppError::UnexpectedMessage);
        };
        Ok(message_body)
    }

    /// Parse a slice of bytes into a `PowerMeterTransmission`
    // TODO: possible improvement: add warnings when values are omitted
    // TODO: possible improvement: add warnings for cases that are currently debug asserts
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AppError> {
        let mut parser = streaming::Parser::new(bytes);

        let MessageBody::OpenResponse(or) = Self::expect_next_message(&mut parser)? else {
            return Err(AppError::UnexpectedMessage);
        };

        let server_id = or.server_id.to_vec();

        // time information can be contained in three different spots:
        // - OpenResponse::ref_time
        // - GetListResponse::act_sensor_time
        // - ListEntry::val_time
        // Use `act_sensor_time` if available since that's the most commonly used
        // attribute. If it's not available, try to use `ref_time`. If that's also
        // not available, use `val_time`.
        let ref_time = or.ref_time.map(SecIndex::from);

        let MessageBody::GetListResponse(glr) = Self::expect_next_message(&mut parser)? else {
            return Err(AppError::UnexpectedMessage);
        };

        // server_id in OpenResponse and GetListResponse match
        debug_assert_eq!(server_id, glr.server_id);

        let act_sensor_time = glr.act_sensor_time.map(SecIndex::from);
        // assert that if both `act_sensor_time` and `ref_time` are set, they contain the same value
        if let (Some(t1), Some(t2)) = (act_sensor_time, ref_time) {
            debug_assert_eq!(t1, t2)
        }

        let mut values = Vec::new();
        let mut val_time = None;
        loop {
            let evt = parser.next().ok_or(AppError::UnexpectedEof)??;
            match evt {
                ParseEvent::ListEntry(le) => {
                    let curr_val_time = le.val_time.map(SecIndex::from).filter(|x| x.as_u32() != 0);
                    // assert that all `val_time`s are equal
                    if let (Some(t1), Some(t2)) = (val_time, curr_val_time) {
                        debug_assert_eq!(t1, t2)
                    }
                    val_time = val_time.or(curr_val_time);

                    // ignore values of type Bool, Bytes or List
                    let Some(val) = le.value.as_i64() else {
                        continue;
                    };

                    let Some(obis_code) = ObisCode::from_octet_str(le.obj_name) else {
                        continue;
                    };

                    let Some(unit) = le.unit.and_then(Unit::from_u8) else {
                        continue;
                    };

                    values.push((
                        obis_code,
                        Value {
                            value: val,
                            unit,
                            scaler: le.scaler.unwrap_or(0),
                        },
                    ));
                }
                ParseEvent::GetListResponseEnd(_) => {
                    break;
                }
                ParseEvent::MessageStart(_) => unreachable!(),
            }
        }

        let MessageBody::CloseResponse(_) = Self::expect_next_message(&mut parser)? else {
            return Err(AppError::UnexpectedMessage);
        };

        if parser.next().is_some() {
            return Err(AppError::UnexpectedMessage);
        }

        let time = act_sensor_time.or(ref_time).or(val_time);

        Ok(PowerMeterTransmission {
            server_id,
            time,
            values,
        })
    }
}

#[test]
fn test_app_layer_no_alloc() {
    use crate::util::ArrayBuf;
    let bytes = include_bytes!("../../tests/libsml-testing/DZG_DVS-7412.2_jmberg.bin");
    let mut decoder = crate::transport::decode_streaming::<ArrayBuf<8000>>(bytes);
    let msg = decoder.next().unwrap().unwrap();
    let res = PowerMeterTransmission2::from_bytes(
        msg,
        [
            ObisCode::from_octet_str(&[1, 0, 16, 7, 0, 255]).unwrap(),
            ObisCode::from_octet_str(&[1, 0, 1, 8, 0, 255]).unwrap(),
        ],
    );

    let expected = PowerMeterTransmission2 {
        time: Some(SecIndex::new(99043543)),
        values: [
            Value {
                value: -29912,
                unit: Unit::Watt,
                scaler: -2,
            },
            Value {
                value: 54301577,
                unit: Unit::WattHour,
                scaler: -1,
            },
        ],
    };

    assert_eq!(Ok(expected), res);
}

#[test]
fn test_app_layer_no_alloc_missing_value() {
    use crate::util::ArrayBuf;
    let bytes = include_bytes!("../../tests/libsml-testing/DZG_DVS-7412.2_jmberg.bin");
    let mut decoder = crate::transport::decode_streaming::<ArrayBuf<8000>>(bytes);
    let msg = decoder.next().unwrap().unwrap();
    let res = PowerMeterTransmission2::from_bytes(
        msg,
        [
            ObisCode::from_octet_str(&[1, 2, 3, 4, 5, 255]).unwrap(),
            ObisCode::from_octet_str(&[1, 0, 1, 8, 0, 255]).unwrap(),
        ],
    );

    assert_eq!(Err(AppError::ValueNotFound), res);
}
