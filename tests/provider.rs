#![cfg(unix)]

use nddev_device_sync_clipboard::{CancellationToken, Clipboard, NativeIo, ReadClipboard};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct OwnedDaemon(Child);
impl Drop for OwnedDaemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn exchange(stream: &mut BufReader<UnixStream>, request: Value) -> Value {
    writeln!(stream.get_mut(), "{request}").unwrap();
    response(stream)
}

fn response(stream: &mut BufReader<UnixStream>) -> Value {
    let mut line = String::new();
    stream.read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

#[tokio::test]
#[ignore = "requires NDS_TEST_CLIPBOARD_DAEMON pointing to a real installed native daemon"]
async fn real_private_archive_listing_and_binary_framing() {
    let executable =
        std::env::var_os("NDS_TEST_CLIPBOARD_DAEMON").expect("native daemon path required");
    assert!(std::path::Path::new(&executable).is_absolute());
    let fixture = tempfile::tempdir().unwrap();
    let mut daemon = OwnedDaemon(
        Command::new(executable)
            .env("RLDYOUR_CLIPBOARD_HOME", fixture.path())
            .env("RLDYOUR_CLIPBOARD_CAPTURE", "0")
            .env("RLDYOUR_CLIPBOARD_BUDGET", "8388608")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let socket = fixture.path().join("rldyour-clipboard.sock");
    let deadline = Instant::now() + Duration::from_secs(10);
    let connection = loop {
        match UnixStream::connect(&socket) {
            Ok(connection) => break connection,
            Err(_) => {
                assert!(Instant::now() < deadline, "native daemon startup timed out");
                assert!(
                    daemon.0.try_wait().unwrap().is_none(),
                    "native daemon startup failed"
                );
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    };
    connection
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    connection
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut stream = BufReader::new(connection);
    assert_eq!(
        exchange(&mut stream, json!({"op":"hello","v":1,"role":"capture"}))["ev"],
        "hello"
    );
    let draft = exchange(&mut stream, json!({"op":"begin","req":1}))["draft"]
        .as_u64()
        .unwrap();
    let content = "Isolated protocol acceptance: Русский / English\n".as_bytes();
    writeln!(stream.get_mut(), "{}", json!({"op":"part","req":2,"draft":draft,"mime":"text/plain;charset=utf-8","bytes":content.len()})).unwrap();
    stream.get_mut().write_all(content).unwrap();
    assert_eq!(response(&mut stream)["ev"], "ok");
    let entry = exchange(&mut stream, json!({"op":"commit","req":3,"draft":draft}))["entry"]
        .as_u64()
        .unwrap();
    let adapter = Clipboard::new(NativeIo::new(2).unwrap().local_handle(), socket).unwrap();
    let page = adapter.list(None, CancellationToken::new()).await.unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, entry);
    assert!(!page.more);
    let fetched = adapter
        .fetch(
            entry,
            "text/plain;charset=utf-8".into(),
            CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(fetched, content);
    assert!(matches!(
        adapter
            .fetch(entry + 1, "text/plain".into(), CancellationToken::new())
            .await,
        Err(nddev_device_sync_clipboard::Error::Provider)
    ));
}
