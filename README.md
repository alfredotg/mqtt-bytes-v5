
# MQTT v5 serialization and deserialization

This is a low level crate with the ability to assemble and disassemble MQTT 5 packets and is used by both client and broker. Uses 'bytes' crate internally

License: Apache-2.0

Based on [rumqttc](https://github.com/bytebeamio/rumqtt)

# Usage

```rust
use bytes::BytesMut;
use mqtt_bytes_v5::{Packet, PubAck, PubAckReason, Error};

let mut buf: BytesMut = BytesMut::new();
let packet = Packet::PubAck(PubAck {
    pkid: 42,
    reason: PubAckReason::Success,
    properties: None,
});
let result: Result<usize, Error> = packet.write(&mut buf);
let result: Result<Packet, Error>  = Packet::read(&mut buf, None);
```

# Features

Configurable MqttString type:

- `default` - uses `String` for MqttString
- `boxed_string` - uses `Box<str>` for MqttString
- `binary_string` - uses `bytes::Bytes` for MqttString (about 20% faster than `String`)

# License

This project is released under The Apache License, Version 2.0 ([LICENSE](./LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
