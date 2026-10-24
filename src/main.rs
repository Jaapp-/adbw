use std::{
    collections::HashSet,
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader, Read},
    process::{Child, Command, ExitCode, Stdio},
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

static DEBUG: AtomicBool = AtomicBool::new(false);
static START: OnceLock<Instant> = OnceLock::new();

fn debug(message: impl std::fmt::Display) {
    if DEBUG.load(Ordering::Relaxed) {
        eprintln!(
            "[debug +{:.3}s] {message}",
            START.get_or_init(Instant::now).elapsed().as_secs_f64()
        );
    }
}

#[derive(Default, Debug)]
struct Service {
    instance: String,
    kind: String,
    ipv4: String,
    ipv6: String,
    port: u16,
}

impl Service {
    fn address(&self) -> Option<String> {
        if self.port == 0 {
            None
        } else if !self.ipv4.is_empty() {
            Some(format!("{}:{}", self.ipv4, self.port))
        } else if !self.ipv6.is_empty() {
            Some(format!("[{}]:{}", self.ipv6, self.port))
        } else {
            None
        }
    }
}

// ADB emits protobuf text snapshots. Only the nested service block is needed;
// metadata such as known_device is deliberately ignored.
fn next_service(reader: &mut impl BufRead) -> io::Result<Option<Service>> {
    let mut service = None;
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            debug("tracker stdout EOF");
            return Ok(None);
        }
        debug(format_args!("tracker: {line:?}"));
        let line = line.trim();
        if line == "service {" {
            service = Some(Service::default());
        } else if line == "}" && service.is_some() {
            return Ok(service);
        } else if let Some(s) = service.as_mut()
            && let Some((key, value)) = line.split_once(':')
        {
            let value = value.trim().trim_matches('"');
            match key {
                "instance" => s.instance = value.to_owned(),
                "service" => s.kind = value.to_owned(),
                "ipv4" => s.ipv4 = value.to_owned(),
                "ipv6" => s.ipv6 = value.to_owned(),
                "port" => s.port = value.parse().unwrap_or(0),
                _ => {}
            }
        }
    }
}

struct Tracker(Child);

impl Drop for Tracker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[derive(PartialEq)]
enum Outcome {
    Success,
    AuthFailure,
    Unavailable,
}

fn needs_pairing(args: &[&str], output: &str) -> bool {
    let output = output.trim();
    output.starts_with("failed to authenticate to ")
        || matches!(args, ["connect", address] if output == format!("failed to connect to {address}"))
}

fn adb(args: &[&str], success: &[&str]) -> Result<Outcome> {
    let started = Instant::now();
    if args.first() == Some(&"pair") {
        debug(format_args!("exec: adb pair {} <redacted>", args[1]));
    } else {
        debug(format_args!("exec: adb {args:?}"));
    }
    let output = Command::new("adb").args(args).output()?;
    debug(format_args!(
        "adb finished after {:.3}s: status={}, stdout={} bytes, stderr={} bytes",
        started.elapsed().as_secs_f64(),
        output.status,
        output.stdout.len(),
        output.stderr.len()
    ));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Pair output is not needed to diagnose connection errors, and may contain secrets.
    if args.first() != Some(&"pair") {
        debug(format_args!("adb stdout: {stdout:?}"));
        debug(format_args!("adb stderr: {stderr:?}"));
    }
    if needs_pairing(args, &stdout) || needs_pairing(args, &stderr) {
        debug(
            "classification: pairing needed (explicit auth error or exact generic connect failure)",
        );
        return Ok(Outcome::AuthFailure);
    }
    debug("no pairing indication; checking success response");
    Ok(
        if output.status.success()
            && success
                .iter()
                .any(|prefix| stdout.trim().starts_with(prefix))
        {
            Outcome::Success
        } else {
            Outcome::Unavailable
        },
    )
}

fn connect(address: &str) -> Result<Outcome> {
    let outcome = connect_with_retries(
        || {
            adb(
                &["connect", address],
                &["connected to ", "already connected to "],
            )
        },
        std::thread::sleep,
    )?;
    if outcome == Outcome::Success {
        println!("Connected to {address}");
    }
    Ok(outcome)
}

fn connect_with_retries(
    mut attempt: impl FnMut() -> Result<Outcome>,
    mut sleep: impl FnMut(Duration),
) -> Result<Outcome> {
    let mut outcome = attempt()?;
    let first_finished = Instant::now();
    for deadline in [Duration::from_millis(200), Duration::from_millis(800)] {
        if outcome == Outcome::Success {
            break;
        }
        sleep(deadline.saturating_sub(first_finished.elapsed()));
        debug(format_args!(
            "retrying connection (+{}ms)",
            deadline.as_millis()
        ));
        outcome = attempt()?;
    }
    Ok(outcome)
}

fn pairing_qr() -> Result<(String, String)> {
    // Linux's OS random source; hex needs no WIFI payload escaping.
    let mut bytes = [0u8; 24];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let name = format!("studio-{}", hex(&bytes[..8]));
    let password = hex(&bytes[8..]);
    let qr = qrcode::QrCode::new(format!("WIFI:T:ADB;S:{name};P:{password};;"))?;
    println!("Scan in Wireless debugging > Pair device with QR code:");
    println!(
        "{}",
        qr.render::<qrcode::render::unicode::Dense1x2>()
            .dark_color(qrcode::render::unicode::Dense1x2::Light)
            .light_color(qrcode::render::unicode::Dense1x2::Dark)
            .build()
    );
    Ok((name, password))
}

fn run() -> Result<()> {
    START.get_or_init(Instant::now);
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !args.is_empty() && (args.len() != 1 || args[0] != "--debug") {
        return Err("usage: adbw [--debug] (requires adb mdns track-services support)".into());
    }
    DEBUG.store(!args.is_empty(), Ordering::Relaxed);
    if DEBUG.load(Ordering::Relaxed) {
        let version = Command::new("adb").arg("version").output()?;
        debug(format_args!(
            "adb version: {}",
            String::from_utf8_lossy(&version.stdout).trim()
        ));
        for key in [
            "ADB_SERVER_SOCKET",
            "ANDROID_ADB_SERVER_ADDRESS",
            "ANDROID_ADB_SERVER_PORT",
            "ADB_MDNS_AUTO_CONNECT",
            "ADB_MDNS_OPENSCREEN",
            "ADB_MDNS",
        ] {
            debug(format_args!("{key}={:?}", std::env::var_os(key)));
        }
    }
    debug("starting adb mdns track-services --proto-text (stderr inherited)");
    let mut tracker = Tracker(
        Command::new("adb")
            .args(["mdns", "track-services", "--proto-text"])
            .stdout(Stdio::piped())
            .stderr(if DEBUG.load(Ordering::Relaxed) {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .spawn()?,
    );
    let mut reader = BufReader::new(tracker.0.stdout.take().unwrap());
    let mut target = None;
    let mut target_instance = None;
    let mut attempted = HashSet::new();
    let mut pairing = None;
    while let Some(service) = next_service(&mut reader)? {
        debug(format_args!("resolved service: {service:?}"));
        let Some(address) = service.address() else {
            debug("ignoring service without usable address/port");
            continue;
        };
        debug(format_args!(
            "selection: selected={target_instance:?}, endpoint={address}, duplicate={}, suffix_skip={}",
            attempted.contains(&address),
            service.instance.ends_with(" (2)")
        ));
        if service.kind == "_adb-tls-connect._tcp"
            && !service.instance.ends_with(" (2)")
            && target_instance
                .as_ref()
                .is_none_or(|name| *name == service.instance)
            && attempted.insert(address.clone())
        {
            target_instance = Some(service.instance);
            let outcome = connect(&address)?;
            target = Some(address);
            match outcome {
                Outcome::Success => return Ok(()),
                Outcome::AuthFailure if pairing.is_none() => pairing = Some(pairing_qr()?),
                _ => {}
            }
        } else if let Some((name, password)) = pairing.as_ref()
            && service.kind == "_adb-tls-pairing._tcp"
            && service.instance == *name
        {
            if adb(&["pair", &address, password], &["Successfully paired"])? != Outcome::Success {
                return Err("pairing failed".into());
            }
            if connect(target.as_deref().unwrap())? == Outcome::Success {
                return Ok(());
            }
            return Err("paired, but connection failed; run adbw again".into());
        }
    }
    Err("ADB discovery stream ended; adb must support mdns track-services --proto-text".into())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("adbw: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_retries_stop_on_success_and_are_bounded() {
        for successes_on in [1, 2, 3, 4] {
            let mut calls = 0;
            let mut delays = Vec::new();
            let outcome = connect_with_retries(
                || {
                    calls += 1;
                    Ok(if calls == successes_on {
                        Outcome::Success
                    } else {
                        Outcome::Unavailable
                    })
                },
                |delay| delays.push(delay),
            )
            .unwrap();
            assert_eq!(calls, successes_on.min(3));
            assert_eq!(delays.len(), calls - 1);
            assert_eq!(outcome == Outcome::Success, successes_on <= 3);
            for (delay, expected) in delays.iter().zip([200, 800]) {
                assert!(*delay <= Duration::from_millis(expected));
                assert!(*delay > Duration::from_millis(expected - 50));
            }
        }
    }

    #[test]
    fn pairing_is_only_triggered_by_auth_or_exact_generic_connect_failure() {
        let args = ["connect", "192.168.8.177:36871"];
        for output in [
            "failed to connect to 192.168.8.177:36871\n",
            "failed to authenticate to 192.168.8.177:36871\n",
        ] {
            assert!(needs_pairing(&args, output), "{output}");
        }
        for output in [
            "failed to connect to '192.168.8.177:36871': Connection refused\n",
            "failed to connect to '192.168.8.177:36871': Connection timed out\n",
            "failed to connect to 192.168.8.177:36871: timeout\n",
            "failed to connect to 192.168.8.177:12345\n",
            "connected to 192.168.8.177:36871\n",
        ] {
            assert!(!needs_pairing(&args, output), "{output}");
        }
        assert!(!needs_pairing(
            &["pair", args[1], "secret"],
            "failed to connect to 192.168.8.177:36871"
        ));
    }

    #[test]
    fn reads_services_across_snapshots_and_ignores_metadata() {
        let mut stream = io::Cursor::new(concat!(
            "\nServices:\ntls {\n  service {\n",
            "    instance: \"adb-phone (2)\"\n",
            "    service: \"_adb-tls-connect._tcp\"\n",
            "    ipv4: \"192.168.8.177\"\n    port: 33997\n",
            "  }\n  known_device: true\n}\n",
            "\nServices:\npairing {\n  service {\n",
            "    instance: \"studio-test\"\n",
            "    service: \"_adb-tls-pairing._tcp\"\n",
            "    ipv6: \"::1\"\n    port: 12345\n  }\n}\n"
        ));
        let first = next_service(&mut stream).unwrap().unwrap();
        assert_eq!(first.instance, "adb-phone (2)");
        assert_eq!(first.address().as_deref(), Some("192.168.8.177:33997"));
        let second = next_service(&mut stream).unwrap().unwrap();
        assert_eq!(second.kind, "_adb-tls-pairing._tcp");
        assert_eq!(second.instance, "studio-test");
        assert_eq!(second.address().as_deref(), Some("[::1]:12345"));
        assert!(next_service(&mut stream).unwrap().is_none());
    }

    #[test]
    fn incomplete_or_unresolved_services_cannot_supply_an_endpoint() {
        let mut stream = io::Cursor::new("tls {\nservice {\ninstance: \"phone\"\n");
        assert!(next_service(&mut stream).unwrap().is_none());
        assert!(Service::default().address().is_none());
        assert!(
            Service {
                port: 1234,
                ..Service::default()
            }
            .address()
            .is_none()
        );
    }
}
