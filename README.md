# MWG Monitoring Tool (Rust TUI)

MWG 프록시 서버 모니터링 도구의 Rust + Ratatui 버전입니다.

## 기능

- **자원 사용률 모니터링**: SNMP를 통한 CPU, 메모리 등 시스템 자원 모니터링
- **세션 브라우저**: SSH를 통한 활성 세션 조회
- **트래픽 로그 분석**: 프록시 로그 분석 및 통계

## 설치 및 실행

### 요구사항

- Rust 1.70 이상
- Cargo

### 빌드

```bash
cargo build --release
```

### 실행

```bash
cargo run
```

또는 릴리즈 빌드:

```bash
./target/release/rust-mmt
```

## 설정

### 프록시 설정

`config/proxies.json` 파일에 프록시 정보를 설정합니다:

```json
{
  "proxies": [
    {
      "id": 1,
      "host": "192.168.1.10",
      "port": 22,
      "username": "admin",
      "password": "password123",
      "group": "프로덕션",
      "traffic_log_path": "/var/log/proxy.log",
      "snmp_community": "public"
    }
  ]
}
```

### SNMP 설정

`config/resource_config.json` 파일에 SNMP OID 설정:

```json
{
  "community": "public",
  "oids": {
    "cpu": "1.3.6.1.4.1.2021.11.11.0",
    "mem": "ssh",
    "cc": "1.3.6.1.4.1.2021.4.11.0"
  }
}
```

## 사용법

### 키보드 단축키

- `Tab` / `Shift+Tab`: 탭 전환 (다음/이전)
- `1`, `2`, `3`: 탭 직접 선택
- `↑` / `↓` 또는 `k` / `j`: 테이블에서 위/아래 이동
- `←` / `→` 또는 `h` / `l`: 탭 전환
- `q` / `Esc`: 종료

### 탭 설명

1. **자원사용률**: 프록시 서버의 CPU, 메모리 사용률 모니터링
2. **세션브라우저**: 활성 세션 조회 및 필터링
3. **트래픽로그**: 트래픽 로그 분석 및 통계

## 로그 파일

모든 수집/분석 결과는 `logs/` 디렉토리에 CSV 파일로 저장됩니다:

- `logs/resource_usage_YYYYMMDD_HHMMSS.csv`: 자원 사용률 수집 결과
- `logs/sessions_YYYYMMDD_HHMMSS.csv`: 세션 조회 결과
- `logs/traffic_analysis_YYYYMMDD_HHMMSS.csv`: 트래픽 로그 분석 결과

## 프로젝트 구조

```
src/
├── main.rs          # 진입점
├── app.rs           # 앱 상태 관리
├── ui.rs            # UI 렌더링 로직
└── crossterm.rs     # 터미널 설정 및 이벤트 처리

config/              # 설정 파일
├── proxies.json
└── resource_config.json

logs/                # 결과 파일 (CSV)
```

## 개발 상태

**현재 진행률: 25%** (Phase 1 완료)

### ✅ Phase 1: 기본 구조 및 탭 UI (완료)
- ✅ 기본 TUI 구조
- ✅ 탭 기반 UI
- ✅ 설정 파일 읽기
- ✅ 기본 레이아웃

### 🔄 Phase 2: 자원 사용률 모니터링 (진행 예정)
- SNMP 클라이언트 구현
- 자원 수집 기능
- CSV 저장 기능

### 📋 Phase 3: 세션 브라우저 (대기 중)
### 📋 Phase 4: 트래픽 로그 분석 (대기 중)

자세한 진행 상황은 [PROGRESS.md](./PROGRESS.md)를 참고하세요.

