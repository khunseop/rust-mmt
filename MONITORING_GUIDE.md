# 모니터링 설정 가이드

이 문서는 MWG 모니터링 도구의 설정 방법을 설명합니다.

## 목차

1. [SNMP 설정](#snmp-설정)
2. [수집 주기 설정](#수집-주기-설정)
3. [OID 설정](#oid-설정)
4. [인터페이스(회선) 설정](#인터페이스회선-설정)
5. [임계치 설정](#임계치-설정)
6. [프록시 설정](#프록시-설정)

---

## SNMP 설정

### SNMP 버전

본 도구는 **SNMP v2c**를 사용합니다. 설정 파일에서 명시적으로 지정할 수 있습니다.

### Community String 설정

`config/resource_config.json` 파일에서 SNMP community string을 설정합니다.

```json
{
  "snmp_version": "2c",
  "community": "public"
}
```

**중요**: `config/proxies.json`에는 더 이상 `snmp_community` 필드를 사용하지 않습니다. 모든 SNMP 설정은 `resource_config.json`에서 관리합니다.

---

## 수집 주기 설정

### 자동 수집 설정

자원 사용률 탭에서 자동 수집을 활성화하고 수집 주기를 설정할 수 있습니다.

#### 키보드 단축키

- **`C`**: 수동 수집 시작
- **`Space`**: 자동 수집 시작/중지 토글
- **`+` 또는 `=`**: 수집 주기 증가 (10초 → 30초 → 60초 → 120초 → 300초 → 600초)
- **`-`**: 수집 주기 감소 (600초 → 300초 → 120초 → 60초 → 30초 → 10초)
- **`Shift+←` / `Shift+→`**: 그룹 선택 (전체보기 포함)

#### 수집 주기 단계

- 10초
- 30초
- 60초 (기본값)
- 120초 (2분)
- 300초 (5분)
- 600초 (10분)

#### 수집 상태 표시

- **대기 중 (Idle)**: 수집 대기 상태
- **시작 중 (Starting)**: 수집 시작 중
- **수집 중 (Collecting)**: 데이터 수집 중 (스피너 애니메이션 표시)
- **성공 (Success)**: 수집 완료
- **실패 (Failed)**: 수집 실패

#### 자동 수집 동작

1. 자동 수집을 활성화하면 설정된 주기마다 자동으로 수집이 시작됩니다.
2. 수집이 완료되면 다음 수집 시간이 자동으로 계산되어 표시됩니다.
3. 수집 중에는 새로운 수집 요청이 무시됩니다.

---

## OID 설정

`config/resource_config.json` 파일의 `oids` 섹션에서 각 지표의 OID를 설정합니다.

### 지원하는 지표

- **cpu**: CPU 사용률 (SNMP OID 또는 "ssh"로 SSH 수집)
- **mem**: 메모리 사용률 (SNMP OID 또는 "ssh"로 SSH 수집)
- **cc**: Current Connections (현재 연결 수)
- **cs**: Current Sessions (현재 세션 수)
- **http**: HTTP 트래픽
- **https**: HTTPS 트래픽
- **ftp**: FTP 트래픽

### 설정 예시

```json
{
  "oids": {
    "cpu": "1.3.6.1.4.1.2021.11.11.0",
    "mem": "ssh",
    "cc": "1.3.6.1.4.1.2021.4.11.0",
    "cs": "1.3.6.1.4.1.2021.4.11.0",
    "http": "",
    "https": "",
    "ftp": ""
  }
}
```

### OID 설정 규칙

1. **빈 문자열 (`""`)**: 해당 지표를 수집하지 않음
2. **"ssh"**: SSH를 통해 수집 (현재는 `mem`만 지원)
3. **유효한 OID**: SNMP를 통해 해당 OID에서 값을 수집

### OID 찾기 방법

1. **SNMP MIB 브라우저 사용**: MIB 파일을 로드하여 원하는 지표의 OID 확인
2. **SNMP Walk 사용**: `snmpwalk -v 2c -c public <host>` 명령으로 사용 가능한 OID 탐색
3. **제조사 문서 참조**: 장비 제조사의 SNMP MIB 문서 참조

---

## 인터페이스(회선) 설정

네트워크 인터페이스의 트래픽(In/Out)을 수집하려면 `config/resource_config.json` 파일의 `interface_oids` 섹션을 설정합니다.

### 사전 정의된 인터페이스

다음 인터페이스가 기본적으로 정의되어 있습니다:

- **eth0**: 이더넷 인터페이스 0
- **eth4**: 이더넷 인터페이스 4
- **eth5**: 이더넷 인터페이스 5
- **eth6**: 이더넷 인터페이스 6
- **eth7**: 이더넷 인터페이스 7
- **bond0**: 본딩 인터페이스 0
- **bond1**: 본딩 인터페이스 1

### 설정 형식

각 인터페이스는 `in_oid`와 `out_oid`를 가집니다:

```json
{
  "interface_oids": {
    "eth0": {
      "in_oid": "1.3.6.1.2.1.2.2.1.10.2",
      "out_oid": "1.3.6.1.2.1.2.2.1.16.2"
    },
    "eth4": {
      "in_oid": "1.3.6.1.2.1.2.2.1.10.6",
      "out_oid": "1.3.6.1.2.1.2.2.1.16.6"
    },
    "eth5": {
      "in_oid": "1.3.6.1.2.1.2.2.1.10.7",
      "out_oid": "1.3.6.1.2.1.2.2.1.16.7"
    },
    "eth6": {
      "in_oid": "1.3.6.1.2.1.2.2.1.10.8",
      "out_oid": "1.3.6.1.2.1.2.2.1.16.8"
    },
    "eth7": {
      "in_oid": "1.3.6.1.2.1.2.2.1.10.9",
      "out_oid": "1.3.6.1.2.1.2.2.1.16.9"
    },
    "bond0": {
      "in_oid": "1.3.6.1.2.1.2.2.1.10.10",
      "out_oid": "1.3.6.1.2.1.2.2.1.16.10"
    },
    "bond1": {
      "in_oid": "1.3.6.1.2.1.2.2.1.11.11",
      "out_oid": "1.3.6.1.2.1.2.2.1.16.11"
    }
  }
}
```

### 인터페이스 OID 찾기

#### 1. SNMP Walk 사용

```bash
# 인터페이스 이름 확인
snmpwalk -v 2c -c public <host> 1.3.6.1.2.1.2.2.1.2

# 입력 바이트 수 (ifInOctets)
snmpwalk -v 2c -c public <host> 1.3.6.1.2.1.2.2.1.10

# 출력 바이트 수 (ifOutOctets)
snmpwalk -v 2c -c public <host> 1.3.6.1.2.1.2.2.1.16
```

#### 2. 표준 SNMP OID

- **ifInOctets**: `1.3.6.1.2.1.2.2.1.10.{interface_index}`
- **ifOutOctets**: `1.3.6.1.2.1.2.2.1.16.{interface_index}`
- **ifDescr**: `1.3.6.1.2.1.2.2.1.2.{interface_index}` (인터페이스 이름 확인용)

`{interface_index}`는 각 인터페이스의 고유 인덱스 번호입니다.

#### 3. 인터페이스 인덱스 확인

```bash
# 인터페이스 이름과 인덱스 매핑 확인
snmpwalk -v 2c -c public <host> 1.3.6.1.2.1.2.2.1.2 | grep -i eth0
```

출력 예시:
```
IF-MIB::ifDescr.2 = STRING: eth0
```

이 경우 `eth0`의 인덱스는 `2`이므로:
- `in_oid`: `1.3.6.1.2.1.2.2.1.10.2`
- `out_oid`: `1.3.6.1.2.1.2.2.1.16.2`

### 트래픽 계산 방식

- 인터페이스 트래픽은 **Mbps (Megabits per second)** 단위로 계산됩니다.
- 이전 수집 값과 현재 값을 비교하여 시간당 변화량을 계산합니다.
- 32비트 카운터 오버플로우를 자동으로 처리합니다.
- 최소 1초 이상의 시간 차이가 있어야 유효한 값으로 계산됩니다.

### 설정 주의사항

1. **OID 형식**: 점(.)으로 구분된 숫자 형식이어야 합니다.
2. **빈 문자열**: `in_oid` 또는 `out_oid`가 빈 문자열이면 해당 방향의 트래픽을 수집하지 않습니다.
3. **인터페이스 이름**: 설정 파일의 키(예: "eth0")는 표시용 이름이며, 실제 OID와 일치해야 합니다.

---

## 임계치 설정

자원 사용률 모니터링에서 각 지표의 임계치를 설정하여 색상으로 상태를 표시할 수 있습니다.

### 임계치 색상 규칙

- **하얀색**: warning 임계치 미만 (정상)
- **노란색**: warning 이상, critical 미만 (경고)
- **빨간색**: critical 이상 (위험)

### 설정 형식

`config/resource_config.json` 파일의 `thresholds` 섹션에서 각 지표의 임계치를 설정합니다:

```json
{
  "thresholds": {
    "cpu": {
      "warning": 70.0,
      "critical": 90.0
    },
    "mem": {
      "warning": 70.0,
      "critical": 90.0
    },
    "cc": {
      "warning": 10000.0,
      "critical": 50000.0
    },
    "cs": {
      "warning": 10000.0,
      "critical": 50000.0
    },
    "http": {
      "warning": 1000000000.0,
      "critical": 5000000000.0
    },
    "https": {
      "warning": 1000000000.0,
      "critical": 5000000000.0
    },
    "ftp": {
      "warning": 1000000000.0,
      "critical": 5000000000.0
    },
    "interface_traffic": {
      "warning": 1000000000.0,
      "critical": 5000000000.0
    }
  }
}
```

### 지표별 단위

- **cpu, mem**: 퍼센트 (0-100)
- **cc, cs**: 개수
- **http, https, ftp**: bps (비트/초)
- **interface_traffic**: bps (비트/초, in/out 중 더 높은 값 기준)

### 기본값

설정 파일에 임계치가 없거나 일부 값이 없으면 다음 기본값이 사용됩니다:

- CPU, MEM: warning 70%, critical 90%
- CC, CS: warning 10000, critical 50000
- HTTP, HTTPS, FTP: warning 1Gbps, critical 5Gbps
- 인터페이스 트래픽: warning 1Gbps, critical 5Gbps

### 예시

CPU 사용률이 75%인 경우:
- warning(70%) 이상이므로 **노란색**으로 표시됩니다.

CPU 사용률이 95%인 경우:
- critical(90%) 이상이므로 **빨간색**으로 표시됩니다.

인터페이스 트래픽의 경우, in/out 중 더 높은 값을 기준으로 색상이 결정됩니다.

---

## 프록시 설정

`config/proxies.json` 파일에서 모니터링할 프록시 서버를 설정합니다.

### 설정 형식

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
      "traffic_log_path": "/var/log/proxy.log"
    }
  ]
}
```

### 필드 설명

- **id**: 프록시 고유 ID (숫자)
- **host**: 프록시 서버 IP 주소 또는 호스트명
- **port**: SSH 포트 (기본값: 22)
- **username**: SSH 사용자명
- **password**: SSH 비밀번호
- **group**: 프록시 그룹명 (필터링에 사용)
- **traffic_log_path**: 트래픽 로그 파일 경로 (선택사항)

### 주의사항

- **SNMP Community**: `proxies.json`에서 제거되었습니다. 모든 SNMP 설정은 `resource_config.json`에서 관리합니다.
- **비밀번호**: 현재는 평문으로 저장되므로 보안에 주의하세요.

---

## 전체 설정 파일 예시

### config/resource_config.json

```json
{
  "snmp_version": "2c",
  "community": "public",
  "oids": {
    "cpu": "1.3.6.1.4.1.2021.11.11.0",
    "mem": "ssh",
    "cc": "1.3.6.1.4.1.2021.4.11.0",
    "cs": "1.3.6.1.4.1.2021.4.11.0",
    "http": "",
    "https": "",
    "ftp": ""
  },
  "interface_oids": {
    "eth0": {
      "in_oid": "1.3.6.1.2.1.2.2.1.10.2",
      "out_oid": "1.3.6.1.2.1.2.2.1.16.2"
    },
    "eth4": {
      "in_oid": "1.3.6.1.2.1.2.2.1.10.6",
      "out_oid": "1.3.6.1.2.1.2.2.1.16.6"
    },
    "eth5": {
      "in_oid": "1.3.6.1.2.1.2.2.1.10.7",
      "out_oid": "1.3.6.1.2.1.2.2.1.16.7"
    },
    "eth6": {
      "in_oid": "1.3.6.1.2.1.2.2.1.10.8",
      "out_oid": "1.3.6.1.2.1.2.2.1.16.8"
    },
    "eth7": {
      "in_oid": "1.3.6.1.2.1.2.2.1.10.9",
      "out_oid": "1.3.6.1.2.1.2.2.1.16.9"
    },
    "bond0": {
      "in_oid": "1.3.6.1.2.1.2.2.1.10.10",
      "out_oid": "1.3.6.1.2.1.2.2.1.16.10"
    },
    "bond1": {
      "in_oid": "1.3.6.1.2.1.2.2.1.10.11",
      "out_oid": "1.3.6.1.2.1.2.2.1.16.11"
    }
  },
  "thresholds": {
    "cpu": {
      "warning": 70.0,
      "critical": 90.0
    },
    "mem": {
      "warning": 70.0,
      "critical": 90.0
    },
    "cc": {
      "warning": 10000.0,
      "critical": 50000.0
    },
    "cs": {
      "warning": 10000.0,
      "critical": 50000.0
    },
    "http": {
      "warning": 1000000000.0,
      "critical": 5000000000.0
    },
    "https": {
      "warning": 1000000000.0,
      "critical": 5000000000.0
    },
    "ftp": {
      "warning": 1000000000.0,
      "critical": 5000000000.0
    },
    "interface_traffic": {
      "warning": 1000000000.0,
      "critical": 5000000000.0
    }
  }
}
```

### config/proxies.json

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
      "traffic_log_path": "/var/log/proxy.log"
    },
    {
      "id": 2,
      "host": "192.168.1.11",
      "port": 22,
      "username": "admin",
      "password": "password123",
      "group": "프로덕션",
      "traffic_log_path": "/var/log/proxy.log"
    }
  ]
}
```

---

## 문제 해결

### SNMP 연결 실패

1. **방화벽 확인**: UDP 포트 161이 열려있는지 확인
2. **Community String 확인**: `resource_config.json`의 `community` 값이 올바른지 확인
3. **SNMP 서비스 확인**: 대상 서버에서 SNMP 서비스가 실행 중인지 확인

### 인터페이스 트래픽이 0으로 표시됨

1. **OID 확인**: `interface_oids`의 OID가 올바른지 확인
2. **인터페이스 인덱스 확인**: `snmpwalk`로 실제 인터페이스 인덱스 확인
3. **첫 수집 대기**: 첫 수집 후 두 번째 수집부터 트래픽이 계산됩니다.

### 수집 주기가 변경되지 않음

- `+`/`-` 키를 사용하여 변경하세요.
- 자동 수집이 활성화되어 있어야 주기 설정이 의미가 있습니다.

---

## 참고 자료

- [SNMP MIB 문서](http://www.oid-info.com/)
- [RFC 1157 - SNMP v1](https://tools.ietf.org/html/rfc1157)
- [RFC 3416 - SNMP v2c](https://tools.ietf.org/html/rfc3416)

