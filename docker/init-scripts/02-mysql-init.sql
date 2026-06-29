-- MySQL 初始化脚本
-- 用于创建日志表和索引

-- 创建 logs 表
CREATE TABLE IF NOT EXISTS logs (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    timestamp DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    level VARCHAR(10) NOT NULL,
    target VARCHAR(255),
    message TEXT NOT NULL,
    fields JSON DEFAULT ('{}'),
    file VARCHAR(512),
    line INT,
    thread_id VARCHAR(100),
    metadata JSON DEFAULT ('{}'),
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    INDEX idx_timestamp (timestamp),
    INDEX idx_level (level),
    INDEX idx_target (target),
    INDEX idx_thread_id (thread_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 添加注释
ALTER TABLE logs COMMENT = 'Inklog 日志记录表';
