-- PostgreSQL 初始化脚本
-- 用于创建日志表和索引

-- 创建 logs 表
CREATE TABLE IF NOT EXISTS logs (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    level VARCHAR(10) NOT NULL,
    target VARCHAR(255),
    message TEXT NOT NULL,
    fields JSONB DEFAULT '{}',
    file VARCHAR(512),
    line INTEGER,
    thread_id VARCHAR(100),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_logs_timestamp ON logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_logs_level ON logs(level);
CREATE INDEX IF NOT EXISTS idx_logs_target ON logs(target);
CREATE INDEX IF NOT EXISTS idx_logs_thread_id ON logs(thread_id);

-- 创建用于归档的分表（月分区示例）
CREATE TABLE IF NOT EXISTS logs_archive (
    LIKE logs INCLUDING ALL
);

-- 添加注释
COMMENT ON TABLE logs IS 'Inklog 日志记录表';
COMMENT ON COLUMN logs.id IS '日志唯一标识';
COMMENT ON COLUMN logs.timestamp IS '日志时间戳';
COMMENT ON COLUMN logs.level IS '日志级别 (trace, debug, info, warn, error)';
COMMENT ON COLUMN logs.target IS '日志目标模块';
COMMENT ON COLUMN logs.message IS '日志消息内容';
COMMENT ON COLUMN logs.fields IS '额外字段 (JSON)';
COMMENT ON COLUMN logs.file IS '源文件路径';
COMMENT ON COLUMN logs.line IS '源文件行号';
COMMENT ON COLUMN logs.thread_id IS '线程 ID';
COMMENT ON COLUMN logs.metadata IS '元数据 (JSON)';
