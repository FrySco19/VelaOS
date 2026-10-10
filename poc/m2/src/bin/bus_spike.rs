use iceoryx2::port::update_connections::UpdateConnections;
// Synthetic end-to-end fixture. Metadata on control plane; bytes on real iceoryx2 IPC.
use anyhow::{Result, anyhow, ensure};
use iceoryx2::prelude::*;
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use vela_lfs_m1::{Lfs, fixture};
use vela_neural_bus_m2::{Endpoint, Frame, PolicyGate, Schema};

fn receive(channel: &str, count: usize) -> Result<()> {
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    let endpoint: Endpoint = serde_json::from_str(&line)?;
    let node = NodeBuilder::new().create::<ipc::Service>()?;
    let forward = node
        .service_builder(&channel.try_into()?)
        .publish_subscribe::<Frame>()
        .open_or_create()?;
    let subscriber = forward.subscriber_builder().create()?;
    let ack_name = format!("{channel}-ack");
    let backward = node
        .service_builder(&ack_name.as_str().try_into()?)
        .publish_subscribe::<u64>()
        .open_or_create()?;
    let publisher = backward.publisher_builder().create()?;
    println!("READY {}", std::process::id());
    std::io::stdout().flush()?;
    let started = Instant::now();
    let mut digest = blake3::Hasher::new();
    for expected in 0..count {
        loop {
            if let Some(sample) = subscriber.receive()? {
                let bytes = endpoint.verify(sample.payload(), expected as u64)?;
                // Borrow the received shared-memory slice. No deserialization of bulk bytes.
                digest.update(bytes);
                publisher
                    .loan_uninit()?
                    .write_payload(expected as u64)
                    .send()?;
                break;
            }
            ensure!(
                started.elapsed() < Duration::from_secs(110),
                "IPC receiver timeout"
            );
            std::hint::spin_loop();
        }
    }
    println!("DONE {}", digest.finalize().to_hex());
    std::io::stdout().flush()?;
    let mut confirmation = String::new();
    std::io::stdin().read_line(&mut confirmation)?;
    ensure!(
        confirmation.trim() == "ACKNOWLEDGED",
        "missing final acknowledgment"
    );
    Ok(())
}
fn transfer(
    route: &vela_neural_bus_m2::Route,
    endpoint: Endpoint,
    payload: &[u8],
    count: usize,
) -> Result<serde_json::Value> {
    ensure!(payload.len() <= 4096, "fixture exceeds frame");
    let node = NodeBuilder::new().create::<ipc::Service>()?;
    let forward = node
        .service_builder(&route.channel.as_str().try_into()?)
        .publish_subscribe::<Frame>()
        .open_or_create()?;
    let publisher = forward.publisher_builder().create()?;
    let ack_name = format!("{}-ack", route.channel);
    let backward = node
        .service_builder(&ack_name.as_str().try_into()?)
        .publish_subscribe::<u64>()
        .open_or_create()?;
    let subscriber = backward.subscriber_builder().create()?;
    let mut child = Command::new(std::env::current_exe()?)
        .arg("receive")
        .arg(&route.channel)
        .arg(count.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    writeln!(
        child.stdin.as_mut().unwrap(),
        "{}",
        serde_json::to_string(&endpoint)?
    )?;
    child.stdin.as_mut().unwrap().flush()?;
    let mut control = BufReader::new(child.stdout.take().unwrap());
    let mut ready = String::new();
    control.read_line(&mut ready)?;
    ensure!(ready.starts_with("READY "), "IPC receiver startup failed");
    let receiver_pid = ready.split_whitespace().nth(1).unwrap().parse::<u32>()?;
    publisher.update_connections()?;
    let mut timings = vec![];
    let mut expected_digest = blake3::Hasher::new();
    let began = Instant::now();
    for seq in 0..count {
        let started = Instant::now();
        let mut sample = publisher.loan_uninit()?.write_payload(Frame::empty());
        sample.payload_mut().bytes[..payload.len()].copy_from_slice(payload);
        endpoint.stamp(sample.payload_mut(), seq as u64, payload.len())?;
        ensure!(
            sample.send()? == 1,
            "expected exactly one authorized fixture subscriber"
        );
        loop {
            if let Some(ack) = subscriber.receive()? {
                ensure!(*ack.payload() == seq as u64, "ack sequence mismatch");
                break;
            }
            ensure!(
                started.elapsed() < Duration::from_secs(10),
                "IPC sender timeout"
            );
            std::hint::spin_loop();
        }
        timings.push(started.elapsed().as_secs_f64() * 1e6);
        expected_digest.update(payload);
    }
    let elapsed = began.elapsed().as_secs_f64();
    let mut done = String::new();
    control.read_line(&mut done)?;
    writeln!(child.stdin.as_mut().unwrap(), "ACKNOWLEDGED")?;
    child.stdin.as_mut().unwrap().flush()?;
    ensure!(child.wait()?.success(), "receiver failed");
    ensure!(
        done.trim() == format!("DONE {}", expected_digest.finalize().to_hex()),
        "receiver digest differs"
    );
    timings.sort_by(f64::total_cmp);
    let p99 = timings[((timings.len() - 1) as f64 * 0.99).ceil() as usize];
    Ok(
        serde_json::json!({"messages":count,"payload_bytes":payload.len(),"sender_pid":std::process::id(),"receiver_pid":receiver_pid,
        "elapsed_seconds":elapsed,"messages_per_second":count as f64/elapsed,"roundtrip_p99_us":p99,
        "ipc":"iceoryx2::ipc::Service","transport_zero_copy":true,"storage_to_loan_staging_copy":true,
        "digest_verified":true,"metric":"sequential authenticated 4KiB ping/ack, not pipelined throughput"}),
    )
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_some_and(|s| s == "receive") {
        return receive(
            args[2].to_str().unwrap(),
            args[3].to_str().unwrap().parse()?,
        );
    }
    ensure!(
        args.len() >= 4,
        "usage: bus-spike PYTHON M0_ADAPTER OUTPUT_REPORT [MESSAGES]"
    );
    let count = args
        .get(4)
        .and_then(|v| v.to_str())
        .unwrap_or("10000")
        .parse::<usize>()?;
    ensure!((1..=100000).contains(&count), "message count out of bounds");
    let temp = tempfile::tempdir()?;
    let token = fixture::token("bridge", &["read", "write", "list"])?;
    let mut lfs = Lfs::unlock(
        &temp.path().join("fixture.redb"),
        fixture::COLLECTION,
        fixture::root().public(),
    )?;
    let configs = [
        ("ZEPHYR-731", "9.9.9.9"),
        ("CafeGuest-5G", "1.1.1.1"),
        ("HomeLan-5G", "8.8.8.8"),
        ("StudioLan", "192.168.1.1"),
        ("TravelGuest", "1.0.0.1"),
    ];
    let mut expected = None;
    for (i, (ssid, dns)) in configs.iter().enumerate() {
        let done = lfs.put(
            &token,
            "bridge",
            &format!("save-{i}"),
            &serde_json::json!({"schema":"vela.net.config.v1","ssid":ssid,"dns":dns}),
        )?;
        if *ssid == "CafeGuest-5G" {
            expected = Some(done.oid);
        }
    }
    let cards = lfs.list(&token, "bridge", 0, 100)?;
    // Capability-filtered cards only: no object payload is sent to the M0 adapter.
    let mut recall = Command::new(&args[1])
        .arg("-X")
        .arg("utf8")
        .arg(&args[2])
        .env("HF_HUB_OFFLINE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    writeln!(
        recall.stdin.as_mut().unwrap(),
        "{}",
        serde_json::json!({"schema":"vela.recall.cards.v1","query":"CafeGuest-5G","cards":cards})
    )?;
    let recalled = recall.wait_with_output()?;
    ensure!(recalled.status.success(), "M0 recall adapter failed");
    let selected: serde_json::Value = serde_json::from_slice(&recalled.stdout)?;
    ensure!(
        selected["auto_resolve"] == false,
        "recall must not resolve bytes"
    );
    let selected_oid = selected["candidates"][0]["oid"]
        .as_str()
        .ok_or_else(|| anyhow!("no recall candidate"))?;
    ensure!(
        Some(selected_oid) == expected.as_deref(),
        "retrieved different typed object"
    );
    // Explicit fixture selection, not an assertion of calibrated automatic recall.
    let handle = lfs.handle(&token, "bridge", selected_oid, 120)?;
    let bytes = lfs.open(&token, "bridge", &handle)?;
    let gate = PolicyGate::new(fixture::root().public());
    let route = gate.issue(
        &token,
        "bridge",
        Schema::NetConfigV1,
        Duration::from_secs(120),
    )?;
    let endpoint = gate.endpoint(&route, "bridge", Schema::NetConfigV1)?;
    let e2e = transfer(&route, endpoint, &bytes, 1)?;
    let bulk_token = biscuit_auth::builder::BiscuitBuilder::new()
        .code("right(\"bridge\",\"Bulk4KiB\",\"read\");")?
        .build(&fixture::root())?
        .to_vec()?;
    let bulk = gate.issue(
        &bulk_token,
        "bridge",
        Schema::Bulk4KiB,
        Duration::from_secs(120),
    )?;
    let endpoint = gate.endpoint(&bulk, "bridge", Schema::Bulk4KiB)?;
    let performance = transfer(&bulk, endpoint, &[73; 4096], count)?;
    let report = serde_json::json!({"m0_to_m1_bridge":true,"selected_oid":selected_oid,"recall":selected,"typed_object_transfer":e2e,
        "data_plane":performance,"llm_invocations":0,"bulk_bytes_on_control_plane":false,
        "m2_gate_passed":false,"limit":"IPC and capability fixture verified; OS/sandbox isolation and independent pipelined performance gate are not established."});
    std::fs::write(Path::new(&args[3]), serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}
