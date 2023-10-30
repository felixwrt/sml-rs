//! High-level APIs for SML

use core::{fmt::Display, time::Duration};

use crate::parser::{
    common::{ListEntry, Time},
    streaming::{MessageBody, MessageStart, ParseEvent, Parser},
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
    pub const fn from_octet_str(value: OctetStr<'_>) -> Option<Self> {
        if value.len() != 6 || value[5] != 255 {
            return None;
        }
        // doesn't look nice, but also works in const contexts
        let mut vals = [0u8; 5];
        let mut idx = 0;
        while idx < 5 {
            vals[idx] = value[idx];
            idx += 1;
        }
        Some(ObisCode { inner: vals })
    }

    /// Parses an Obis Code from a string such as `"1-0:1.8.0"`
    ///
    /// # Examples
    ///
    /// ```
    /// # use sml_rs::application::ObisCode;
    /// let code = ObisCode::from_str("1-2:3.4.5");
    /// assert_eq!(&format!("{code}"), "1-2:3.4.5");
    /// ```
    pub const fn from_str(s: &str) -> Self {
        const SEPARATORS: &[u8; 4] = b"-:..";
        let bytes = s.as_bytes();
        let mut vals = [0u8; 5];
        let mut idx = 0;
        let mut val_idx = 0;
        while idx < bytes.len() {
            match bytes[idx] {
                b'0'..=b'9' => {
                    let n = bytes[idx] - b'0';
                    let Some(val) = vals[val_idx].checked_mul(10) else {
                        panic!("Overflow");
                    };
                    let Some(val) = val.checked_add(n) else {
                        panic!("Overflow");
                    };
                    vals[val_idx] = val;
                }
                b if SEPARATORS[val_idx] == b => {
                    val_idx += 1;
                    if val_idx > 4 {
                        panic!("Too many separators");
                    }
                }
                _ => {
                    panic!("Unexpected separator")
                }
            }
            idx += 1;
        }

        ObisCode { inner: vals }
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
    ValueNotFound(usize),
}

impl From<ParseError> for AppError {
    fn from(value: ParseError) -> Self {
        AppError::ParseError(value)
    }
}

enum ParseState {
    Initial,
    Data,
    Done,
}

enum TransmissionParserItem<'i> {
    MetaData {
        server_id: &'i [u8],
        time: Option<SecIndex>,
    },
    Data(ObisCode, Value),
}

struct TransmissionParser<'i> {
    parser: Parser<'i>,
    parse_state: ParseState,
    server_id: &'i [u8],
    // time information can be contained in three different spots:
    // - OpenResponse::ref_time
    // - GetListResponse::act_sensor_time
    // - ListEntry::val_time
    // Use `act_sensor_time` if available since that's the most commonly used
    // attribute. If it's not available, try to use `ref_time`. If that's also
    // not available, use `val_time`.
    time: Option<SecIndex>,
    use_val_time: bool,
}

impl<'i> TransmissionParser<'i> {
    pub fn new(data: &'i [u8]) -> Self {
        TransmissionParser {
            parser: Parser::new(data),
            parse_state: ParseState::Initial,
            server_id: &[],
            time: None,
            use_val_time: false,
        }
    }

    fn expect_next_message(&mut self) -> Result<MessageBody<'i>, AppError> {
        let evt = self.parser.next().ok_or(AppError::UnexpectedEof)??;
        let ParseEvent::MessageStart(MessageStart { message_body, .. }) = evt else {
            return Err(AppError::UnexpectedMessage);
        };
        Ok(message_body)
    }

    fn parse_initial(&mut self) -> Result<TransmissionParserItem<'i>, AppError> {
        let MessageBody::OpenResponse(or) = self.expect_next_message()? else {
            return Err(AppError::UnexpectedMessage);
        };

        self.server_id = or.server_id;
        let ref_time = or.ref_time.map(SecIndex::from);

        let MessageBody::GetListResponse(glr) = self.expect_next_message()? else {
            return Err(AppError::UnexpectedMessage);
        };

        let act_sensor_time = glr.act_sensor_time.map(SecIndex::from);
        self.time = act_sensor_time.or(ref_time);
        self.use_val_time = self.time.is_none();
        self.parse_state = ParseState::Data;
        self.parse_data()
    }

    fn parse_data(&mut self) -> Result<TransmissionParserItem<'i>, AppError> {
        loop {
            let evt = self.parser.next().ok_or(AppError::UnexpectedEof)??;
            match evt {
                ParseEvent::ListEntry(le) => {
                    if self.use_val_time {
                        let curr_val_time = le
                            .val_time
                            .as_ref()
                            .map(SecIndex::from)
                            .filter(|x| x.as_u32() != 0);
                        self.time = self.time.or(curr_val_time);
                    }

                    if let Some((obis_code, value)) = Self::parse_list_entry(&le) {
                        return Ok(TransmissionParserItem::Data(obis_code, value));
                    }
                }
                ParseEvent::GetListResponseEnd(_) => {
                    break;
                }
                ParseEvent::MessageStart(_) => unreachable!(),
            }
        }

        let MessageBody::CloseResponse(_) = self.expect_next_message()? else {
            return Err(AppError::UnexpectedMessage);
        };

        if self.parser.next().is_some() {
            return Err(AppError::UnexpectedMessage);
        }

        self.parse_state = ParseState::Done;
        // Ok(TransmissionParserItem::Time(self.time))
        Ok(TransmissionParserItem::MetaData {
            server_id: self.server_id,
            time: self.time,
        })
    }

    fn parse_list_entry(le: &ListEntry) -> Option<(ObisCode, Value)> {
        Some((
            ObisCode::from_octet_str(le.obj_name)?,
            Value {
                // ignore values of type Bool, Bytes or List
                value: le.value.as_i64()?,
                unit: le.unit.and_then(Unit::from_u8)?,
                scaler: le.scaler.unwrap_or(0),
            },
        ))
    }
}

impl<'i> Iterator for TransmissionParser<'i> {
    type Item = Result<TransmissionParserItem<'i>, AppError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.parse_state {
            ParseState::Initial => Some(self.parse_initial()),
            ParseState::Data => Some(self.parse_data()),
            ParseState::Done => None,
        }
    }
}

#[cfg(feature = "alloc")]
type ValueVec = Vec<(ObisCode, Value)>;

/// High-Level data structure containing data received from a power meter.
///
/// This data structure is designed for ease-of-use, containing only information
/// that's used by usual power meters. It should cover most use cases.
///
/// The `parser` module provides lower-level data structures that can be used
/// to access data not exposed by this API.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PowerMeterTransmission<'i, V> {
    /// identification of the server
    pub server_id: &'i [u8],
    /// time information (optional)
    pub time: Option<SecIndex>,
    /// vector of obis codes and their values
    pub values: V,
}

#[cfg(feature = "alloc")]
impl<'i> Display for PowerMeterTransmission<'i, ValueVec> {
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
impl<'i> PowerMeterTransmission<'i, ValueVec> {
    /// Parse a slice of bytes into a `PowerMeterTransmission`
    pub fn from_bytes(bytes: &'i [u8]) -> Result<Self, AppError> {
        let parser = TransmissionParser::new(bytes);

        let mut res = PowerMeterTransmission {
            server_id: [].as_slice(),
            time: None,
            values: alloc::vec::Vec::new(),
        };

        for item in parser {
            match item? {
                TransmissionParserItem::Data(obis_code, value) => {
                    res.values.push((obis_code, value))
                }
                TransmissionParserItem::MetaData { server_id, time } => {
                    res.server_id = server_id;
                    res.time = time;
                }
            }
        }

        Ok(res)
    }
}

impl<'i, const N: usize> PowerMeterTransmission<'i, [Value; N]> {
    /// Parse a slice of bytes into an sml transmission and extract values.
    ///
    /// Returns an array that for each obis code in `obis_codes` contains the
    /// correcsponding value. Returns `Err(AppError::ValueNotFound)` if a given
    /// obis code isn't found in the sml transmission.
    pub fn from_bytes_extract(
        bytes: &'i [u8],
        obis_codes: [ObisCode; N],
    ) -> Result<Self, AppError> {
        let parser = TransmissionParser::new(bytes);

        const DEFAULT_VAL: Option<Value> = None;
        let mut values = [DEFAULT_VAL; N];
        let mut time2 = None;
        let mut server_id2 = [].as_slice();

        for item in parser {
            match item? {
                TransmissionParserItem::MetaData { server_id, time } => {
                    server_id2 = server_id;
                    time2 = time;
                }
                TransmissionParserItem::Data(obis_code, value) => {
                    // continue if the elements' obis code is not in the array of expected ones
                    let Some(idx) = obis_codes.iter().position(|x| *x == obis_code) else {
                        continue;
                    };

                    values[idx] = Some(value);
                }
            }
        }

        for (idx, x) in values.iter().enumerate() {
            if x.is_none() {
                return Err(AppError::ValueNotFound(idx));
            }
        }
        let values = values.map(|x| x.unwrap());

        Ok(Self {
            server_id: server_id2,
            time: time2,
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
    const CODES: [ObisCode; 2] = [
        ObisCode::from_str("1-0:16.7.0"),
        ObisCode::from_str("1-0:1.8.0"),
    ];
    let res = PowerMeterTransmission::from_bytes_extract(msg, CODES);

    let expected = PowerMeterTransmission {
        server_id: &[10, 1, 68, 90, 71, 0, 2, 130, 34, 94],
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

    // let [watts, energy] = res.unwrap().values;

    // let PowerMeterTransmission2 {
    //     time,
    //     values: [watts, energy]
    // } = res.unwrap();
}

#[test]
fn test_app_layer_no_alloc_missing_value() {
    use crate::util::ArrayBuf;
    let bytes = include_bytes!("../../tests/libsml-testing/DZG_DVS-7412.2_jmberg.bin");
    let mut decoder = crate::transport::decode_streaming::<ArrayBuf<8000>>(bytes);
    let msg = decoder.next().unwrap().unwrap();
    let res = PowerMeterTransmission::from_bytes_extract(
        msg,
        [
            ObisCode::from_octet_str(&[1, 0, 1, 8, 0, 255]).unwrap(),
            ObisCode::from_octet_str(&[1, 2, 3, 4, 5, 255]).unwrap(),
        ],
    );

    assert_eq!(Err(AppError::ValueNotFound(1)), res);
}

#[test]
fn test_obis_codes() {
    const X: ObisCode = ObisCode::from_str("1-0:16.7.0");
    assert_eq!(X.inner, [1, 0, 16, 7, 0]);

    const X2: ObisCode = ObisCode::from_str("255-0:16.7.0");
    assert_eq!(X2.inner, [255, 0, 16, 7, 0]);
}
