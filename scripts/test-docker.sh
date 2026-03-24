#!/bin/bash
# Inklog 集成测试 Docker 环境管理脚本
#
# 功能：
#   - 启动/停止测试环境中的外部服务
#   - 运行集成测试
#   - 清理测试资源
#
# 使用方式:
#   ./scripts/test-docker.sh start    # 启动测试环境
#   ./scripts/test-docker.sh stop     # 停止并清理
#   ./scripts/test-docker.sh test     # 运行集成测试（自动启动环境）
#   ./scripts/test-docker.sh status    # 查看服务状态
#   ./scripts/test-docker.sh logs     # 查看服务日志

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 脚本目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
COMPOSE_FILE="$PROJECT_ROOT/docker-compose.test.yml"
ENV_FILE="$PROJECT_ROOT/.env.test"

# 默认配置
COMPOSE_PROJECT_NAME="inklog_test"
WAIT_TIMEOUT=120  # 等待服务就绪的超时时间（秒）
TEST_FEATURES="dbnexus,aws"

# ============ 日志函数 ============

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# ============ 检查函数 ============

check_docker() {
    if ! command -v docker &> /dev/null; then
        log_error "Docker 未安装或不在 PATH 中"
        exit 1
    fi

    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        log_error "Docker Compose 未安装或不在 PATH 中"
        exit 1
    fi

    # 检查 Docker daemon 是否运行
    if ! docker info &> /dev/null; then
        log_error "Docker daemon 未运行，请先启动 Docker"
        exit 1
    fi
}

check_ports() {
    local ports=("5432" "3306" "6379" "4566" "9000")
    local occupied_ports=()

    for port in "${ports[@]}"; do
        if netstat -tuln 2>/dev/null | grep -q ":$port " || ss -tuln 2>/dev/null | grep -q ":$port "; then
            occupied_ports+=("$port")
        fi
    done

    if [ ${#occupied_ports[@]} -gt 0 ]; then
        log_warning "以下端口已被占用: ${occupied_ports[*]}"
        log_warning "测试服务可能无法启动或与其他服务冲突"
    fi
}

# ============ 环境准备 ============

prepare_environment() {
    log_info "准备测试环境..."

    # 创建 .env.test 文件（如果不存在）
    if [ ! -f "$ENV_FILE" ]; then
        cat > "$ENV_FILE" << 'EOF'
# Inklog 测试环境配置
# 这些环境变量用于集成测试

# PostgreSQL 配置
POSTGRES_USER=inklog_test
POSTGRES_PASSWORD=inklog_test_pass
POSTGRES_DB=inklog_logs
DATABASE_URL=postgres://inklog_test:inklog_test_pass@localhost:5432/inklog_logs

# MySQL 配置
MYSQL_ROOT_PASSWORD=inklog_root_pass
MYSQL_USER=inklog_test
MYSQL_PASSWORD=inklog_test_pass
MYSQL_DATABASE=inklog_logs
MYSQL_URL=mysql://inklog_test:inklog_test_pass@localhost:3306/inklog_logs

# Redis 配置
REDIS_URL=redis://localhost:6379/0

# LocalStack S3 配置
AWS_ACCESS_KEY_ID=test
AWS_SECRET_ACCESS_KEY=test
AWS_DEFAULT_REGION=us-east-1
S3_ENDPOINT_URL=http://localhost:4566
S3_BUCKET=inklog-archives
S3_REGION=us-east-1

# MinIO 配置
MINIO_ENDPOINT=http://localhost:9000
MINIO_ACCESS_KEY=minioadmin
MINIO_SECRET_KEY=minioadmin123
MINIO_BUCKET=inklog-archives

# 测试配置
RUST_LOG=debug
TEST_MODE=integration
EOF
        log_success "创建环境配置文件: $ENV_FILE"
    fi
}

# ============ 服务管理函数 ============

start_services() {
    log_info "启动测试服务..."

    check_docker
    check_ports
    prepare_environment

    # 使用 docker compose 或 docker-compose
    if docker compose version &> /dev/null; then
        COMPOSE_CMD="docker compose"
    else
        COMPOSE_CMD="docker-compose"
    fi

    # 启动服务（后台运行）
    $COMPOSE_CMD -f "$COMPOSE_FILE" -p "$COMPOSE_PROJECT_NAME" up -d

    log_info "等待服务启动..."
    wait_for_services

    log_success "所有测试服务已启动"
    show_service_status
}

wait_for_services() {
    local elapsed=0
    local interval=2

    while [ $elapsed -lt $WAIT_TIMEOUT ]; do
        local all_ready=true

        # 检查 PostgreSQL
        if ! docker exec inklog-test-postgres pg_isready -U inklog_test &> /dev/null; then
            all_ready=false
        fi

        # 检查 Redis
        if ! docker exec inklog-test-redis redis-cli ping &> /dev/null; then
            all_ready=false
        fi

        # 检查 LocalStack
        if ! curl -s http://localhost:4566/_localstack/health | grep -q '"s3": "available"'; then
            all_ready=false
        fi

        if $all_ready; then
            return 0
        fi

        echo -n "."
        sleep $interval
        elapsed=$((elapsed + interval))
    done

    log_warning "等待服务超时，部分服务可能未就绪"
}

stop_services() {
    log_info "停止测试服务并清理..."

    check_docker

    if docker compose version &> /dev/null; then
        COMPOSE_CMD="docker compose"
    else
        COMPOSE_CMD="docker-compose"
    fi

    # 停止并移除容器
    $COMPOSE_CMD -f "$COMPOSE_FILE" -p "$COMPOSE_PROJECT_NAME" down -v --remove-orphans

    log_success "测试服务已停止并清理"
}

show_service_status() {
    echo ""
    echo "=========================================="
    echo "         测试服务状态"
    echo "=========================================="
    echo ""

    if docker compose version &> /dev/null; then
        COMPOSE_CMD="docker compose"
    else
        COMPOSE_CMD="docker-compose"
    fi

    $COMPOSE_CMD -f "$COMPOSE_FILE" -p "$COMPOSE_PROJECT_NAME" ps

    echo ""
    echo "服务连接信息:"
    echo "  PostgreSQL: localhost:5432 (用户: inklog_test)"
    echo "  MySQL:      localhost:3306 (用户: inklog_test)"
    echo "  Redis:      localhost:6379"
    echo "  LocalStack: localhost:4566 (S3 端点)"
    echo "  MinIO:      localhost:9000 (S3 API), localhost:9001 (Console)"
    echo ""
}

show_logs() {
    local service="${1:-}"

    if docker compose version &> /dev/null; then
        COMPOSE_CMD="docker compose"
    else
        COMPOSE_CMD="docker-compose"
    fi

    if [ -z "$service" ]; then
        $COMPOSE_CMD -f "$COMPOSE_FILE" -p "$COMPOSE_PROJECT_NAME" logs --tail=50
    else
        $COMPOSE_CMD -f "$COMPOSE_FILE" -p "$COMPOSE_PROJECT_NAME" logs --tail=50 "$service"
    fi
}

# ============ 测试函数 ============

run_tests() {
    log_info "运行集成测试..."

    # 检查服务是否运行
    if ! docker ps --format '{{.Names}}' | grep -q "inklog-test"; then
        log_warning "测试服务未运行，正在启动..."
        start_services
    fi

    # 导出环境变量供测试使用
    export_env_vars

    cd "$PROJECT_ROOT"

    # 运行集成测试
    log_info "执行: cargo test --features $TEST_FEATURES --test integration_tests"
    cargo test --features "$TEST_FEATURES" --test integration_tests "$@"

    local exit_code=$?

    if [ $exit_code -eq 0 ]; then
        log_success "集成测试全部通过"
    else
        log_error "集成测试失败 (退出码: $exit_code)"
    fi

    return $exit_code
}

run_all_tests() {
    log_info "运行所有测试（包括需要外部服务的测试）..."

    # 检查服务是否运行
    if ! docker ps --format '{{.Names}}' | grep -q "inklog-test"; then
        log_warning "测试服务未运行，正在启动..."
        start_services
    fi

    export_env_vars

    cd "$PROJECT_ROOT"

    # 运行所有测试
    log_info "执行: cargo test --features $TEST_FEATURES"
    cargo test --features "$TEST_FEATURES" "$@"

    local exit_code=$?

    if [ $exit_code -eq 0 ]; then
        log_success "所有测试全部通过"
    else
        log_error "测试失败 (退出码: $exit_code)"
    fi

    return $exit_code
}

export_env_vars() {
    # 从 .env.test 导出环境变量
    if [ -f "$ENV_FILE" ]; then
        set -a
        source "$ENV_FILE"
        set +a
    fi

    # 设置 Rust 日志级别
    export RUST_LOG="${RUST_LOG:-debug}"
    export TEST_MODE=integration
}

# ============ 清理函数 ============

cleanup_all() {
    log_info "执行完整清理..."

    # 停止 Docker 服务
    stop_services

    # 清理测试产生的临时文件
    log_info "清理临时文件..."
    rm -rf "$PROJECT_ROOT/target/debug/.fingerprint/inklog-*"
    rm -rf "$PROJECT_ROOT/tests/temp_*"
    rm -f "$PROJECT_ROOT/test.log"

    # 清理 Rust 测试缓存
    log_info "清理 Rust 测试缓存..."
    cargo clean -p inklog --tests &> /dev/null || true

    log_success "清理完成"
}

# ============ 帮助信息 ============

show_help() {
    cat << EOF
Inklog 集成测试 Docker 环境管理脚本

用法: ./scripts/test-docker.sh <命令> [选项]

命令:
    start           启动测试环境中的所有服务
    stop            停止所有测试服务并清理容器
    restart         重启测试服务
    test            运行集成测试（自动启动环境）
    all-tests       运行所有测试
    status          显示服务状态
    logs [服务]     查看服务日志（可选指定服务名）
    cleanup         完整清理（停止服务 + 清理临时文件）
    help            显示帮助信息

选项:
    --features      指定测试特性 (默认: $TEST_FEATURES)
    --timeout       设置服务启动超时 (默认: $WAIT_TIMEOUT 秒)

示例:
    ./scripts/test-docker.sh start
    ./scripts/test-docker.sh test --features "dbnexus,aws"
    ./scripts/test-docker.sh logs postgres
    ./scripts/test-docker.sh stop

环境变量:
    可在 .env.test 文件中配置数据库连接等信息

EOF
}

# ============ 主函数 ============

main() {
    local command="${1:-}"

    # 解析全局选项
    while [[ "$1" == --* ]]; do
        case "$1" in
            --features)
                TEST_FEATURES="$2"
                shift 2
                ;;
            --timeout)
                WAIT_TIMEOUT="$2"
                shift 2
                ;;
            *)
                log_error "未知选项: $1"
                show_help
                exit 1
                ;;
        esac
    done

    case "$command" in
        start)
            start_services
            ;;
        stop)
            stop_services
            ;;
        restart)
            stop_services
            sleep 2
            start_services
            ;;
        test)
            shift
            run_tests "$@"
            ;;
        all-tests|all)
            shift
            run_all_tests "$@"
            ;;
        status)
            show_service_status
            ;;
        logs)
            shift
            show_logs "$1"
            ;;
        cleanup)
            cleanup_all
            ;;
        help|--help|-h)
            show_help
            ;;
        "")
            show_help
            ;;
        *)
            log_error "未知命令: $command"
            show_help
            exit 1
            ;;
    esac
}

main "$@"
