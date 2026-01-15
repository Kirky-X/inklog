use inklog::archive::{ArchiveMetadata, CompressionType, ScheduleState, StorageClass};

#[test]
fn test_archive_metadata_creation() {
    let metadata = ArchiveMetadata::new(100, 50000, "json");

    assert_eq!(metadata.record_count, 100);
    assert_eq!(metadata.original_size, 50000);
    assert!(metadata.compressed_size >= 0);
    assert_eq!(metadata.archive_type, "json");
}

#[test]
fn test_archive_metadata_with_tag() {
    let metadata = ArchiveMetadata::new(50, 25000, "parquet")
        .with_tag("daily")
        .with_tag("automated");

    let tags: Vec<String> = metadata.tags.to_vec();
    assert!(tags.contains(&"daily".to_string()));
    assert!(tags.contains(&"automated".to_string()));
}

#[test]
fn test_archive_metadata_mark_success() {
    let metadata = ArchiveMetadata::new(100, 50000, "json");

    let result = metadata.mark_success();

    // 验证状态已更改
    match result.status {
        inklog::archive::ArchiveStatus::Success => {}
        _ => panic!("Expected Success status"),
    }
}

#[test]
fn test_archive_metadata_mark_failed() {
    let metadata = ArchiveMetadata::new(100, 50000, "json");

    let result = metadata.mark_failed();

    match result.status {
        inklog::archive::ArchiveStatus::Failed => {}
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_schedule_state_default() {
    let state = ScheduleState::default();

    assert!(state.last_scheduled_run.is_none());
    assert!(state.last_successful_run.is_none());
    assert!(state.last_run_status.is_none());
    assert_eq!(state.consecutive_failures, 0);
    assert!(state.locked_date.is_none());
    assert!(!state.is_running);
}

#[test]
fn test_schedule_state_start_execution() {
    let mut state = ScheduleState::default();

    state.start_execution();

    assert!(state.last_scheduled_run.is_some());
    assert!(state.locked_date.is_some());
    assert!(state.is_running);
}

#[test]
fn test_schedule_state_success() {
    let mut state = ScheduleState::default();

    state.start_execution();
    state.mark_success();

    assert_eq!(state.consecutive_failures, 0);
    assert!(state.last_successful_run.is_some());
    assert!(!state.is_running);
}

#[test]
fn test_schedule_state_failure() {
    let mut state = ScheduleState::default();

    state.start_execution();
    state.mark_failed();

    assert_eq!(state.consecutive_failures, 1);
    assert!(!state.is_running);
}

#[test]
fn test_schedule_state_consecutive_failures() {
    let mut state = ScheduleState::default();

    for _ in 0..3 {
        state.mark_failed();
    }

    assert_eq!(state.consecutive_failures, 3);
}

#[test]
fn test_compression_type_values() {
    // 测试 CompressionType 变体
    let _none = CompressionType::None;
    let _gzip = CompressionType::Gzip;
    let _zstd = CompressionType::Zstd;
    let _lz4 = CompressionType::Lz4;
    let _brotli = CompressionType::Brotli;
}

#[test]
fn test_storage_class_values() {
    // 测试 StorageClass 变体
    let _standard = StorageClass::Standard;
    let _standard_ia = StorageClass::StandardIa;
    let _glacier = StorageClass::Glacier;
}

#[test]
fn test_archive_metadata_parquet_type() {
    let metadata = ArchiveMetadata::new(100, 50000, "parquet");

    assert_eq!(metadata.archive_type, "parquet");
}

#[test]
fn test_schedule_state_reset_after_success() {
    let mut state = ScheduleState::default();

    state.mark_failed();
    state.mark_failed();
    assert_eq!(state.consecutive_failures, 2);

    state.mark_success();

    assert_eq!(state.consecutive_failures, 0);
}

#[test]
fn test_schedule_state_can_run_today() {
    let state = ScheduleState::default();

    assert!(state.can_run_today());
}

#[test]
fn test_schedule_state_cannot_run_when_locked() {
    let mut state = ScheduleState::default();

    state.start_execution();

    assert!(!state.can_run_today());
}
