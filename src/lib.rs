#![deny(unused_must_use)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::recursive_format_impl)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

pub use self::{
    connack::{ConnAck, ConnAckProperties, ConnectReturnCode},
    connect::{Connect, ConnectProperties, LastWill, LastWillProperties, Login},
    disconnect::{Disconnect, DisconnectProperties, DisconnectReasonCode},
    ping::{PingReq, PingResp},
    puback::{PubAck, PubAckProperties, PubAckReason},
    pubcomp::{PubComp, PubCompProperties, PubCompReason},
    publish::{Publish, PublishProperties},
    pubrec::{PubRec, PubRecProperties, PubRecReason},
    pubrel::{PubRel, PubRelProperties, PubRelReason},
    suback::{SubAck, SubAckProperties, SubscribeReasonCode},
    subscribe::{Filter, RetainForwardRule, Subscribe, SubscribeProperties},
    unsuback::{UnsubAck, UnsubAckProperties, UnsubAckReason},
    unsubscribe::{Unsubscribe, UnsubscribeProperties},
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
#[cfg(feature = "cow_string")]
use std::borrow::Cow;
use std::{fmt::Debug, slice::Iter};
use std::{str::Utf8Error, vec};

mod connack;
mod connect;
mod disconnect;
mod ping;
mod puback;
mod pubcomp;
mod publish;
mod pubrec;
mod pubrel;
mod suback;
mod subscribe;
mod unsuback;
mod unsubscribe;

#[cfg(all(feature = "boxed_string", feature = "binary_string"))]
compile_error!(
    "feature \"boxed_string\" and feature \"binary_string\" cannot be enabled at the same time"
);
#[cfg(all(feature = "boxed_string", feature = "cow_string"))]
compile_error!(
    "feature \"boxed_string\" and feature \"cow_string\" cannot be enabled at the same time"
);
#[cfg(all(feature = "binary_string", feature = "cow_string"))]
compile_error!(
    "feature \"binary_string\" and feature \"cow_string\" cannot be enabled at the same time"
);

#[cfg(feature = "boxed_string")]
type MqttString = Box<str>;

#[cfg(feature = "binary_string")]
type MqttString = Bytes;

#[cfg(feature = "cow_string")]
type MqttString = Cow<'static, str>;

#[cfg(all(
    not(feature = "boxed_string"),
    not(feature = "binary_string"),
    not(feature = "cow_string")
))]
type MqttString = String;

#[cfg(all(
    not(feature = "boxed_string"),
    not(feature = "binary_string"),
    not(feature = "cow_string")
))]
#[inline]
fn mqtt_string_eq(m: &MqttString, str: &str) -> bool {
    m == str
}

#[cfg(any(feature = "boxed_string", feature = "cow_string"))]
#[inline]
fn mqtt_string_eq(m: &MqttString, str: &str) -> bool {
    m.as_ref().eq(str)
}

#[cfg(feature = "binary_string")]
#[inline]
fn mqtt_string_eq(m: &Bytes, str: &str) -> bool {
    m.eq(str.as_bytes())
}

#[cfg(all(
    not(feature = "boxed_string"),
    not(feature = "binary_string"),
    not(feature = "cow_string")
))]
#[inline]
#[must_use]
pub fn mqtt_string_new(str: &'static str) -> MqttString {
    str.to_string()
}

#[cfg(feature = "boxed_string")]
#[inline]
#[must_use]
pub fn mqtt_string_new(str: &'static str) -> MqttString {
    str.into()
}

#[cfg(feature = "binary_string")]
#[inline]
#[must_use]
pub fn mqtt_string_new(str: &str) -> MqttString {
    Bytes::copy_from_slice(str.as_bytes())
}

#[cfg(feature = "cow_string")]
#[inline]
#[must_use]
pub fn mqtt_string_new(str: &'static str) -> MqttString {
    Cow::Borrowed(str)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet {
    Connect(Connect, Option<LastWill>, Option<Login>),
    ConnAck(ConnAck),
    Publish(Publish),
    PubAck(PubAck),
    PingReq(PingReq),
    PingResp(PingResp),
    Subscribe(Subscribe),
    SubAck(SubAck),
    PubRec(PubRec),
    PubRel(PubRel),
    PubComp(PubComp),
    Unsubscribe(Unsubscribe),
    UnsubAck(UnsubAck),
    Disconnect(Disconnect),
}

impl Packet {
    /// Reads a stream of bytes and extracts next MQTT packet out of it
    pub fn read(stream: &mut BytesMut, max_size: Option<usize>) -> Result<Packet, Error> {
        let fixed_header = check(stream.iter(), max_size)?;

        // Test with a stream with exactly the size to check border panics
        let packet = stream.split_to(fixed_header.frame_length());
        let packet_type = fixed_header.packet_type()?;

        if fixed_header.remaining_len == 0 && packet_type != PacketType::Disconnect {
            return match packet_type {
                PacketType::PingReq => Ok(Packet::PingReq(PingReq)),
                PacketType::PingResp => Ok(Packet::PingResp(PingResp)),
                _ => Err(Error::PayloadRequired),
            };
        }

        let packet = packet.freeze();
        let packet = match packet_type {
            PacketType::Connect => {
                let (connect, will, login) = Connect::read(fixed_header, packet)?;
                Packet::Connect(connect, will, login)
            }
            PacketType::Publish => {
                let publish = Publish::read(fixed_header, packet)?;
                Packet::Publish(publish)
            }
            PacketType::Subscribe => {
                let subscribe = Subscribe::read(fixed_header, packet)?;
                Packet::Subscribe(subscribe)
            }
            PacketType::Unsubscribe => {
                let unsubscribe = Unsubscribe::read(fixed_header, packet)?;
                Packet::Unsubscribe(unsubscribe)
            }
            PacketType::ConnAck => {
                let connack = ConnAck::read(fixed_header, packet)?;
                Packet::ConnAck(connack)
            }
            PacketType::PubAck => {
                let puback = PubAck::read(fixed_header, packet)?;
                Packet::PubAck(puback)
            }
            PacketType::PubRec => {
                let pubrec = PubRec::read(fixed_header, packet)?;
                Packet::PubRec(pubrec)
            }
            PacketType::PubRel => {
                let pubrel = PubRel::read(fixed_header, packet)?;
                Packet::PubRel(pubrel)
            }
            PacketType::PubComp => {
                let pubcomp = PubComp::read(fixed_header, packet)?;
                Packet::PubComp(pubcomp)
            }
            PacketType::SubAck => {
                let suback = SubAck::read(fixed_header, packet)?;
                Packet::SubAck(suback)
            }
            PacketType::UnsubAck => {
                let unsuback = UnsubAck::read(fixed_header, packet)?;
                Packet::UnsubAck(unsuback)
            }
            PacketType::PingReq => Packet::PingReq(PingReq),
            PacketType::PingResp => Packet::PingResp(PingResp),
            PacketType::Disconnect => {
                let disconnect = Disconnect::read(fixed_header, packet)?;
                Packet::Disconnect(disconnect)
            }
        };

        Ok(packet)
    }

    pub fn write(&self, write: &mut BytesMut) -> Result<usize, Error> {
        match self {
            Self::Publish(publish) => publish.write(write),
            Self::Subscribe(subscription) => subscription.write(write),
            Self::Unsubscribe(unsubscribe) => unsubscribe.write(write),
            Self::ConnAck(ack) => ack.write(write),
            Self::PubAck(ack) => ack.write(write),
            Self::SubAck(ack) => ack.write(write),
            Self::UnsubAck(unsuback) => unsuback.write(write),
            Self::PubRec(pubrec) => pubrec.write(write),
            Self::PubRel(pubrel) => pubrel.write(write),
            Self::PubComp(pubcomp) => pubcomp.write(write),
            Self::Connect(connect, will, login) => connect.write(will, login, write),
            Self::PingReq(_) => PingReq::write(write),
            Self::PingResp(_) => PingResp::write(write),
            Self::Disconnect(disconnect) => disconnect.write(write),
        }
    }
}

/// MQTT packet type
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    Connect = 1,
    ConnAck,
    Publish,
    PubAck,
    PubRec,
    PubRel,
    PubComp,
    Subscribe,
    SubAck,
    Unsubscribe,
    UnsubAck,
    PingReq,
    PingResp,
    Disconnect,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PropertyType {
    PayloadFormatIndicator = 1,
    MessageExpiryInterval = 2,
    ContentType = 3,
    ResponseTopic = 8,
    CorrelationData = 9,
    SubscriptionIdentifier = 11,
    SessionExpiryInterval = 17,
    AssignedClientIdentifier = 18,
    ServerKeepAlive = 19,
    AuthenticationMethod = 21,
    AuthenticationData = 22,
    RequestProblemInformation = 23,
    WillDelayInterval = 24,
    RequestResponseInformation = 25,
    ResponseInformation = 26,
    ServerReference = 28,
    ReasonString = 31,
    ReceiveMaximum = 33,
    TopicAliasMaximum = 34,
    TopicAlias = 35,
    MaximumQos = 36,
    RetainAvailable = 37,
    UserProperty = 38,
    MaximumPacketSize = 39,
    WildcardSubscriptionAvailable = 40,
    SubscriptionIdentifierAvailable = 41,
    SharedSubscriptionAvailable = 42,
}

/// Packet type from a byte
///
/// ```ignore
///          7                          3                          0
///          +--------------------------+--------------------------+
/// byte 1   | MQTT Control Packet Type | Flags for each type      |
///          +--------------------------+--------------------------+
///          |         Remaining Bytes Len  (1/2/3/4 bytes)        |
///          +-----------------------------------------------------+
///
/// <https://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html#_Toc385349207>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct FixedHeader {
    /// First byte of the stream. Used to identify packet types and
    /// several flags
    byte1: u8,
    /// Length of fixed header. Byte 1 + (1..4) bytes. So fixed header
    /// len can vary from 2 bytes to 5 bytes
    /// 1..4 bytes are variable length encoded to represent remaining length
    fixed_header_len: usize,
    /// Remaining length of the packet. Doesn't include fixed header bytes
    /// Represents variable header + payload size
    remaining_len: usize,
}

impl FixedHeader {
    #[must_use]
    pub fn new(byte1: u8, remaining_len_len: usize, remaining_len: usize) -> FixedHeader {
        FixedHeader {
            byte1,
            fixed_header_len: remaining_len_len + 1,
            remaining_len,
        }
    }

    pub fn packet_type(&self) -> Result<PacketType, Error> {
        let num = self.byte1 >> 4;
        match num {
            1 => Ok(PacketType::Connect),
            2 => Ok(PacketType::ConnAck),
            3 => Ok(PacketType::Publish),
            4 => Ok(PacketType::PubAck),
            5 => Ok(PacketType::PubRec),
            6 => Ok(PacketType::PubRel),
            7 => Ok(PacketType::PubComp),
            8 => Ok(PacketType::Subscribe),
            9 => Ok(PacketType::SubAck),
            10 => Ok(PacketType::Unsubscribe),
            11 => Ok(PacketType::UnsubAck),
            12 => Ok(PacketType::PingReq),
            13 => Ok(PacketType::PingResp),
            14 => Ok(PacketType::Disconnect),
            _ => Err(Error::InvalidPacketType(num)),
        }
    }

    /// Returns the size of full packet (fixed header + variable header + payload)
    /// Fixed header is enough to get the size of a frame in the stream
    #[must_use]
    pub fn frame_length(&self) -> usize {
        self.fixed_header_len + self.remaining_len
    }
}

fn property(num: u8) -> Result<PropertyType, Error> {
    let property = match num {
        1 => PropertyType::PayloadFormatIndicator,
        2 => PropertyType::MessageExpiryInterval,
        3 => PropertyType::ContentType,
        8 => PropertyType::ResponseTopic,
        9 => PropertyType::CorrelationData,
        11 => PropertyType::SubscriptionIdentifier,
        17 => PropertyType::SessionExpiryInterval,
        18 => PropertyType::AssignedClientIdentifier,
        19 => PropertyType::ServerKeepAlive,
        21 => PropertyType::AuthenticationMethod,
        22 => PropertyType::AuthenticationData,
        23 => PropertyType::RequestProblemInformation,
        24 => PropertyType::WillDelayInterval,
        25 => PropertyType::RequestResponseInformation,
        26 => PropertyType::ResponseInformation,
        28 => PropertyType::ServerReference,
        31 => PropertyType::ReasonString,
        33 => PropertyType::ReceiveMaximum,
        34 => PropertyType::TopicAliasMaximum,
        35 => PropertyType::TopicAlias,
        36 => PropertyType::MaximumQos,
        37 => PropertyType::RetainAvailable,
        38 => PropertyType::UserProperty,
        39 => PropertyType::MaximumPacketSize,
        40 => PropertyType::WildcardSubscriptionAvailable,
        41 => PropertyType::SubscriptionIdentifierAvailable,
        42 => PropertyType::SharedSubscriptionAvailable,
        num => return Err(Error::InvalidPropertyType(num)),
    };

    Ok(property)
}

/// Checks if the stream has enough bytes to frame a packet and returns fixed header
/// only if a packet can be framed with existing bytes in the `stream`.
/// The passed stream doesn't modify parent stream's cursor. If this function
/// returned an error, next `check` on the same parent stream is forced start
/// with cursor at 0 again (Iter is owned. Only Iter's cursor is changed internally)
pub fn check(stream: Iter<u8>, max_packet_size: Option<usize>) -> Result<FixedHeader, Error> {
    // Create fixed header if there are enough bytes in the stream
    // to frame full packet
    let stream_len = stream.len();
    let fixed_header = parse_fixed_header(stream)?;

    // Don't let rogue connections attack with huge payloads.
    // Disconnect them before reading all that data
    if let Some(max_size) = max_packet_size {
        if fixed_header.remaining_len > max_size {
            return Err(Error::PayloadSizeLimitExceeded {
                pkt_size: fixed_header.remaining_len,
                max: max_size,
            });
        }
    }

    // If the current call fails due to insufficient bytes in the stream,
    // after calculating remaining length, we extend the stream
    let frame_length = fixed_header.frame_length();
    if stream_len < frame_length {
        return Err(Error::InsufficientBytes(frame_length - stream_len));
    }

    Ok(fixed_header)
}

/// Parses fixed header
pub(crate) fn parse_fixed_header(mut stream: Iter<u8>) -> Result<FixedHeader, Error> {
    // At least 2 bytes are necessary to frame a packet
    let stream_len = stream.len();
    if stream_len < 2 {
        return Err(Error::InsufficientBytes(2 - stream_len));
    }

    let byte1 = stream.next().unwrap();
    let (len_len, len) = length(stream)?;

    Ok(FixedHeader::new(*byte1, len_len, len))
}

/// Parses variable byte integer in the stream and returns the length
/// and number of bytes that make it. Used for remaining length calculation
/// as well as for calculating property lengths
fn length(stream: Iter<u8>) -> Result<(usize, usize), Error> {
    let mut len: usize = 0;
    let mut len_len = 0;
    let mut done = false;
    let mut shift = 0;

    // Use continuation bit at position 7 to continue reading next
    // byte to frame 'length'.
    // Stream 0b1xxx_xxxx 0b1yyy_yyyy 0b1zzz_zzzz 0b0www_wwww will
    // be framed as number 0bwww_wwww_zzz_zzzz_yyy_yyyy_xxx_xxxx
    for byte in stream {
        len_len += 1;
        let byte = *byte as usize;
        len += (byte & 0x7F) << shift;

        // stop when continue bit is 0
        done = (byte & 0x80) == 0;
        if done {
            break;
        }

        shift += 7;

        // Only a max of 4 bytes allowed for remaining length
        // more than 4 shifts (0, 7, 14, 21) implies bad length
        if shift > 21 {
            return Err(Error::MalformedRemainingLength);
        }
    }

    // Not enough bytes to frame remaining length. wait for
    // one more byte
    if !done {
        return Err(Error::InsufficientBytes(1));
    }

    Ok((len_len, len))
}

/// Reads a series of bytes with a length from a byte stream
#[inline]
fn read_mqtt_bytes(stream: &mut Bytes) -> Result<Bytes, Error> {
    let len = read_u16(stream)? as usize;

    // Prevent attacks with wrong remaining length. This method is used in
    // `packet.assembly()` with (enough) bytes to frame packet. Ensures that
    // reading variable len string or bytes doesn't cross promised boundary
    // with `read_fixed_header()`
    if len > stream.len() {
        return Err(Error::BoundaryCrossed(len));
    }

    Ok(stream.split_to(len))
}

/// Reads a string from bytes stream
#[inline]
#[cfg(all(not(feature = "binary_string"), not(feature = "cow_string"),))]
fn read_mqtt_string(stream: &mut Bytes) -> Result<MqttString, Error> {
    let bytes = read_mqtt_bytes(stream)?;
    match std::str::from_utf8(&bytes) {
        Ok(v) => Ok(v.into()),
        Err(_) => Err(Error::TopicNotUtf8),
    }
}

#[inline]
#[cfg(feature = "cow_string")]
fn read_mqtt_string(stream: &mut Bytes) -> Result<Cow<'static, str>, Error> {
    let bytes = read_mqtt_bytes(stream)?;
    match std::str::from_utf8(&bytes) {
        Ok(v) => Ok(Cow::Owned(v.to_string())),
        Err(_) => Err(Error::TopicNotUtf8),
    }
}

#[inline]
#[cfg(feature = "binary_string")]
fn read_mqtt_string(stream: &mut Bytes) -> Result<MqttString, Error> {
    read_mqtt_bytes(stream)
}

/// Serializes bytes to stream (including length)
#[inline]
fn write_mqtt_bytes(stream: &mut BytesMut, bytes: &[u8]) -> Result<(), Error> {
    let Ok(len) = u16::try_from(bytes.len()) else {
        return Err(Error::BinaryDataTooLong);
    };
    stream.put_u16(len);
    stream.extend_from_slice(bytes);
    Ok(())
}

/// Serializes a string to stream
#[inline]
#[cfg(not(feature = "binary_string"))]
fn write_mqtt_string(stream: &mut BytesMut, string: &MqttString) -> Result<(), Error> {
    write_mqtt_bytes(stream, string.as_bytes())
}

#[cfg(feature = "binary_string")]
fn write_mqtt_string(stream: &mut BytesMut, string: &MqttString) -> Result<(), Error> {
    write_mqtt_bytes(stream, string)
}

/// Writes remaining length to stream and returns number of bytes for remaining length
fn write_remaining_length(stream: &mut BytesMut, len: usize) -> Result<usize, Error> {
    if len > 268_435_455 {
        return Err(Error::PayloadTooLong);
    }

    let mut done = false;
    let mut x = len;
    let mut count = 0;

    while !done {
        #[allow(clippy::cast_possible_truncation)]
        let mut byte = (x % 128) as u8;
        x /= 128;
        if x > 0 {
            byte |= 128;
        }

        stream.put_u8(byte);
        count += 1;
        done = x == 0;
    }

    Ok(count)
}

/// Return number of remaining length bytes required for encoding length
#[inline]
fn len_len(len: usize) -> usize {
    if len >= 2_097_152 {
        4
    } else if len >= 16_384 {
        3
    } else if len >= 128 {
        2
    } else {
        1
    }
}

/// After collecting enough bytes to frame a packet (packet's `frame()`)
/// , It's possible that content itself in the stream is wrong. Like expected
/// packet id or qos not being present. In cases where `read_mqtt_string` or
/// `read_mqtt_bytes` exhausted remaining length but packet framing expects to
/// parse qos next, these pre checks will prevent `bytes` crashes
#[inline]
fn read_u16(stream: &mut Bytes) -> Result<u16, Error> {
    if stream.len() < 2 {
        return Err(Error::MalformedPacket);
    }

    Ok(stream.get_u16())
}

#[inline]
fn read_u8(stream: &mut Bytes) -> Result<u8, Error> {
    if stream.is_empty() {
        return Err(Error::MalformedPacket);
    }

    Ok(stream.get_u8())
}

#[inline]
fn read_u32(stream: &mut Bytes) -> Result<u32, Error> {
    if stream.len() < 4 {
        return Err(Error::MalformedPacket);
    }

    Ok(stream.get_u32())
}

/// Quality of service
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
#[allow(clippy::enum_variant_names)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

impl Default for QoS {
    fn default() -> Self {
        Self::AtMostOnce
    }
}

/// Maps a number to `QoS`
#[must_use]
pub fn qos(num: u8) -> Option<QoS> {
    match num {
        0 => Some(QoS::AtMostOnce),
        1 => Some(QoS::AtLeastOnce),
        2 => Some(QoS::ExactlyOnce),
        _ => None,
    }
}

/// Error during serialization and deserialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("Invalid return code received as response for connect = {0}")]
    InvalidConnectReturnCode(u8),
    #[error("Invalid reason = {0}")]
    InvalidReason(u8),
    #[error("Invalid remaining length = {0}")]
    InvalidRemainingLength(usize),
    #[error("Invalid protocol used")]
    InvalidProtocol,
    #[error("Invalid protocol level")]
    InvalidProtocolLevel(u8),
    #[error("Invalid packet format")]
    IncorrectPacketFormat,
    #[error("Invalid packet type = {0}")]
    InvalidPacketType(u8),
    #[error("Invalid retain forward rule = {0}")]
    InvalidRetainForwardRule(u8),
    #[error("Invalid QoS level = {0}")]
    InvalidQoS(u8),
    #[error("Invalid subscribe reason code = {0}")]
    InvalidSubscribeReasonCode(u8),
    #[error("Packet received has id Zero")]
    PacketIdZero,
    #[error("Empty Subscription")]
    EmptySubscription,
    #[error("Subscription had id Zero")]
    SubscriptionIdZero,
    #[error("Payload size is incorrect")]
    PayloadSizeIncorrect,
    #[error("Payload is too long")]
    PayloadTooLong,
    #[error("Binary data is too long")]
    BinaryDataTooLong,
    #[error("Max Payload size of {max:?} has been exceeded by packet of {pkt_size:?} bytes")]
    PayloadSizeLimitExceeded { pkt_size: usize, max: usize },
    #[error("Payload is required")]
    PayloadRequired,
    #[error("Payload is required = {0}")]
    PayloadNotUtf8(#[from] Utf8Error),
    #[error("Topic not utf-8")]
    TopicNotUtf8,
    #[error("Promised boundary crossed, contains {0} bytes")]
    BoundaryCrossed(usize),
    #[error("Packet is malformed")]
    MalformedPacket,
    #[error("Remaining length is malformed")]
    MalformedRemainingLength,
    #[error("Invalid property type = {0}")]
    InvalidPropertyType(u8),
    /// More bytes required to frame packet. Argument
    /// implies minimum additional bytes required to
    /// proceed further
    #[error("Insufficient number of bytes to frame packet, {0} more bytes required")]
    InsufficientBytes(usize),
}

mod test {
    use bytes::BytesMut;

    use crate::Packet;

    // These are used in tests by packets
    #[allow(dead_code)]
    pub const USER_PROP_KEY: &str = "property";
    #[allow(dead_code)]
    pub const USER_PROP_VAL: &str = "a value thats really long............................................................................................................";

    #[allow(dead_code)]
    pub fn read_write_packets(packets: Vec<Packet>) {
        for out in packets {
            let mut buf = BytesMut::new();
            out.write(&mut buf).unwrap();
            let incoming = Packet::read(&mut buf, None).unwrap();
            assert_eq!(incoming, out);
            assert_eq!(buf.len(), 0);
        }
    }
}
