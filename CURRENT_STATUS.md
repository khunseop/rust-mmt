# 현재 상태 문서

## 프로젝트 개요

MWG 프록시 서버 모니터링 도구의 Rust + Ratatui 버전입니다.

## 전체 진행률

**75% 완료** (Phase 1, 2, 3 완료)

---

## 모듈 구조

### 코드 모듈화 완료

프로젝트가 모듈화되어 다음과 같은 구조를 가집니다:

```
src/
├── main.rs                    # 진입점
├── app/                       # 앱 상태 관리 모듈
│   ├── mod.rs                 # 모듈 진입점
│   ├── app.rs                 # App 구조체 및 메인 로직
│   ├── states.rs              # 각 탭 상태 구조체
│   ├── types.rs               # 공통 타입 정의 (Proxy, SessionData, ResourceData 등)
│   └── config.rs              # 설정 파일 로드 헬퍼
├── ui/                        # UI 렌더링 모듈
│   ├── mod.rs                 # UI 모듈 진입점
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
```

---

## 완료된 기능

### ✅ Phase 1: 기본 구조 및 탭 UI (100%)

- 탭 기반 UI (4개 탭)
- 키보드 네비게이션
- 설정 파일 읽기
- 프록시 관리 탭

### ✅ Phase 2: 자원 사용률 모니터링 (100%)

- SNMP 클라이언트 (BER 인코딩/디코딩 직접 구현)
- SSH 클라이언트 (ssh2 크레이트)
- 자원 수집 (CPU, MEM, CC, CS, HTTP, HTTPS, FTP)
- 인터페이스 트래픽 수집
- 임계치 기반 색상 표시
- CSV 저장
- 자동 수집 기능

### ✅ Phase 3: 세션 브라우저 (100%)

- SSH를 통한 세션 조회
- MWG 명령어 실행 (`/opt/mwg/bin/mwg-core -S connections`)
- 파이프 구분 형식 파싱 (기존 Python 앱 로직 포팅)
- 19개 필드 모두 파싱 및 표시:
  - 호스트, 트랜잭션, 생성시간, 프로토콜, CustID, 사용자
  - 클라이언트IP, CL-MWG-IP, SRV-MWG-IP, 서버IP
  - CL수신, CL송신, SRV수신, SRV송신
  - TrxnIdx, Age(초), 상태, InUse, URL
- 가로 스크롤 기능 (좌우 화살표키)
- 로딩 인디케이터 (스피너 애니메이션)
- 그룹별 필터링
- CSV 저장 (모든 필드 포함)

---

## 현재 상태

### 컴파일 상태

✅ **컴파일 성공** (경고만 있음, 에러 없음)

```
warning: field `filter` is never read
warning: field `selected_proxy` is never read
warning: field `max_workers` is never read
```

이러한 경고는 사용되지 않는 필드에 대한 것으로, 기능에는 영향 없음.

### 주요 기능 동작 상태

1. **자원 사용률 모니터링**: ✅ 작동
   - SNMP 수집 정상 작동
   - SSH 메모리 수집 정상 작동
   - 인터페이스 트래픽 수집 정상 작동
   - 임계치 색상 표시 정상 작동

2. **세션 브라우저**: ✅ 작동
   - SSH 세션 조회 정상 작동
   - 파싱 정상 작동 (19개 필드 모두 파싱)
   - UI 표시 정상 작동
   - 가로 스크롤 정상 작동
   - CSV 저장 정상 작동

3. **프록시 관리**: ✅ 작동
   - 프록시 목록 표시 정상 작동
   - 그룹별 표시 정상 작동

---

## 알려진 이슈 및 개선 사항

### 해결된 이슈

1. ✅ 컬럼 스크롤이 탭 전환으로 동작하던 문제 → 수정 완료
2. ✅ 생성시간이 NA로 표시되던 문제 → 파싱 로직 개선 완료
3. ✅ 상하 이동이 느리던 문제 → tick_rate 50ms로 변경 완료
4. ✅ 로딩 인디케이터가 안움직이던 문제 → 스피너 애니메이션 개선 완료
5. ✅ 모든 컬럼이 표시 안되던 문제 → 19개 컬럼 모두 표시, 가로 스크롤 완료

### 향후 개선 사항

1. **세션 브라우저 기능 개선**
   - [ ] 검색 기능 (텍스트 검색)
   - [ ] 필터링 기능 (컬럼별 필터)
   - [ ] 정렬 기능 (컬럼별 정렬)
   - [ ] 페이지네이션 (대용량 데이터 처리)

2. **트래픽 로그 분석 기능** (Phase 4)
   - [ ] SSH를 통한 원격 로그 조회
   - [ ] 로그 파일 업로드 처리
   - [ ] 로그 파싱 및 분석
   - [ ] TOP N 분석
   - [ ] BarChart 시각화

---

## 기술 스택

### 사용 중인 크레이트

- `ratatui`: TUI 프레임워크
- `crossterm`: 터미널 제어
- `serde` + `serde_json`: JSON 직렬화
- `tokio`: 비동기 런타임
- `anyhow`: 에러 처리
- `chrono`: 시간 처리
- `csv`: CSV 파일 읽기/쓰기
- `regex`: 정규식 패턴 매칭
- `ssh2`: SSH 클라이언트
- `snmp`: SNMP 클라이언트 (직접 구현)

---

## 파일 구조

```
rust-mmt/
├── src/
│   ├── main.rs                    # 진입점
│   ├── app/                       # 앱 상태 관리 모듈
│   │   ├── mod.rs
│   │   ├── app.rs
│   │   ├── states.rs
│   │   ├── types.rs
│   │   └── config.rs
│   ├── ui/                        # UI 렌더링 모듈
│   │   ├── mod.rs
│   │   ├── proxy_management.rs
│   │   ├── resource_usage.rs
│   │   ├── session_browser.rs
│   │   ├── traffic_logs.rs
│   │   └── config.rs
│   ├── crossterm.rs               # 터미널 제어
│   ├── snmp.rs                    # SNMP 클라이언트
│   ├── ssh.rs                     # SSH 클라이언트
│   ├── collector.rs               # 자원 수집기
│   ├── session_collector.rs       # 세션 조회기
│   └── csv_writer.rs              # CSV 저장
├── config/
│   ├── proxies.json               # 프록시 설정
│   └── resource_config.json       # SNMP 설정
├── logs/                          # CSV 파일 저장 디렉토리
├── Cargo.toml
├── README.md
├── ANALYSIS.md
├── MONITORING_GUIDE.md
├── PROGRESS.md
└── CURRENT_STATUS.md              # 이 문서
```

---

## 실행 방법

```bash
# 개발 모드 실행
cargo run

# 릴리즈 빌드
cargo build --release

# 릴리즈 실행
./target/release/rust-mmt
```

---

## 키보드 단축키

### 공통
- `Tab`: 탭 전환
- `1`, `2`, `3`, `4`: 탭 직접 선택
- `q` / `Esc`: 종료

### 자원 사용률 탭
- `C`: 수동 수집 시작
- `Space`: 자동 수집 토글
- `+` / `-`: 수집 주기 증가/감소
- `Shift+←` / `Shift+→`: 그룹 선택
- `↑` / `↓`: 행 이동

### 세션 브라우저 탭
- `S`: 세션 조회 시작
- `←` / `→`: 컬럼 스크롤 (가로 스크롤)
- `Shift+←` / `Shift+→`: 그룹 선택
- `↑` / `↓`: 행 이동

---

## 업데이트 날짜

2024-01-XX: Phase 3 완료 및 코드 모듈화 완료
