# MWG Monitoring Tool → Rust + Ratatui 변환 분석 및 단순화 방안

## 현재 앱 구조 분석

### 주요 기능

1. **프록시 관리 (Proxy Management)**
   - 프록시 그룹 생성/관리
   - 프록시 서버 등록 (호스트, 포트, SSH 인증 정보)
   - 프록시 비밀번호 암호화 저장
   - 프록시 활성화/비활성화

2. **자원 사용률 모니터링 (Resource Usage)**
   - SNMP를 통한 CPU, 메모리, 네트워크 인터페이스 수집
   - SSH를 통한 메모리 수집 (선택적)
   - 실시간 수집 및 이력 저장 (90일 보관)
   - 백그라운드 자동 수집 (선택적)
   - 웹소켓을 통한 실시간 업데이트

3. **세션 브라우저 (Session Browser)**
   - SSH를 통해 프록시 서버의 활성 세션 조회
   - 세션 정보 파싱 (트랜잭션, 클라이언트 IP, URL 등)
   - 임시 파일 저장소 사용 (JSON Lines)
   - 필터링 및 정렬 기능

4. **트래픽 로그 분석 (Traffic Logs)**
   - SSH를 통한 원격 로그 조회
   - 로그 파일 업로드 및 분석
   - TOP N 분석 (클라이언트, 호스트, URL별 통계)
   - 파싱된 로그 저장

### 데이터 저장 구조

**데이터베이스 (SQLite):**
- `proxies`: 프록시 서버 정보
- `proxy_groups`: 프록시 그룹
- `resource_usage`: 자원 사용률 이력
- `resource_config`: SNMP OID 설정
- `traffic_logs`: 파싱된 트래픽 로그
- `session_browser_config`: 세션 브라우저 설정

**임시 파일 저장소:**
- 세션 브라우저 데이터: JSON Lines 형식 (`/tmp/session_browser/proxy_{id}/batch_{timestamp}.jsonl`)

---

## 단순화 방안

### 1. 데이터 저장 방식 단순화

#### 현재: SQLite 데이터베이스
#### 제안: JSON 파일 기반 저장

```
config/
├── proxies.json          # 프록시 설정 (읽기 전용)
└── resource_config.json  # SNMP 설정

logs/
├── resource_usage_20240101_142315.csv  # 자원 사용률 수집 결과
├── resource_usage_20240101_143000.csv
├── sessions_20240101_142500.csv         # 세션 조회 결과
├── sessions_20240101_143200.csv
├── traffic_analysis_20240101_144000.csv  # 트래픽 로그 분석 결과
└── traffic_analysis_20240101_145000.csv
```

**장점:**
- 데이터베이스 설정 불필요
- 파일 기반으로 간단한 백업/복원
- 디버깅 용이 (텍스트 에디터로 확인 가능)
- SQLite 의존성 제거

**단점:**
- 대용량 데이터 처리 시 성능 저하 가능
- 동시성 제어 필요 (파일 락 사용)

### 2. 기능 단순화

#### A. 프록시 관리
- ❌ **제거: 프록시 CRUD 기능 (추가/수정/삭제)**
- ✅ **대안: 프록시 정보는 하드코딩 또는 JSON 설정 파일로 관리**
  - `config/proxies.json` 파일에 프록시 정보 저장
  - 앱 실행 시 설정 파일에서 읽어옴
  - 수정이 필요하면 설정 파일을 직접 편집
- ⚠️ 단순화: 비밀번호 암호화 → 평문 저장 (또는 간단한 base64 인코딩)
- ❌ 제거: 프록시 활성화/비활성화 (필요시 설정 파일에서 제거)

#### B. 자원 사용률 모니터링
- ✅ 유지: SNMP를 통한 수집
- ✅ 유지: 실시간 수집 (수동 실행)
- ✅ **변경: 수집 결과는 CSV 파일로 저장**
  - `logs/resource_usage_YYYYMMDD_HHMMSS.csv` 형식
  - 각 수집마다 새 파일 생성 또는 타임스탬프 추가
- ❌ 제거: 백그라운드 자동 수집
- ❌ 제거: 웹소켓 실시간 업데이트
- ⚠️ 단순화: 인터페이스 MBPS → 기본 인터페이스만 수집

#### C. 세션 브라우저
- ✅ 유지: SSH를 통한 세션 조회
- ✅ 유지: 세션 정보 파싱
- ✅ **변경: 조회 결과는 CSV 파일로 저장**
  - `logs/sessions_YYYYMMDD_HHMMSS.csv` 형식
  - 각 조회마다 새 파일 생성
- ✅ 유지: 필터링 (간단한 텍스트 검색)

#### D. 트래픽 로그 분석
- ✅ 유지: SSH를 통한 원격 로그 조회
- ✅ 유지: 파일 업로드 분석
- ✅ 유지: TOP N 분석
- ✅ **변경: 분석 결과는 CSV 파일로 저장**
  - `logs/traffic_analysis_YYYYMMDD_HHMMSS.csv` 형식
  - 파싱된 로그와 TOP N 분석 결과 모두 저장

### 3. UI 구조 (Ratatui)

#### 탭 기반 UI 구조 (Crossterm Demo 스타일)

**메인 화면 - 탭 네비게이션:**
```
┌─────────────────────────────────────────┐
│  MWG Monitoring Tool                    │
├─────────────────────────────────────────┤
│  [자원사용률] [세션브라우저] [트래픽로그] │  ← 탭 (Tab)
├─────────────────────────────────────────┤
│                                         │
│  [각 탭의 콘텐츠 영역]                   │
│                                         │
└─────────────────────────────────────────┘
```

**탭 전환:**
- `Tab` / `Shift+Tab`: 다음/이전 탭으로 이동
- `1`, `2`, `3`: 숫자 키로 직접 탭 선택
- 각 탭은 독립적인 화면과 상태를 가짐

#### 각 탭 화면 구조

**1. 자원 사용률 모니터링 탭:**
```
┌─────────────────────────────────────────┐
│  [자원사용률] [세션브라우저] [트래픽로그] │  ← 탭 헤더 (Tabs 위젯)
├─────────────────────────────────────────┤
│  자원 사용률 모니터링          [수집] [CSV] │
├─────────────────────────────────────────┤
│  프록시: [전체 ▼]                        │
├─────────────────────────────────────────┤
│  ┌────────────────────────────────────┐ │
│  │ 호스트      │ CPU  │ MEM  │ 시간   │ │  ← Table 위젯
│  ├────────────────────────────────────┤ │
│  │ 192.168.1.10│45.2% │67.3% │14:23:15│ │
│  │ 192.168.1.11│32.1% │54.2% │14:23:15│ │
│  └────────────────────────────────────┘ │
│                                         │
│  ↑/↓: 이동, Enter: 상세보기              │
└─────────────────────────────────────────┘
```

**Table 위젯 사용:**
- [Ratatui Table 예제](https://ratatui.rs/examples/widgets/table/) 참고
- `Table::new()`로 테이블 생성
- `Constraint`로 컬럼 너비 조정
- `Row`와 `Cell`로 데이터 구성
- 선택 가능한 행 (highlight) 지원

**2. 세션 브라우저 탭:**
```
┌─────────────────────────────────────────┐
│  [자원사용률] [세션브라우저] [트래픽로그] │  ← 탭 헤더 (Tabs 위젯)
├─────────────────────────────────────────┤
│  세션 브라우저              [조회] [CSV] │
├─────────────────────────────────────────┤
│  프록시: [전체 ▼]                        │
├─────────────────────────────────────────┤
│  ┌────────────────────────────────────┐ │
│  │ 호스트    │ 클라이언트IP │ URL    │ │  ← Table 위젯
│  ├────────────────────────────────────┤ │
│  │192.168.1.10│ 1.2.3.4    │http://..│ │
│  │192.168.1.10│ 1.2.3.5    │https://.│ │
│  └────────────────────────────────────┘ │
│  ↑/↓: 이동, Enter: 상세보기              │
└─────────────────────────────────────────┘
```

**Table 위젯 기능:**
- 스크롤 가능한 테이블 (ListState 활용)
- 행 선택 및 하이라이트
- 컬럼별 정렬 (선택적)

**3. 트래픽 로그 분석 탭:**
```
┌─────────────────────────────────────────┐
│  [자원사용률] [세션브라우저] [트래픽로그] │  ← 탭 헤더 (Tabs 위젯)
├─────────────────────────────────────────┤
│  트래픽 로그 분석    [원격조회] [업로드] [CSV] │
├─────────────────────────────────────────┤
│  프록시: [선택 ▼]                        │
├─────────────────────────────────────────┤
│  [요약 정보]                             │
│  총 요청: 1,234                          │
├─────────────────────────────────────────┤
│  ┌────────────────────────────────────┐ │
│  │ 순위│ 클라이언트IP │ 요청수 │ 바이트 │ │  ← Table 위젯
│  ├────────────────────────────────────┤ │
│  │  1  │ 1.2.3.4     │ 123   │ 1.2MB │ │
│  │  2  │ 1.2.3.5     │  89   │ 0.8MB │ │
│  └────────────────────────────────────┘ │
├─────────────────────────────────────────┤
│  [Bar Chart - TOP 호스트]               │
│  example.com  ██████████ 456건         │
│  example.org  ██████   234건           │
└─────────────────────────────────────────┘
```

**Table + Chart 조합:**
- Table 위젯으로 TOP N 리스트 표시
- BarChart 위젯으로 시각화
- 분석 결과를 구조화된 테이블로 표시

**탭 UI 특징:**
- 상단에 탭 헤더 표시 (현재 활성 탭 하이라이트)
- 각 탭은 독립적인 위젯과 상태 관리
- Ratatui의 `Tabs` 위젯 활용
- 키보드로 탭 전환 가능 (`Tab`, `Shift+Tab`, 숫자 키)

**참고:**
- [Ratatui Demo 예제](https://ratatui.rs/examples/apps/demo/) 구조 참고
  - `app.rs`: 앱 상태 관리
  - `ui.rs`: UI 렌더링 로직
  - `crossterm.rs`: 이벤트 처리 및 터미널 설정

### 4. 기술 스택 제안

#### Rust 크레이트

**핵심:**
- `ratatui`: TUI 프레임워크
  - `Tabs` 위젯: 탭 네비게이션
  - `Table` 위젯: 데이터 테이블 표시 ([예제](https://ratatui.rs/examples/widgets/table/))
  - `BarChart`: 차트 표시 (트래픽 로그 분석용)
  - `List`: 리스트 표시
- `crossterm`: 터미널 제어 (Ratatui 데모 앱과 동일)
- `tokio`: 비동기 런타임 (SSH, SNMP 작업용)
- `serde` + `serde_json`: JSON 직렬화/역직렬화

**네트워크:**
- `ssh2` 또는 `russh`: SSH 클라이언트
- `snmp` 또는 `snmp-parser`: SNMP 클라이언트 (또는 직접 UDP 구현)

**파일 처리:**
- `std::fs`: 파일 I/O
- `csv`: CSV 파일 읽기/쓰기
- `flock` 또는 `fs2`: 파일 락 (동시성 제어, 선택적)

**기타:**
- `chrono`: 시간 처리
- `anyhow`: 에러 처리
- `argh`: CLI 인자 파싱 (Ratatui 데모 앱과 동일)

**참고 자료:**
- [Ratatui Demo 예제](https://ratatui.rs/examples/apps/demo/): 앱 구조 및 이벤트 처리
- [Ratatui Table 예제](https://ratatui.rs/examples/widgets/table/): 테이블 위젯 사용법

### 4-1. 앱 구조 (Ratatui Demo 참고)

**파일 구조:**
```
src/
├── main.rs          # 진입점, CLI 인자 파싱
├── app.rs           # 앱 상태 관리
│   - App 구조체: 탭 인덱스, 각 탭의 상태
│   - 이벤트 핸들러: on_left(), on_right(), on_tick() 등
├── ui.rs            # UI 렌더링 로직
│   - draw() 함수: 전체 UI 렌더링
│   - 각 탭별 렌더링 함수
│   - Table, Tabs, BarChart 위젯 사용
└── crossterm.rs     # 터미널 설정 및 이벤트 루프
    - run() 함수: 터미널 초기화
    - 이벤트 폴링 및 처리
    - 터미널 복원
```

**앱 상태 구조 예시:**
```rust
pub struct App {
    pub title: String,
    pub tabs: TabsState,           // 탭 상태
    pub resource_usage: ResourceUsageState,
    pub session_browser: SessionBrowserState,
    pub traffic_logs: TrafficLogsState,
    pub should_quit: bool,
}

pub struct ResourceUsageState {
    pub proxies: Vec<Proxy>,
    pub selected_proxy: Option<usize>,
    pub table_state: TableState,   // Table 위젯 상태
    pub data: Vec<ResourceData>,
}

pub struct SessionBrowserState {
    pub table_state: TableState,
    pub sessions: Vec<Session>,
    pub filter: String,
}
```

**이벤트 처리:**
- `Tab` / `Shift+Tab`: 탭 전환
- `↑` / `↓`: 테이블 행 선택
- `Enter`: 상세보기 또는 액션 실행
- `q`: 종료
- `c`: 수집 시작 (자원 사용률 탭)
- `s`: 조회 시작 (세션 브라우저 탭)

### 5. 구현 단계

#### Phase 1: 기본 구조 및 탭 UI
1. Ratatui 기본 화면 구조 (Ratatui Demo 예제 참고)
   - `app.rs`: 앱 상태 관리 (탭 인덱스, 각 탭의 상태)
   - `ui.rs`: UI 렌더링 로직 (Tabs, Table 위젯)
   - `crossterm.rs`: 이벤트 처리 및 터미널 설정
2. 탭 기반 UI 구현 (Tabs 위젯)
   - 상단 탭 헤더
   - 탭 전환 로직 (`Tab`, `Shift+Tab`, 숫자 키)
3. 설정 파일 읽기 (`config/proxies.json`)
4. 각 탭의 기본 레이아웃 구성
   - Table 위젯으로 데이터 표시 준비

#### Phase 2: 자원 사용률 모니터링
1. SNMP 클라이언트 구현
2. 자원 수집 기능
3. Table 위젯으로 수집 결과 표시
   - 프록시별 CPU, MEM, 시간 등을 테이블로 표시
   - 행 선택 및 하이라이트
   - 스크롤 가능한 테이블
4. CSV 파일로 저장

#### Phase 3: 세션 브라우저
1. SSH 클라이언트 구현
2. 세션 정보 파싱
3. Table 위젯으로 세션 목록 표시
   - 호스트, 클라이언트 IP, URL 등을 테이블로 표시
   - 스크롤 가능한 리스트 (ListState 활용)
   - 행 선택 및 상세보기
4. 간단한 필터링 (텍스트 검색)
5. CSV 파일로 저장

#### Phase 4: 트래픽 로그 분석
1. SSH를 통한 로그 조회
2. 파일 업로드 처리
3. 로그 파싱 및 분석
4. Table 위젯으로 TOP N 결과 표시
   - TOP 클라이언트, TOP 호스트 등을 테이블로 표시
5. BarChart로 시각화
6. CSV 파일로 저장

### 6. 단순화 포인트 요약

✅ **유지할 기능:**
- SNMP/SSH를 통한 데이터 수집
- 기본적인 분석 기능
- 탭 기반 UI (Ratatui)

⚠️ **단순화할 기능:**
- 데이터 저장: SQLite → CSV 파일 (로그/결과)
- 프록시 관리: CRUD 제거 → 설정 파일로 관리
- 이력 관리: 90일 보관 → CSV 파일로 계속 저장
- 비밀번호: 복잡한 암호화 → 평문 또는 간단한 인코딩

❌ **제거할 기능:**
- 프록시 CRUD 기능 (설정 파일로 대체)
- 백그라운드 자동 수집
- 웹소켓 실시간 업데이트
- 복잡한 설정 옵션
- 웹 UI 관련 모든 기능
- 데이터베이스 저장 (CSV 파일로 대체)

### 7. 데이터 구조 예시

#### config/proxies.json (설정 파일)
```json
{
  "proxies": [
    {
      "id": 1,
      "host": "192.168.1.10",
      "port": 22,
      "username": "admin",
      "password": "plain_text_or_base64",
      "group": "프로덕션",
      "traffic_log_path": "/var/log/proxy.log",
      "snmp_community": "public"
    },
    {
      "id": 2,
      "host": "192.168.1.11",
      "port": 22,
      "username": "admin",
      "password": "plain_text_or_base64",
      "group": "프로덕션",
      "traffic_log_path": "/var/log/proxy.log",
      "snmp_community": "public"
    }
  ]
}
```

#### config/resource_config.json
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

#### logs/resource_usage_20240101_142315.csv
```csv
timestamp,proxy_id,host,cpu,mem,cc,cs,http,https,ftp
2024-01-01 14:23:15,1,192.168.1.10,45.2,67.3,1234,5678,1024,2048,0
2024-01-01 14:23:15,2,192.168.1.11,32.1,54.2,987,4321,512,1024,0
```

#### logs/sessions_20240101_142500.csv
```csv
timestamp,proxy_id,host,transaction,client_ip,server_ip,url,protocol,age_seconds
2024-01-01 14:25:00,1,192.168.1.10,T12345,1.2.3.4,10.0.0.1,http://example.com,HTTP,120
2024-01-01 14:25:00,1,192.168.1.10,T12346,1.2.3.5,10.0.0.2,https://example.org,HTTPS,45
```

#### logs/traffic_analysis_20240101_144000.csv
```csv
type,rank,value,count,bytes
top_clients,1,1.2.3.4,123,1048576
top_clients,2,1.2.3.5,89,524288
top_hosts,1,example.com,456,2097152
top_hosts,2,example.org,234,1048576
```

---

## 결론

이 방안으로 구현하면:
- ✅ 데이터베이스 의존성 제거
- ✅ 웹 서버/브라우저 불필요
- ✅ 단순한 파일 기반 저장 (CSV)
- ✅ 설정 파일 기반 프록시 관리 (CRUD 불필요)
- ✅ 탭 기반 UI로 직관적인 네비게이션
- ✅ 모든 결과를 CSV로 저장하여 외부 도구로 분석 가능
- ✅ TUI로 터미널에서 직접 사용 가능
- ✅ 핵심 기능 유지하면서 복잡도 대폭 감소

**주요 변경사항 요약:**
1. **프록시 관리**: CRUD 제거 → `config/proxies.json` 설정 파일로 관리
2. **UI 구조**: 메뉴 방식 → 탭 기반 UI (Crossterm Demo 스타일)
3. **데이터 저장**: JSON Lines → CSV 파일 (모든 로그/결과)
4. **파일 구조**: `data/` → `config/` (설정) + `logs/` (결과)

단계적으로 구현하여 각 기능을 검증하면서 진행하는 것을 권장합니다.

