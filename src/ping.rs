use super::{Debug, Error};
use bytes::{BufMut, BytesMut};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PingReq;

impl PingReq {
    pub fn write(payload: &mut BytesMut) -> Result<usize, Error> {
        payload.put_slice(&[0xC0, 0x00]);
        Ok(2)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PingResp;

impl PingResp {
    pub fn write(payload: &mut BytesMut) -> Result<usize, Error> {
        payload.put_slice(&[0xD0, 0x00]);
        Ok(2)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{test::read_write_packets, Packet};

    #[test]
    fn test_write_read() {
        read_write_packets(write_read_provider());
    }

    fn write_read_provider() -> Vec<Packet> {
        vec![Packet::PingReq(PingReq {}), Packet::PingResp(PingResp {})]
    }
}
