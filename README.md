# MWG Monitoring Tool (Rust TUI)

MWG 프록시 서버 모니터링 도구의 Rust + Ratatui 버전입니다.

## 기능

- **자원 사용률 모니터링**: SNMP를 통한 CPU, 메모리 등 시스템 자원 모니터링
  - 실시간 자원 사용률 수집 및 표시
  - 임계치 기반 색상 표시 (정상/경고/위험)
  - 자동 수집 기능 (주기 설정 가능)
  - 그룹별 필터링 지원
  - 인터페이스(회선) 트래픽 모니터링
- **세션 브라우저**: SSH를 통한 활성 세션 조회
  - MWG 명령어 기반 세션 조회 (`/opt/mwg/bin/mwg-core -S connections`)
  - 19개 필드 파싱 및 표시 (트랜잭션, 생성시간, 프로토콜, 클라이언트IP, 서버IP, URL 등)
  - 가로 스크롤 기능 (좌우 화살표키)
  - 그룹별 필터링 지원
  - CSV 저장 기능
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

`config/resource_config.json` 파일에 SNMP OID 및 임계치 설정:

```json
{
  "snmp_version": "2c",
  "community": "public",
  "oids": {
    "cpu": "1.3.6.1.4.1.2021.11.11.0",
    "mem": "ssh",
    "cc": "1.3.6.1.4.1.2021.4.11.0",
    "cs": "",
    "http": "",
    "https": "",
    "ftp": ""
  },
  "thresholds": {
    "cpu": {
      "warning": 70.0,
      "critical": 90.0
    },
    "mem": {
      "warning": 70.0,
      "critical": 90.0
    }
  }
}
```

자세한 설정 방법은 [MONITORING_GUIDE.md](./MONITORING_GUIDE.md)를 참고하세요.

## 사용법

### 키보드 단축키

#### 공통 단축키
- `Tab` / `Shift+Tab`: 탭 전환 (다음/이전)
- `1`, `2`, `3`, `4`: 탭 직접 선택
- `↑` / `↓` 또는 `k` / `j`: 테이블에서 위/아래 이동
- `←` / `→` 또는 `h` / `l`: 탭 전환
- `q` / `Esc`: 종료

#### 자원사용률 탭
- `C`: 수동 수집 시작
- `Space`: 자동 수집 시작/중지 토글
- `+` / `-`: 수집 주기 증가/감소
- `Shift+←` / `Shift+→`: 그룹 선택 (전체보기 포함)

#### 세션브라우저 탭
- `S`: 세션 조회 시작
- `←` / `→`: 컬럼 스크롤 (가로 스크롤)
- `Shift+←` / `Shift+→`: 그룹 선택
- `↑` / `↓`: 행 이동

### 탭 설명

1. **프록시관리**: 프록시 서버 목록 및 그룹 관리
2. **자원사용률**: 프록시 서버의 CPU, 메모리, 연결 수, 트래픽 등 모니터링
   - 임계치 기반 색상 표시 (하얀색: 정상, 노란색: 경고, 빨간색: 위험)
   - 자동 수집 기능
   - 그룹별 필터링
   - 인터페이스(회선) 트래픽 모니터링
3. **세션브라우저**: 활성 세션 조회 및 필터링
   - SSH를 통한 실시간 세션 조회
   - 19개 필드 표시 (호스트, 트랜잭션, 생성시간, 프로토콜, CustID, 사용자, 클라이언트IP, 서버IP, URL 등)
   - 가로 스크롤로 모든 컬럼 확인 가능
   - 그룹별 필터링
   - 조회 결과 CSV 저장
4. **트래픽로그**: 트래픽 로그 분석 및 통계

## 로그 파일

모든 수집/분석 결과는 `logs/` 디렉토리에 CSV 파일로 저장됩니다:

- `logs/resource_usage_YYYYMMDD_HHMMSS.csv`: 자원 사용률 수집 결과
- `logs/sessions_YYYYMMDD_HHMMSS.csv`: 세션 조회 결과
- `logs/traffic_analysis_YYYYMMDD_HHMMSS.csv`: 트래픽 로그 분석 결과

## 프로젝트 구조

```
src/
├── main.rs                    # 진입점
├── app/                       # 앱 상태 관리 모듈
│   ├── mod.rs
│   ├── app.rs                 # App 구조체 및 메인 로직
│   ├── states.rs              # 각 탭 상태 구조체
│   ├── types.rs               # 공통 타입 정의
│   └── config.rs              # 설정 파일 로드 헬퍼
├── ui/                        # UI 렌더링 모듈
│   ├── mod.rs
│   ├── proxy_management.rs   # 프록시 관리 탭 UI
│   ├── resource_usage.rs     # 자원 사용률 탭 UI
│   ├── session_browser.rs     # 세션 브라우저 탭 UI
│   ├── traffic_logs.rs        # 트래픽 로그 탭 UI
│   └── config.rs              # UI 설정 헬퍼
├── crossterm.rs               # 터미널 제어 및 이벤트 처리
├── snmp.rs                    # SNMP 클라이언트
├── ssh.rs                     # SSH 클라이언트
├── collector.rs               # 자원 수집기
├── session_collector.rs       # 세션 조회기
└── csv_writer.rs              # CSV 저장 기능

config/                        # 설정 파일
├── proxies.json
└── resource_config.json

logs/                          # 결과 파일 (CSV)
```

## 개발 상태

**현재 진행률: 75%** (Phase 1, Phase 2, Phase 3 완료)

### ✅ Phase 1: 기본 구조 및 탭 UI (완료)
- ✅ 기본 TUI 구조
- ✅ 탭 기반 UI (4개 탭)
- ✅ 설정 파일 읽기
- ✅ 기본 레이아웃
- ✅ 프록시 관리 탭

### ✅ Phase 2: 자원 사용률 모니터링 (완료)
- ✅ SNMP 클라이언트 구현 (SNMPv2c)
- ✅ SSH 클라이언트 구현
- ✅ 자원 수집 기능 (CPU, MEM, CC, CS, HTTP, HTTPS, FTP)
- ✅ 인터페이스 트래픽 수집
- ✅ CSV 저장 기능
- ✅ 자동 수집 기능
- ✅ 임계치 기반 색상 표시
- ✅ 그룹별 필터링

### ✅ Phase 3: 세션 브라우저 (완료)
- ✅ SSH를 통한 세션 조회
- ✅ MWG 명령어 실행 (`/opt/mwg/bin/mwg-core -S connections`)
- ✅ 파이프 구분 형식 파싱 (19개 필드 모두 파싱)
- ✅ Table 위젯에 세션 목록 표시
- ✅ 가로 스크롤 기능 (좌우 화살표키)
- ✅ 로딩 인디케이터 (스피너 애니메이션)
- ✅ 그룹별 필터링
- ✅ CSV 저장 (모든 필드 포함)

### 📋 Phase 4: 트래픽 로그 분석 (대기 중)

자세한 진행 상황은 [PROGRESS.md](./PROGRESS.md)를 참고하세요.

