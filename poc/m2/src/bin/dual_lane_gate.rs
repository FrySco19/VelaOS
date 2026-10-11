//! Real cross-process credit-based IPC benchmark. Does not certify native peer isolation.
use anyhow::{Result, anyhow, ensure};
use iceoryx2::{port::update_connections::UpdateConnections, prelude::*};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use vela_neural_bus_m2::{
    Endpoint, PolicyGate, Schema,
    streaming::{BULK_BLOCK, ChunkHeader, CreditAck, CreditWindow, MAX_CREDITS},
};

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn receive(name: &str, total: u64, block: usize, slow_us: u64) -> Result<()> {
    let mut credential = String::new();
    std::io::stdin().read_line(&mut credential)?;
    let endpoint: Endpoint = serde_json::from_str(&credential)?;
    let node = NodeBuilder::new().create::<ipc::Service>()?;
    let forward = node
        .service_builder(&name.try_into()?)
        .publish_subscribe::<[u8]>()
        .user_header::<ChunkHeader>()
        .max_publishers(1)
        .max_subscribers(1)
        .max_nodes(2)
        .subscriber_max_buffer_size(MAX_CREDITS)
        .enable_safe_overflow(false)
        .open()?;
    let subscriber = forward
        .subscriber_builder()
        .buffer_size(MAX_CREDITS)
        .create()?;
    let ack_name = format!("{name}-credit");
    let backward = node
        .service_builder(&ack_name.as_str().try_into()?)
        .publish_subscribe::<CreditAck>()
        .max_publishers(1)
        .max_subscribers(1)
        .max_nodes(2)
        .subscriber_max_buffer_size(MAX_CREDITS)
        .enable_safe_overflow(false)
        .open()?;
    let publisher = backward
        .publisher_builder()
        .max_loaned_samples(MAX_CREDITS)
        .backpressure_strategy(BackpressureStrategy::DiscardData)
        .create()?;
    publisher.update_connections()?;
    println!("READY {}", std::process::id());
    std::io::stdout().flush()?;
    let started = Instant::now();
    let mut offset = 0_u64;
    let mut sequence = 0;
    let mut digest = blake3::Hasher::new();
    while offset < total {
        if let Some(sample) = subscriber.receive()? {
            let expected = (total - offset).min(block as u64) as usize;
            ensure!(sample.payload().len() == expected, "unexpected chunk size");
            endpoint.verify_chunk(sample.user_header(), sequence, offset, sample.payload())?;
            digest.update(sample.payload());
            offset += expected as u64;
            drop(sample); // The received shared-memory loan is released before issuing credit.
            if slow_us > 0 {
                std::thread::sleep(Duration::from_micros(slow_us));
            }
            ensure!(
                publisher
                    .loan_uninit()?
                    .write_payload(endpoint.credit_ack(sequence)?)
                    .send()?
                    == 1,
                "credit receiver disconnected"
            );
            sequence += 1;
        } else {
            ensure!(
                started.elapsed() < Duration::from_secs(110),
                "receiver deadline"
            );
            std::hint::spin_loop();
        }
    }
    println!("DONE {} {}", offset, digest.finalize().to_hex());
    std::io::stdout().flush()?;
    let mut finish = String::new();
    std::io::stdin().read_line(&mut finish)?;
    ensure!(
        finish.trim() == "ACKNOWLEDGED",
        "missing completion acknowledgement"
    );
    Ok(())
}

fn run_lane(
    label: &str,
    total: u64,
    block: usize,
    slow_us: u64,
    ready_flag: Option<Arc<AtomicBool>>,
) -> Result<serde_json::Value> {
    ensure!(
        total > 0 && block > 0 && block <= BULK_BLOCK,
        "invalid transfer"
    );
    let root = biscuit_auth::KeyPair::new();
    let token = biscuit_auth::builder::BiscuitBuilder::new()
        .code("right(\"gate\",\"BulkStream\",\"read\");")?
        .build(&root)?
        .to_vec()?;
    let gate = PolicyGate::new(root.public());
    let route = gate.issue(
        &token,
        "gate",
        Schema::BulkStreamV1,
        Duration::from_secs(120),
    )?;
    let endpoint = gate.endpoint(&route, "gate", Schema::BulkStreamV1)?;
    let node = NodeBuilder::new().create::<ipc::Service>()?;
    let forward = node
        .service_builder(&route.channel.as_str().try_into()?)
        .publish_subscribe::<[u8]>()
        .user_header::<ChunkHeader>()
        .max_publishers(1)
        .max_subscribers(1)
        .max_nodes(2)
        .subscriber_max_buffer_size(MAX_CREDITS)
        .enable_safe_overflow(false)
        .create()?;
    let publisher = forward
        .publisher_builder()
        .initial_max_slice_len(block)
        .max_loaned_samples(MAX_CREDITS)
        .backpressure_strategy(BackpressureStrategy::DiscardData)
        .create()?;
    let ack_name = format!("{}-credit", route.channel);
    let backward = node
        .service_builder(&ack_name.as_str().try_into()?)
        .publish_subscribe::<CreditAck>()
        .max_publishers(1)
        .max_subscribers(1)
        .max_nodes(2)
        .subscriber_max_buffer_size(MAX_CREDITS)
        .enable_safe_overflow(false)
        .create()?;
    let subscriber = backward
        .subscriber_builder()
        .buffer_size(MAX_CREDITS)
        .create()?;
    let mut child = ChildGuard(
        Command::new(std::env::current_exe()?)
            .arg("receive")
            .arg(&route.channel)
            .arg(total.to_string())
            .arg(block.to_string())
            .arg(slow_us.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?,
    );
    writeln!(
        child
            .0
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("missing control input"))?,
        "{}",
        serde_json::to_string(&endpoint)?
    )?;
    child.0.stdin.as_mut().unwrap().flush()?;
    let mut control = BufReader::new(child.0.stdout.take().unwrap());
    let mut ready = String::new();
    control.read_line(&mut ready)?;
    ensure!(ready.starts_with("READY "), "receiver startup failed");
    let receiver_pid: u32 = ready
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow!("missing PID"))?
        .parse()?;
    publisher.update_connections()?;
    let mut credits = CreditWindow::new(total)?;
    let mut pending = BTreeMap::new();
    let mut timings = Vec::new();
    let mut digest = blake3::Hasher::new();
    let start = Instant::now();
    if let Some(flag) = ready_flag {
        flag.store(true, Ordering::Release);
    }
    while credits.charged() < total || !credits.drained() {
        while credits.available() && credits.charged() < total {
            let length = (total - credits.charged()).min(block as u64) as usize;
            let (sequence, offset) = credits.issue(length)?;
            let sent_at = Instant::now();
            let mut loan = publisher
                .loan_slice_uninit(length)?
                .write_from_fn(|_| sequence as u8);
            *loan.user_header_mut() = endpoint.seal_chunk(sequence, offset, loan.payload())?;
            digest.update(loan.payload());
            ensure!(loan.send()? == 1, "lane receiver missing or queue overflow");
            pending.insert(sequence, sent_at);
        }
        while let Some(sample) = subscriber.receive()? {
            let ack = *sample.payload();
            endpoint.verify_credit(&ack)?;
            credits.acknowledge(ack.sequence)?;
            let sent_at = pending
                .remove(&ack.sequence)
                .ok_or_else(|| anyhow!("unknown credit timestamp"))?;
            // Latency telemetry is capped, independent of file size.
            if timings.len() < 100_000 {
                timings.push(sent_at.elapsed().as_secs_f64() * 1e6);
            }
        }
        ensure!(
            start.elapsed() < Duration::from_secs(110),
            "sender deadline"
        );
        std::hint::spin_loop();
    }
    let elapsed = start.elapsed().as_secs_f64();
    let mut done = String::new();
    control.read_line(&mut done)?;
    ensure!(
        done.trim() == format!("DONE {total} {}", digest.finalize().to_hex()),
        "stream digest differs"
    );
    writeln!(child.0.stdin.as_mut().unwrap(), "ACKNOWLEDGED")?;
    child.0.stdin.as_mut().unwrap().flush()?;
    ensure!(child.0.wait()?.success(), "receiver failed");
    timings.sort_by(f64::total_cmp);
    let p99 = timings[((timings.len() - 1) as f64 * 0.99).ceil() as usize];
    let messages = total.div_ceil(block as u64);
    Ok(
        serde_json::json!({"lane":label,"bytes":total,"block_bytes":block,"messages":messages,
        "elapsed_seconds":elapsed,"messages_per_second":messages as f64/elapsed,
        "bytes_per_second":total as f64/elapsed,"roundtrip_p99_us":p99,"samples":timings.len(),
        "latency_scope":"from loan construction to authenticated receiver credit; window=8",
        "sender_pid":std::process::id(),"receiver_pid":receiver_pid,"receiver_delay_us":slow_us,
        "digest_verified":true,"max_inflight_payload_bytes":MAX_CREDITS*block,
        "backpressure":"eight credits, non-overflowing queues, DiscardData treated as fatal error",
        "transport":"iceoryx2::ipc::Service","source":"deterministic generated bytes, not durable LFS file",
        "llm_calls":0}),
    )
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "receive") {
        return receive(
            &args[2],
            args[3].parse()?,
            args[4].parse()?,
            args[5].parse()?,
        );
    }
    ensure!(args.len() == 2, "usage: dual-lane-gate OUTPUT_REPORT");
    let fast_bytes = 100_000_u64 * 4096;
    let baseline = run_lane("fast-baseline", fast_bytes, 4096, 0, None)?;
    let started = Arc::new(AtomicBool::new(false));
    let flag = started.clone();
    let bulk = std::thread::spawn(move || {
        run_lane(
            "bulk-concurrent",
            10_000_000_000,
            BULK_BLOCK,
            100,
            Some(flag),
        )
    });
    let waiting = Instant::now();
    while !started.load(Ordering::Acquire) {
        ensure!(!bulk.is_finished(), "bulk startup failed");
        ensure!(
            waiting.elapsed() < Duration::from_secs(15),
            "bulk startup deadline"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    // Join the bulk worker even when the fast lane fails: its child must be reaped.
    let concurrent_result = run_lane("fast-concurrent", fast_bytes, 4096, 0, None);
    let bulk_report = bulk.join().map_err(|_| anyhow!("bulk worker panicked"))??;
    let concurrent = concurrent_result?;
    let base_p99 = baseline["roundtrip_p99_us"].as_f64().unwrap();
    let regression = concurrent["roundtrip_p99_us"].as_f64().unwrap() / base_p99 - 1.0;
    let performance_passed = baseline["messages_per_second"].as_f64().unwrap() > 100_000.0
        && base_p99 < 100.0
        && concurrent["messages_per_second"].as_f64().unwrap() > 100_000.0
        && concurrent["roundtrip_p99_us"].as_f64().unwrap() < 100.0
        && regression <= 0.20;
    let report = serde_json::json!({"baseline":baseline,"concurrent_fast":concurrent,"bulk":bulk_report,
        "fast_p99_regression":regression,"performance_passed":performance_passed,
        "m2_gate_closed":false,"limitations":["trusted native peers; no OS-wide read isolation proof",
        "generated data stream, durable large-object LFS writer still required",
        "no full-OS RAM qualification; pool backing exceeds eight in-flight payloads",
        "single measured run; CPU contention and repetition require qualification"]});
    std::fs::write(&args[1], serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}
