use std::io::Read;
use std::sync::Arc;
use std::{io, thread};

use grpcio::{
    ChannelBuilder, ClientStreamingSink, Environment, RequestStream, ResourceQuota, RpcContext,
    ServerBuilder,
};
use kvproto::resource_usage_agent::{ResourceUsageAgent, CollectCpuTimeRequest, CollectCpuTimeResponse, create_resource_usage_agent};
use futures::channel::oneshot;
use futures::executor::block_on;
use futures::prelude::*;

#[derive(Clone)]
struct ResourceMeteringService;

impl ResourceUsageAgent for ResourceMeteringService {
    fn collect_cpu_time(
        &mut self,
        ctx: RpcContext,
        mut stream: RequestStream<CollectCpuTimeRequest>,
        sink: ClientStreamingSink<CollectCpuTimeResponse>,
    ) {
        let f = async move {
            while let Some(req) = stream.try_next().await? {
                println!("tag:{:?}", hex::encode_upper(req.get_resource_tag()));
                println!("ts_list:{:?}", req.get_timestamp_list());
                println!("cpu_time_list:{:?}", req.get_cpu_time_ms_list());
                println!("----------------------------------------------");
            }
            sink.success(CollectCpuTimeResponse::default()).await?;

            Ok(())
        }
        .map_err(|_e: grpcio::Error| {})
        .map(|_| {});
        ctx.spawn(f);
    }
}

fn main() {
    let env = Arc::new(Environment::new(1));
    let service = create_resource_usage_agent(ResourceMeteringService);

    let quota = ResourceQuota::new(Some("ResourceMeteringServerQuota")).resize_memory(1024 * 1024);
    let ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota);

    let mut server = ServerBuilder::new(env)
        .register_service(service)
        .bind("127.0.0.1", 10088)
        .channel_args(ch_builder.build_args())
        .build()
        .unwrap();
    server.start();
    for (host, port) in server.bind_addrs() {
        println!("listening on {}:{}", host, port);
    }
    let (tx, rx) = oneshot::channel();
    thread::spawn(move || {
        println!("Press ENTER to exit...");
        let _ = io::stdin().read(&mut [0]).unwrap();
        tx.send(())
    });
    let _ = block_on(rx);
    let _ = block_on(server.shutdown());
}
