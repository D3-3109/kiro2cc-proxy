// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 文件写入工具

/// rename 覆盖目标失败是否属于「目标是挂载点 / 跨设备」类错误。
///
/// 目标文件是单文件 bind mount（Docker `-v ./config.json:/app/config/config.json`、
/// 部分平台的「配置文件挂载」）时，rename 覆盖挂载点会返回 EBUSY（`Resource busy`，
/// os error 16）；跨设备为 EXDEV（18）。此时只能原地写入。
pub(crate) fn is_replace_blocked(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::ResourceBusy | std::io::ErrorKind::CrossesDevices
    ) || matches!(e.raw_os_error(), Some(16) | Some(18))
}

/// 原地写文件（截断后写入 + fsync）：保留 inode 与权限，可用于挂载点文件。
/// 非原子——仅作为 rename 不可用时的兜底。
pub(crate) fn write_in_place(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// 原子写文件：写临时文件 → fsync → rename 替换 → fsync 父目录。
/// 同目录 rename 在 POSIX 上是原子操作，避免写半截导致目标文件损坏。
/// 目标为挂载点导致 rename 失败（EBUSY / EXDEV）时降级为原地写入。
pub(crate) fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));

    // 写临时文件并落盘
    {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }

    // 原子替换目标文件
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp); // 清理残留临时文件
        if is_replace_blocked(&e) {
            tracing::warn!(
                "rename 覆盖 {} 失败（{}），降级为原地写入",
                path.display(),
                e
            );
            return write_in_place(path, bytes);
        }
        return Err(e);
    }

    // fsync 父目录，确保 rename 元数据落盘（容器持久化卷必须）
    if let Ok(dir_file) = std::fs::File::open(dir) {
        let _ = dir_file.sync_all();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_replace_blocked_detects_ebusy_and_exdev() {
        assert!(is_replace_blocked(&std::io::Error::from_raw_os_error(16)));
        assert!(is_replace_blocked(&std::io::Error::from_raw_os_error(18)));
        assert!(!is_replace_blocked(&std::io::Error::from_raw_os_error(13)));
        assert!(!is_replace_blocked(&std::io::Error::from(
            std::io::ErrorKind::NotFound
        )));
    }

    #[test]
    fn write_in_place_truncates_and_keeps_inode() {
        let dir = std::env::temp_dir().join(format!("kiro-fs-inplace-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.json");
        std::fs::write(&path, "0123456789-long-old-content").unwrap();
        #[cfg(unix)]
        let inode_before = {
            use std::os::unix::fs::MetadataExt;
            std::fs::metadata(&path).unwrap().ino()
        };

        write_in_place(&path, b"short").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "short");
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(std::fs::metadata(&path).unwrap().ino(), inode_before);
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
