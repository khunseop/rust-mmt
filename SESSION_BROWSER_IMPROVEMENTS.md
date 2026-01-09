# 세션 브라우저 개선 방안

## 현재 문제점

1. **컬럼 내용이 모두 보이지 않음**: 고정 너비 사용으로 긴 내용이 잘림
2. **조작이 느림**: 대량 데이터 렌더링 시 성능 저하
3. **페이지네이션 없음**: 모든 데이터를 한 번에 표시
4. **열 선택 기능 없음**: 정렬이나 특정 열 기준 작업 불가
5. **상세보기 없음**: 셀 선택 후 상세 정보 확인 불가

## 개선 방안

### 1. 컬럼 너비 자동 조절

**목표**: URL을 제외한 모든 컬럼의 너비를 자동으로 조절하여 내용이 잘리지 않도록 함

**구현 방법**:
- `Constraint::Min(min_width)` 사용하여 최소 너비 보장
- `Constraint::Percentage(percent)` 사용하여 가용 공간에 비례 배분
- URL 컬럼만 `Constraint::Min(30)` 고정 (긴 URL은 말줄임표 처리)
- 각 컬럼의 실제 데이터 길이를 계산하여 동적 너비 결정

**예시**:
```rust
let constraints = vec![
    Constraint::Min(12),  // 호스트 (최소 12자)
    Constraint::Min(10),  // 트랜잭션
    Constraint::Min(19),  // 생성시간 (고정)
    Constraint::Min(8),   // 프로토콜
    Constraint::Percentage(5),  // CustID (비율)
    Constraint::Percentage(5),  // 사용자
    Constraint::Min(15),  // 클라이언트IP
    // ... 
    Constraint::Min(30),  // URL (고정, 긴 내용은 말줄임표)
];
```

### 2. 페이지네이션

**목표**: 대량 데이터를 페이지 단위로 나누어 표시하여 성능 향상

**구현 방법**:
- `SessionBrowserState`에 페이지네이션 상태 추가:
  - `page_size: usize` (기본값: 50)
  - `current_page: usize` (0부터 시작)
  - `total_pages: usize`
- 키보드 단축키:
  - `PageDown` / `Space`: 다음 페이지
  - `PageUp` / `b`: 이전 페이지
  - `Home`: 첫 페이지
  - `End`: 마지막 페이지
  - `g`: 페이지 번호 입력 (나중에)
- UI에 페이지 정보 표시: "페이지 1/10 (총 500개)"

**상태 구조**:
```rust
pub struct SessionBrowserState {
    // ... 기존 필드
    pub page_size: usize,
    pub current_page: usize,
    pub total_pages: usize,
}
```

### 3. 열 선택 및 정렬

**목표**: 특정 열을 선택하여 정렬하거나 해당 열 데이터 활용

**구현 방법**:
- `selected_column: Option<usize>` 추가 (선택된 컬럼 인덱스)
- 키보드 단축키:
  - `Tab` (테이블 모드에서): 컬럼 헤더로 포커스 이동
  - `←` / `→`: 컬럼 선택 이동
  - `Enter`: 선택된 컬럼 기준 정렬 (오름차순/내림차순 토글)
  - `s`: 정렬 모드 토글 (오름차순/내림차순/정렬 없음)
- 정렬 상태 표시: 헤더에 `↑` / `↓` 표시
- 정렬 로직: 각 컬럼 타입에 맞는 정렬 (문자열, 숫자, 날짜)

**상태 구조**:
```rust
pub struct SessionBrowserState {
    // ... 기존 필드
    pub selected_column: Option<usize>,
    pub sort_column: Option<usize>,
    pub sort_ascending: bool,
}
```

### 4. 셀 상세보기 모달창

**목표**: 선택된 행의 모든 필드를 모달창으로 표시

**구현 방법**:
- `show_detail_modal: bool` 플래그 추가
- `Enter` 키로 모달 표시/숨김
- 모달 레이아웃:
  - 중앙에 팝업 창
  - 모든 필드를 키-값 쌍으로 표시
  - 스크롤 가능 (긴 URL 등)
  - `Esc` / `Enter` / `q`로 닫기

**모달 구조**:
```
┌─────────────────────────────────────┐
│  세션 상세 정보              [Esc: 닫기] │
├─────────────────────────────────────┤
│  호스트: 192.168.1.10               │
│  트랜잭션: T12345                    │
│  생성시간: 2024-01-08 14:23:15      │
│  프로토콜: HTTP                      │
│  ...                                 │
│  URL: https://example.com/very/...  │
│         [스크롤 가능]                │
└─────────────────────────────────────┘
```

### 5. 성능 최적화

**목표**: 렌더링 속도 향상

**구현 방법**:
1. **가상 스크롤링**: 화면에 보이는 행만 렌더링
2. **데이터 캐싱**: 포맷팅된 문자열 캐싱
3. **렌더링 최적화**: 변경된 부분만 업데이트
4. **페이지네이션과 결합**: 페이지 단위로만 렌더링

**최적화 전략**:
- 현재 페이지의 데이터만 포맷팅
- 선택된 행만 하이라이트 계산
- 컬럼 너비 계산 캐싱

### 6. 고급 데이터 테이블 라이브러리 검토

**Ratatui 한계**:
- Ratatui는 기본적인 Table 위젯만 제공
- 고급 기능(정렬, 필터링, 컬럼 리사이즈 등)은 직접 구현 필요

**대안 검토**:
1. **tui-table-rs**: Ratatui 기반 고급 테이블 위젯
   - 정렬, 필터링 지원
   - 컬럼 리사이즈 가능
   - 하지만 프로젝트가 비활성화된 것으로 보임

2. **직접 구현**: Ratatui Table 위젯을 확장하여 필요한 기능 추가
   - 가장 안정적인 방법
   - 프로젝트 요구사항에 맞게 커스터마이징 가능

**결론**: Ratatui의 기본 Table 위젯을 확장하여 필요한 기능을 직접 구현하는 것이 가장 현실적

## 구현 우선순위

### Phase 1: 필수 개선 (즉시 구현)
1. ✅ 컬럼 너비 자동 조절
2. ✅ 페이지네이션
3. ✅ 성능 최적화

### Phase 2: 사용성 개선 (단기)
4. ✅ 열 선택 및 정렬
5. ✅ 셀 상세보기 모달

### Phase 3: 고급 기능 (중기)
6. ⏳ 검색 기능
7. ⏳ 컬럼 순서 변경

## 구현 세부사항

### 컬럼 너비 자동 조절 알고리즘

```rust
fn calculate_column_widths(
    sessions: &[SessionData],
    available_width: usize,
    url_min_width: usize,
) -> Vec<Constraint> {
    let mut max_widths = vec![0; 19];
    
    // 각 컬럼의 최대 내용 길이 계산
    for session in sessions {
        max_widths[0] = max_widths[0].max(session.host.len());
        max_widths[1] = max_widths[1].max(session.transaction.as_ref().map(|s| s.len()).unwrap_or(0));
        // ... 모든 컬럼
    }
    
    // 헤더 길이도 고려
    let headers = vec!["호스트", "트랜잭션", ...];
    for (i, header) in headers.iter().enumerate() {
        max_widths[i] = max_widths[i].max(header.len());
    }
    
    // URL을 제외한 총 너비 계산
    let url_index = 18;
    let total_non_url_width: usize = max_widths.iter()
        .enumerate()
        .filter(|(i, _)| *i != url_index)
        .map(|(_, &w)| w + 2) // 패딩 포함
        .sum();
    
    let url_width = url_min_width;
    let remaining_width = available_width.saturating_sub(url_width);
    
    // 비율 계산하여 Constraint 생성
    let mut constraints = Vec::new();
    for (i, &max_w) in max_widths.iter().enumerate() {
        if i == url_index {
            constraints.push(Constraint::Min(url_width));
        } else {
            let ratio = if total_non_url_width > 0 {
                (max_w as f64 / total_non_url_width as f64) * 100.0
            } else {
                100.0 / (max_widths.len() - 1) as f64
            };
            constraints.push(Constraint::Min(max_w.max(8)).max(Constraint::Percentage(ratio as u16)));
        }
    }
    
    constraints
}
```

### 페이지네이션 로직

```rust
impl SessionBrowserState {
    fn get_paginated_sessions(&self, sessions: &[SessionData]) -> Vec<&SessionData> {
        let start = self.current_page * self.page_size;
        let end = (start + self.page_size).min(sessions.len());
        sessions[start..end].iter().collect()
    }
    
    fn next_page(&mut self) {
        if self.current_page < self.total_pages.saturating_sub(1) {
            self.current_page += 1;
            // 선택된 행을 페이지 내 첫 번째로 리셋
            self.table_state.select(Some(0));
        }
    }
    
    fn previous_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.table_state.select(Some(0));
        }
    }
    
    fn update_total_pages(&mut self, total_items: usize) {
        self.total_pages = (total_items + self.page_size - 1) / self.page_size;
        if self.current_page >= self.total_pages {
            self.current_page = self.total_pages.saturating_sub(1);
        }
    }
}
```

### 정렬 로직

```rust
fn sort_sessions(sessions: &mut [SessionData], column: usize, ascending: bool) {
    match column {
        0 => sessions.sort_by(|a, b| {
            let ord = a.host.cmp(&b.host);
            if ascending { ord } else { ord.reverse() }
        }),
        1 => sessions.sort_by(|a, b| {
            let ord = a.transaction.cmp(&b.transaction);
            if ascending { ord } else { ord.reverse() }
        }),
        2 => sessions.sort_by(|a, b| {
            let ord = a.creation_time.cmp(&b.creation_time);
            if ascending { ord } else { ord.reverse() }
        }),
        // 숫자 컬럼
        10 => sessions.sort_by(|a, b| {
            let ord = a.cl_bytes_received.cmp(&b.cl_bytes_received);
            if ascending { ord } else { ord.reverse() }
        }),
        // ... 모든 컬럼
        _ => {}
    }
}
```

## 키보드 단축키 정리

### 기존 단축키
- `S`: 세션 조회 시작
- `←` / `→`: 컬럼 스크롤
- `Shift+←` / `Shift+→`: 그룹 선택
- `↑` / `↓`: 행 이동

### 추가 단축키
- `PageDown` / `Space`: 다음 페이지
- `PageUp` / `b`: 이전 페이지
- `Home`: 첫 페이지
- `End`: 마지막 페이지
- `Tab`: 컬럼 헤더 선택 모드 (테이블 모드에서)
- `Enter`: 
  - 컬럼 헤더 선택 모드: 정렬 토글
  - 행 선택 모드: 상세보기 모달 표시
- `s`: 정렬 방향 토글 (오름차순/내림차순/정렬 없음)
- `Esc`: 모달 닫기

## UI 레이아웃 변경

```
┌─────────────────────────────────────────────────────────┐
│  [그룹선택] [상태] [마지막조회] [페이지: 1/10 (500개)]   │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────┐ │
│  │ 호스트│트랜잭션│생성시간│...│URL              [↑]    │ │ ← 정렬 표시
│  ├─────────────────────────────────────────────────────┤ │
│  │ 192...│T12345 │2024... │...│https://...           │ │
│  │ 192...│T12346 │2024... │...│https://...           │ │
│  │ ...   │...    │...     │...│...                   │ │
│  └─────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────┤
│  [단축키 도움말]                                         │
└─────────────────────────────────────────────────────────┘
```

## 참고 자료

- [Ratatui Table 문서](https://docs.rs/ratatui/latest/ratatui/widgets/struct.Table.html)
- [Ratatui Layout 문서](https://docs.rs/ratatui/latest/ratatui/layout/index.html)
- [Ratatui Popup 예제](https://github.com/ratatui-org/ratatui/tree/main/examples)
