use super::{ArchiveMetadata, S3ArchiveConfig};
use chrono::Utc;

#[cfg(feature = "aws")]
use crate::error::InklogError;

#[cfg(feature = "aws")]
#[derive(Default)]
#[allow(dead_code)]
pub struct MockS3ArchiveManager {
    config: S3ArchiveConfig,
}

#[cfg(feature = "aws")]
impl MockS3ArchiveManager {
    pub async fn archive_logs(
        &self,
        _data: Vec<u8>,
        _start_date: chrono::DateTime<chrono::Utc>,
        _end_date: chrono::DateTime<Utc>,
        _metadata: ArchiveMetadata,
    ) -> Result<String, InklogError> {
        Ok(format!(
            "mock://logs/archive_{}.parquet",
            chrono::Utc::now().timestamp()
        ))
    }

    pub async fn list_archives(
        &self,
        _start_date: Option<chrono::DateTime<chrono::Utc>>,
        _end_date: Option<chrono::DateTime<chrono::Utc>>,
        _prefix: Option<&str>,
    ) -> Result<Vec<super::ArchiveInfo>, InklogError> {
        Ok(Vec::new())
    }

    pub async fn restore_archive(&self, _key: &str) -> Result<Vec<u8>, InklogError> {
        Ok(Vec::new())
    }

    pub async fn delete_archive(&self, _key: &str) -> Result<(), InklogError> {
        Ok(())
    }
}
