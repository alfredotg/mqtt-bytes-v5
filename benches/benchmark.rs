use bytes::BytesMut;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mqttbytes5::{Packet, Publish, PublishProperties, QoS};

pub fn publish_benchmark(c: &mut Criterion) {
    c.bench_function("publish", |b| {
        b.iter(|| {
            let pkg = Packet::Publish(Publish::new(
                "hello/world/this/is/a/very/long/topic",
                black_box(QoS::AtMostOnce),
                vec![1; 400],
                Some(PublishProperties {
                    payload_format_indicator: Some(1),
                    message_expiry_interval: Some(100),
                    topic_alias: Some(10),
                    response_topic: Some("response/topic".into()),
                    correlation_data: Some(vec![1, 2, 3].into()),
                    user_properties: vec![(
                        "key1".into(),
                        "valuevaluevaluevaluevaluevaluevaluevalue".into(),
                    )],
                    subscription_identifiers: vec![1, 2, 3, 4, 5, 6, 7],
                    content_type: Some("content/type".into()),
                }),
            ));
            for _ in 0..100 {
                let mut buf = BytesMut::new();
                pkg.write(&mut buf).unwrap();
                let _read = Packet::read(&mut buf, None).unwrap();
            }
        })
    });
}

criterion_group!(benches, publish_benchmark);
criterion_main!(benches);
