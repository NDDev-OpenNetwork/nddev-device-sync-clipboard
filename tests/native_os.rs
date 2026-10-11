#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
#[tokio::test]
#[ignore = "requires an owned isolated native clipboard session; never run against unrelated live clipboard"]
async fn actual_native_clipboard_retains_text_for_its_owned_lifetime() {
    use nddev_device_sync_clipboard::os::TextClipboard;
    use zeroize::Zeroizing;
    assert_eq!(std::env::var("NDS_TEST_OWNED_CLIPBOARD").unwrap(), "1");
    let clipboard = TextClipboard::default();
    let text = "Actual native NDS clipboard: проверка";
    clipboard
        .write(Zeroizing::new(text.into()), false)
        .await
        .unwrap();
    let observed = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if arboard::Clipboard::new()
                .unwrap()
                .get_text()
                .is_ok_and(|value| value == text)
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(observed.is_ok());
    assert!(
        clipboard
            .write(Zeroizing::new("x".repeat(8193)), false)
            .await
            .is_err()
    );
}
