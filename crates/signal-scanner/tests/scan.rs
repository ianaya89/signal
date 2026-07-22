#![allow(clippy::unwrap_used)]

use std::io::Write;
use std::path::Path;

use signal_core::EventBus;
use signal_db::DbPool;
use signal_scanner::Scanner;

/// Minimal valid 16-bit/44.1kHz mono PCM WAV (0.1s of silence).
fn write_wav(path: &Path) {
    let sample_rate: u32 = 44_100;
    let samples: u32 = sample_rate / 10;
    let data_len = samples * 2;
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + data_len).to_le_bytes()).unwrap();
    f.write_all(b"WAVE").unwrap();
    f.write_all(b"fmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    f.write_all(&1u16.to_le_bytes()).unwrap(); // mono
    f.write_all(&sample_rate.to_le_bytes()).unwrap();
    f.write_all(&(sample_rate * 2).to_le_bytes()).unwrap(); // byte rate
    f.write_all(&2u16.to_le_bytes()).unwrap(); // block align
    f.write_all(&16u16.to_le_bytes()).unwrap(); // bits per sample
    f.write_all(b"data").unwrap();
    f.write_all(&data_len.to_le_bytes()).unwrap();
    f.write_all(&vec![0u8; data_len as usize]).unwrap();
}

#[tokio::test]
async fn full_scan_imports_wav_and_skips_rescan() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    std::fs::create_dir_all(&music).unwrap();
    write_wav(&music.join("silence.wav"));
    std::fs::write(music.join("notes.txt"), "not audio").unwrap();

    let db = DbPool::connect(&dir.path().join("test.db")).await.unwrap();
    let events = EventBus::default();
    let mut rx = events.subscribe();
    let scanner = Scanner::new(db.clone(), events, dir.path().join("cache"));

    let report = scanner.scan_full(music.clone()).await.unwrap();
    assert_eq!(report.added, 1);
    assert_eq!(report.errors, 0);

    // progress + done events were published
    let first = rx.recv().await.unwrap();
    assert_eq!(first.channel(), "scanner:progress");

    let track_id = db
        .tracks()
        .id_by_path(&music.join("silence.wav").to_string_lossy())
        .await
        .unwrap()
        .unwrap();
    let track = db.tracks().get(track_id).await.unwrap().unwrap();
    assert_eq!(track.title, "silence");
    assert_eq!(track.technical.sample_rate_hz, 44_100);
    assert_eq!(track.technical.bit_depth, Some(16));
    assert_eq!(track.technical.channels, 1);
    assert_eq!(track.technical.codec, "PCM (WAV)");

    // second scan: same file is skipped, nothing new added
    let rescan = scanner.scan_full(music).await.unwrap();
    assert_eq!(rescan.added, 0);
    assert_eq!(rescan.skipped, 1);

    let bad_root = scanner.scan_full(dir.path().join("missing")).await;
    assert!(bad_root.is_err());
}
