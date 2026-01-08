# 프로젝트 진행률 문서

## 전체 진행 상황

**전체 진행률: 60%** (Phase 1 완료, Phase 2 완료)

---

## Phase별 진행 상황

### ✅ Phase 1: 기본 구조 및 탭 UI (완료)

**진행률: 100%** (2024-01-XX 완료)

#### 완료된 작업

1. **프로젝트 구조 설정**
   - ✅ Cargo.toml 의존성 추가 (ratatui, crossterm, serde, tokio 등)
   - ✅ 기본 모듈 구조 생성 (app.rs, ui.rs, crossterm.rs)

2. **앱 상태 관리**
   - ✅ App 구조체 정의
   - ✅ 탭 인덱스 및 상태 관리 (TabIndex enum) - 4개 탭 (프록시관리, 자원사용률, 세션브라우저, 트래픽로그)
   - ✅ 각 탭별 상태 구조체 (ResourceUsageState, SessionBrowserState, TrafficLogsState)
   - ✅ 프록시 데이터 구조 정의
   - ✅ 그룹 선택 상태 관리
   - ✅ 수집 주기 설정 상태 관리
   - ✅ 마지막 수집 시간 추적

3. **UI 구현**
   - ✅ 탭 기반 UI (Tabs 위젯)
   - ✅ 각 탭의 기본 레이아웃
   - ✅ Table 위젯 준비 (자원사용률, 세션브라우저 탭)
   - ✅ 키보드 네비게이션 (탭 전환, 행 선택)
   - ✅ 프록시관리 탭 추가 (프록시 목록 표시)
   - ✅ 그룹 선택 기능 (전체보기 포함)
   - ✅ 수집 주기 설정 UI (+/- 키로 변경)
   - ✅ 마지막 수집 시간 표시

4. **터미널 설정**
   - ✅ Crossterm 백엔드 설정
   - ✅ 이벤트 루프 구현
   - ✅ 키보드 이벤트 처리

5. **설정 파일**
   - ✅ config/proxies.json 예제 생성
   - ✅ config/resource_config.json 예제 생성
   - ✅ 설정 파일 읽기 기능

#### 현재 상태

- ✅ 기본 TUI 앱 실행 가능
- ✅ 탭 전환 및 네비게이션 작동
- ✅ 빈 테이블 레이아웃 표시
- ⚠️ 실제 데이터 수집 기능은 아직 미구현

#### 테스트 결과

```bash
$ cargo build
   Compiling rust-mmt v0.1.0
   Finished dev [unoptimized + debuginfo] target(s) in 4.91s

$ cargo run
# 앱 실행 성공, 탭 전환 및 기본 UI 작동 확인
```

---

### ✅ Phase 2: 자원 사용률 모니터링 (완료)

**진행률: 100%**

#### 완료된 작업

1. **SNMP 클라이언트 구현** (완료)
   - [x] SNMP 클라이언트 모듈 생성 (src/snmp.rs)
   - [x] SNMP GET 요청 기본 구조 구현
   - [x] 비동기 SNMP GET 함수 구현
   - [x] BER 인코딩/디코딩 직접 구현 완료
   - [x] SNMPv2c GET 요청/응답 처리 완료
   - [x] INTEGER, Counter32, Gauge32, Counter64 타입 지원

2. **SSH 클라이언트 구현** (완료)
   - [x] SSH 클라이언트 모듈 생성 (src/ssh.rs)
   - [x] SSH 명령 실행 인터페이스 정의
   - [x] 메모리 정보 수집 함수 구현
   - [x] russh 크레이트 통합 완료
   - [x] 실제 SSH 연결 및 명령 실행 구현
   - [x] 타임아웃 처리 구현

3. **자원 수집 기능** (완료)
   - [x] 수집기 모듈 생성 (src/collector.rs)
   - [x] 프록시별 SNMP 수집 로직 구현
   - [x] CPU, 메모리 등 지표 수집 구조
   - [x] 비동기 수집 (여러 프록시 동시 수집)
   - [x] 수집 결과 데이터 구조화
   - [x] 인터페이스(회선) 트래픽 수집 로직 구현
   - [x] 인터페이스 카운터 캐시 및 Mbps 계산
   - [x] 32비트 카운터 오버플로우 처리

4. **UI 통합** (완료)
   - [x] 수집 키 바인딩 ('C' 키)
   - [x] Table 위젯에 데이터 표시 준비
   - [x] 수집 중 상태 플래그
   - [x] 비동기 수집 작업 통합
   - [x] 키보드 단축키 도움말 프레임 추가
   - [x] 수집 중 경과 시간 실시간 표시
   - [x] 마지막 수집 시간 표시
   - [x] 수집 상태 개선 (수집 시작 시 즉시 Collecting 상태로 변경)

5. **CSV 저장 기능** (완료)
   - [x] CSV 파일 생성 로직 (src/csv_writer.rs)
   - [x] 타임스탬프 포함 파일명 생성
   - [x] 데이터 포맷팅 및 저장
   - [x] logs/ 디렉토리 자동 생성

6. **의존성 추가** (완료)
   - [x] Cargo.toml에 russh, russh-keys 추가
   - [x] Cargo.toml에 async-trait, rand 추가

7. **설정 파일 개선** (완료)
   - [x] SNMP 버전 명시 (v2c)
   - [x] 모든 OID 필드 추가 (cpu, mem, cc, cs, http, https, ftp)
   - [x] 인터페이스 OID 설정 구조 추가 (eth0, eth4, eth5, eth6, eth7, bond0, bond1)
   - [x] SNMP community 중복 제거 (proxies.json에서 제거, resource_config.json에서만 관리)
   - [x] 인터페이스별 in_oid/out_oid 설정 지원

8. **문서화** (완료)
   - [x] 모니터링 설정 가이드 작성 (MONITORING_GUIDE.md)
   - [x] SNMP 설정 방법 문서화
   - [x] 수집 주기 설정 가이드
   - [x] OID 설정 방법 가이드
   - [x] 인터페이스(회선) 설정 가이드

9. **임계치 기반 색상 표시** (완료)
   - [x] 임계치 설정 파일 구조 추가 (resource_config.json)
   - [x] 임계치 로드 함수 구현
   - [x] 모든 컬럼에 임계치 색상 적용 (CPU, MEM, CC, CS, HTTP, HTTPS, FTP, 인터페이스 트래픽)
   - [x] 색상 규칙: 하얀색(정상), 노란색(경고), 빨간색(위험)

#### 남은 작업

1. **테스트 및 검증**
   - [ ] 실제 SNMP 서버와의 통신 테스트
   - [ ] 실제 SSH 서버와의 통신 테스트
   - [ ] 수집 기능 통합 테스트
   - [ ] 인터페이스 트래픽 수집 검증
   - [ ] 임계치 색상 표시 검증

2. **에러 처리 개선**
   - [x] 수집 실패 시 UI에 에러 메시지 표시 (완료)
   - [x] 타임아웃 처리 구현 (완료)
   - [ ] 네트워크 오류 처리 개선

#### 예상 소요 시간

- SNMP 클라이언트: 2-3일
- SSH 클라이언트: 1-2일
- 수집 로직 및 UI 통합: 2-3일
- CSV 저장: 1일
- **총 예상: 6-9일**

---

### 📋 Phase 3: 세션 브라우저 (대기 중)

**진행률: 0%**

#### 계획된 작업

1. **SSH 세션 조회**
   - [ ] 프록시 서버에 SSH 접속
   - [ ] 세션 조회 명령 실행 (MWG 명령어)
   - [ ] 출력 파싱

2. **세션 데이터 파싱**
   - [ ] 세션 정보 추출 (트랜잭션, 클라이언트 IP, URL 등)
   - [ ] 데이터 구조화

3. **UI 통합**
   - [ ] 조회 버튼/키 바인딩 (예: 'S' 키)
   - [ ] Table 위젯에 세션 목록 표시
   - [ ] 필터링 기능 (텍스트 검색)

4. **CSV 저장**
   - [ ] 세션 데이터 CSV 저장

#### 예상 소요 시간

- SSH 세션 조회: 2일
- 파싱 및 UI 통합: 2일
- **총 예상: 4일**

---

### 📋 Phase 4: 트래픽 로그 분석 (대기 중)

**진행률: 0%**

#### 계획된 작업

1. **로그 조회**
   - [ ] SSH를 통한 원격 로그 조회
   - [ ] 파일 업로드 처리

2. **로그 파싱**
   - [ ] 로그 라인 파싱
   - [ ] 필드 추출 (클라이언트 IP, 호스트, URL 등)

3. **분석 기능**
   - [ ] TOP N 분석 (클라이언트, 호스트, URL별)
   - [ ] 통계 계산

4. **UI 통합**
   - [ ] 분석 버튼/키 바인딩 (예: 'A' 키)
   - [ ] Table 위젯으로 TOP N 표시
   - [ ] BarChart로 시각화

5. **CSV 저장**
   - [ ] 분석 결과 CSV 저장

#### 예상 소요 시간

- 로그 조회 및 파싱: 3일
- 분석 로직: 2일
- UI 통합: 2일
- **총 예상: 7일**

---

## 기술 스택 현황

### ✅ 구현 완료

- `ratatui`: TUI 프레임워크
- `crossterm`: 터미널 제어
- `serde` + `serde_json`: JSON 직렬화
- `tokio`: 비동기 런타임 (준비 완료)
- `anyhow`: 에러 처리
- `chrono`: 시간 처리

### ✅ 추가 완료

- SNMP 구현: BER 인코딩/디코딩 직접 구현 완료
- SSH 라이브러리: `russh` (순수 Rust) 통합 완료
- `russh-keys`: SSH 키 관리
- `async-trait`: 비동기 트레이트 지원
- `rand`: 랜덤 값 생성 (russh 의존성)

---

## 파일 구조

```
rust-mmt/
├── src/
│   ├── main.rs          ✅ 완료
│   ├── app.rs           ✅ 완료
│   ├── ui.rs            ✅ 완료
│   ├── crossterm.rs     ✅ 완료
│   ├── snmp.rs          ✅ 완료 (기본 구조)
│   ├── ssh.rs           ✅ 완료 (기본 구조)
│   ├── collector.rs     ✅ 완료
│   └── csv_writer.rs    ✅ 완료
├── config/
│   ├── proxies.json     ✅ 완료 (예제, snmp_community 제거됨)
│   └── resource_config.json ✅ 완료 (예제, SNMP 버전 및 인터페이스 OID 추가)
├── logs/                ✅ 생성됨 (CSV 파일 저장됨)
├── Cargo.toml           ✅ 완료
├── README.md            ✅ 완료
├── ANALYSIS.md          ✅ 완료
├── MONITORING_GUIDE.md  ✅ 완료 (모니터링 설정 가이드)
└── PROGRESS.md          ✅ 완료 (이 문서)
```

---

## 다음 단계 (Phase 2 시작)

### 우선순위

1. **SNMP 라이브러리 선택 및 통합**
   - 라이브러리 비교 및 선택
   - 기본 SNMP GET 테스트

2. **프록시 설정 읽기 검증**
   - 설정 파일 로드 테스트
   - 프록시 목록 표시

3. **기본 SNMP 수집 구현**
   - 단일 프록시 SNMP 수집 테스트
   - CPU 지표 수집

4. **UI에 데이터 표시**
   - 수집된 데이터를 Table에 표시
   - 실시간 업데이트

---

## 알려진 이슈

### 현재 없음

Phase 1은 정상적으로 완료되었습니다.

---

## 참고 자료

- [Ratatui Demo 예제](https://ratatui.rs/examples/apps/demo/)
- [Ratatui Table 예제](https://ratatui.rs/examples/widgets/table/)
- [ANALYSIS.md](./ANALYSIS.md): 상세 설계 문서
- [MONITORING_GUIDE.md](./MONITORING_GUIDE.md): 모니터링 설정 가이드

---

## 업데이트 이력

- **2024-01-XX**: Phase 1 완료
  - 기본 TUI 구조 및 탭 UI 구현 완료
  - 설정 파일 읽기 기능 완료
  - Table 위젯 기본 레이아웃 완료
  - 프록시관리 탭 추가 (프록시 목록 표시)
  - 그룹 선택 기능 추가 (전체보기 포함, Shift+←/→로 변경)
  - 수집 주기 설정 추가 (기본 60초, +/- 키로 변경)
  - 마지막 수집 시간 표시 기능 추가

- **2024-01-XX**: Phase 2 진행 중 (60%)
  - SNMP 클라이언트 기본 구조 구현
  - SSH 클라이언트 기본 구조 구현
  - 자원 수집기 구현 (비동기 병렬 수집)
  - CSV 저장 기능 구현
  - UI 통합 (C 키로 수집 시작)
  - ⚠️ SNMP/SSH 실제 라이브러리 통합 필요

- **2024-01-XX**: Phase 2 진행 중 (85%)
  - SNMP 클라이언트 실제 구현 완료 (BER 인코딩/디코딩 직접 구현)
  - SSH 클라이언트 실제 구현 완료 (russh 크레이트 통합)
  - Cargo.toml에 필요한 의존성 추가 (russh, russh-keys, async-trait, rand)
  - SNMPv2c GET 요청/응답 처리 완료
  - SSH 명령 실행 및 타임아웃 처리 완료

- **2024-01-XX**: Phase 2 진행 중 (95%)
  - SNMP 설정 개선: 버전 명시(v2c), community 중복 제거
  - 모든 OID 필드 추가 (cpu, mem, cc, cs, http, https, ftp)
  - 인터페이스(회선) 설정 구조 추가 (eth0, eth4, eth5, eth6, eth7, bond0, bond1)
  - 인터페이스 트래픽 수집 로직 구현 (in/out Mbps 계산)
  - 인터페이스 카운터 캐시 및 오버플로우 처리 구현
  - UI 개선: 키보드 단축키 도움말 프레임 추가
  - UI 개선: 수집 중 경과 시간 실시간 표시
  - UI 개선: 마지막 수집 시간 표시
  - UI 개선: 수집 시작 시 즉시 Collecting 상태로 변경
  - 모니터링 설정 가이드 문서 작성 (MONITORING_GUIDE.md)

- **2024-01-XX**: Phase 2 완료 (100%)
  - 임계치 기반 색상 표시 기능 구현
  - resource_config.json에 thresholds 섹션 추가
  - 모든 컬럼에 임계치 색상 적용 (CPU, MEM, CC, CS, HTTP, HTTPS, FTP, 인터페이스 트래픽)
  - 색상 규칙: 하얀색(정상), 노란색(경고), 빨간색(위험)
  - 임계치 설정 파일에서 읽어오기 기능 구현

---

## 체크리스트

### Phase 1 ✅
- [x] 프로젝트 구조 설정
- [x] 앱 상태 관리
- [x] 탭 기반 UI (4개 탭)
- [x] 터미널 설정
- [x] 설정 파일 구조
- [x] 프록시관리 탭 구현
- [x] 그룹 선택 기능
- [x] 수집 주기 설정
- [x] 마지막 수집 시간 표시

### Phase 2 (완료 - 100%)
- [x] SNMP 클라이언트 기본 구조
- [x] SSH 클라이언트 기본 구조
- [x] 자원 수집 기능
- [x] CSV 저장
- [x] UI 통합
- [x] SNMP 실제 구현 (BER 인코딩/디코딩 직접 구현)
- [x] SSH 실제 라이브러리 통합 (russh)
- [x] SNMP 설정 개선 (버전 명시, community 중복 제거)
- [x] 모든 OID 필드 추가
- [x] 인터페이스(회선) 설정 및 수집 로직 구현
- [x] UI 개선 (키보드 단축키 도움말, 경과 시간, 마지막 수집 시간)
- [x] 모니터링 설정 가이드 문서 작성
- [x] 임계치 기반 색상 표시 기능 구현
- [x] 모든 컬럼에 임계치 색상 적용
- [ ] 실제 환경 테스트 및 검증

### Phase 3
- [ ] SSH 세션 조회
- [ ] 세션 파싱
- [ ] UI 통합
- [ ] CSV 저장

### Phase 4
- [ ] 로그 조회
- [ ] 로그 파싱
- [ ] 분석 기능
- [ ] BarChart 시각화
- [ ] CSV 저장

