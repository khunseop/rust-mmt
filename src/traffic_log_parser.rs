use anyhow::Result;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use std::collections::HashMap;

/// 트래픽 로그 필드 (28개)
const FIELDS: &[&str] = &[
    "datetime", "username", "client_ip", "url_destination_ip", "timeintransaction",
    "response_statuscode", "cache_status", "comm_name", "url_protocol", "url_host",
    "url_path", "url_parametersstring", "url_port", "url_categories", "url_reputationstring",
    "url_reputation", "mediatype_header", "recv_byte", "sent_byte", "user_agent", "referer",
    "url_geolocation", "application_name", "currentruleset", "currentrule", "action_names",
    "block_id", "proxy_id", "ssl_certificate_cn", "ssl_certificate_sigmethod",
    "web_socket", "content_lenght",
];

const DELIMITER: &str = " :| ";

/// 트래픽 로그 레코드
#[derive(Debug, Clone)]
pub struct TrafficLogRecord {
    pub datetime: Option<DateTime<Local>>,
    pub username: Option<String>,
    pub client_ip: Option<String>,
    pub url_destination_ip: Option<String>,
    pub timeintransaction: Option<f64>,
    pub response_statuscode: Option<i32>,
    pub cache_status: Option<String>,
    pub comm_name: Option<String>,
    pub url_protocol: Option<String>,
    pub url_host: Option<String>,
    pub url_path: Option<String>,
    pub url_parametersstring: Option<String>,
    pub url_port: Option<i32>,
    pub url_categories: Option<String>,
    pub url_reputationstring: Option<String>,
    pub url_reputation: Option<i32>,
    pub mediatype_header: Option<String>,
    pub recv_byte: Option<i64>,
    pub sent_byte: Option<i64>,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub url_geolocation: Option<String>,
    pub application_name: Option<String>,
    pub currentruleset: Option<String>,
    pub currentrule: Option<String>,
    pub action_names: Option<String>,
    pub block_id: Option<String>,
    pub proxy_id: Option<i32>,
    pub ssl_certificate_cn: Option<String>,
    pub ssl_certificate_sigmethod: Option<String>,
    pub web_socket: Option<bool>,
    pub content_lenght: Option<i64>,
}

impl TrafficLogRecord {
    /// 로그 라인을 파싱하여 TrafficLogRecord 생성
    pub fn parse(line: &str) -> Result<Self> {
        let parts: Vec<&str> = line.trim_end_matches('\n').split(DELIMITER).collect();
        
        // 필드 수가 맞지 않으면 패딩 또는 자르기
        let mut parts = parts;
        while parts.len() < FIELDS.len() {
            parts.push("");
        }
        if parts.len() > FIELDS.len() {
            parts.truncate(FIELDS.len());
        }

        let mut record = TrafficLogRecord {
            datetime: None,
            username: None,
            client_ip: None,
            url_destination_ip: None,
            timeintransaction: None,
            response_statuscode: None,
            cache_status: None,
            comm_name: None,
            url_protocol: None,
            url_host: None,
            url_path: None,
            url_parametersstring: None,
            url_port: None,
            url_categories: None,
            url_reputationstring: None,
            url_reputation: None,
            mediatype_header: None,
            recv_byte: None,
            sent_byte: None,
            user_agent: None,
            referer: None,
            url_geolocation: None,
            application_name: None,
            currentruleset: None,
            currentrule: None,
            action_names: None,
            block_id: None,
            proxy_id: None,
            ssl_certificate_cn: None,
            ssl_certificate_sigmethod: None,
            web_socket: None,
            content_lenght: None,
        };

        for (i, field_name) in FIELDS.iter().enumerate() {
            if let Some(value) = parts.get(i) {
                let value = value.trim();
                if value.is_empty() {
                    continue;
                }

                match *field_name {
                    "datetime" => {
                        // 날짜 시간 파싱 시도
                        if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
                            record.datetime = Some(Local.from_local_datetime(&dt)
                                .single()
                                .unwrap_or_else(|| dt.and_utc().with_timezone(&Local)));
                        }
                    }
                    "username" => record.username = Some(value.to_string()),
                    "client_ip" => record.client_ip = Some(value.to_string()),
                    "url_destination_ip" => record.url_destination_ip = Some(value.to_string()),
                    "timeintransaction" => {
                        if let Ok(v) = value.parse::<f64>() {
                            record.timeintransaction = Some(v);
                        }
                    }
                    "response_statuscode" => {
                        if let Ok(v) = value.parse::<i32>() {
                            record.response_statuscode = Some(v);
                        }
                    }
                    "cache_status" => record.cache_status = Some(value.to_string()),
                    "comm_name" => record.comm_name = Some(value.to_string()),
                    "url_protocol" => record.url_protocol = Some(value.to_string()),
                    "url_host" => record.url_host = Some(value.to_string()),
                    "url_path" => record.url_path = Some(value.to_string()),
                    "url_parametersstring" => record.url_parametersstring = Some(value.to_string()),
                    "url_port" => {
                        if let Ok(v) = value.parse::<i32>() {
                            record.url_port = Some(v);
                        }
                    }
                    "url_categories" => record.url_categories = Some(value.to_string()),
                    "url_reputationstring" => record.url_reputationstring = Some(value.to_string()),
                    "url_reputation" => {
                        if let Ok(v) = value.parse::<i32>() {
                            record.url_reputation = Some(v);
                        }
                    }
                    "mediatype_header" => record.mediatype_header = Some(value.to_string()),
                    "recv_byte" => {
                        if let Ok(v) = value.parse::<i64>() {
                            record.recv_byte = Some(v);
                        }
                    }
                    "sent_byte" => {
                        if let Ok(v) = value.parse::<i64>() {
                            record.sent_byte = Some(v);
                        }
                    }
                    "user_agent" => record.user_agent = Some(value.to_string()),
                    "referer" => record.referer = Some(value.to_string()),
                    "url_geolocation" => record.url_geolocation = Some(value.to_string()),
                    "application_name" => record.application_name = Some(value.to_string()),
                    "currentruleset" => record.currentruleset = Some(value.to_string()),
                    "currentrule" => record.currentrule = Some(value.to_string()),
                    "action_names" => record.action_names = Some(value.to_string()),
                    "block_id" => record.block_id = Some(value.to_string()),
                    "proxy_id" => {
                        if let Ok(v) = value.parse::<i32>() {
                            record.proxy_id = Some(v);
                        }
                    }
                    "ssl_certificate_cn" => record.ssl_certificate_cn = Some(value.to_string()),
                    "ssl_certificate_sigmethod" => record.ssl_certificate_sigmethod = Some(value.to_string()),
                    "web_socket" => {
                        let v = value.to_lowercase();
                        record.web_socket = Some(v == "1" || v == "true" || v == "yes" || v == "y");
                    }
                    "content_lenght" => {
                        if let Ok(v) = value.parse::<i64>() {
                            record.content_lenght = Some(v);
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(record)
    }
}

/// TOP N 분석 결과
#[derive(Debug, Clone)]
pub struct TopNAnalysis {
    pub top_clients: Vec<TopClient>,
    pub top_hosts: Vec<TopHost>,
    pub top_urls: Vec<TopUrl>,
    pub total_records: usize,
    pub parsed_records: usize,
    pub unparsed_records: usize,
    pub total_recv_bytes: i64,
    pub total_sent_bytes: i64,
    pub blocked_count: usize,
    pub unique_clients: usize,
    pub unique_hosts: usize,
}

#[derive(Debug, Clone)]
pub struct TopClient {
    pub client_ip: String,
    pub request_count: usize,
    pub recv_bytes: i64,
    pub sent_bytes: i64,
}

#[derive(Debug, Clone)]
pub struct TopHost {
    pub host: String,
    pub request_count: usize,
    pub recv_bytes: i64,
    pub sent_bytes: i64,
}

#[derive(Debug, Clone)]
pub struct TopUrl {
    pub url: String,
    pub request_count: usize,
}

/// 트래픽 로그 분석기
pub struct TrafficLogAnalyzer {
    top_n: usize,
}

impl TrafficLogAnalyzer {
    pub fn new(top_n: usize) -> Self {
        Self { top_n }
    }

    /// 로그 라인들을 분석하여 TOP N 결과 반환
    pub fn analyze(&self, lines: &[String]) -> TopNAnalysis {
        let mut client_counter: HashMap<String, (usize, i64, i64)> = HashMap::new();
        let mut host_counter: HashMap<String, (usize, i64, i64)> = HashMap::new();
        let mut url_counter: HashMap<String, usize> = HashMap::new();
        let mut unique_clients = std::collections::HashSet::new();
        let mut unique_hosts = std::collections::HashSet::new();
        
        let mut total_recv_bytes = 0i64;
        let mut total_sent_bytes = 0i64;
        let mut blocked_count = 0usize;
        let mut parsed_records = 0usize;
        let mut unparsed_records = 0usize;

        for line in lines {
            if line.trim().is_empty() {
                continue;
            }

            match TrafficLogRecord::parse(line) {
                Ok(record) => {
                    parsed_records += 1;

                    let client_ip = record.client_ip.as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    let url_host = record.url_host.as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    let url_path = record.url_path.as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    let recv_bytes = record.recv_byte.unwrap_or(0);
                    let sent_bytes = record.sent_byte.unwrap_or(0);
                    let action_names = record.action_names.as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    let block_id = record.block_id.as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("");

                    // 클라이언트 통계
                    if !client_ip.is_empty() {
                        unique_clients.insert(client_ip.clone());
                        let entry = client_counter.entry(client_ip.clone()).or_insert((0, 0, 0));
                        entry.0 += 1;
                        entry.1 += recv_bytes;
                        entry.2 += sent_bytes;
                    }

                    // 호스트 통계
                    if !url_host.is_empty() {
                        unique_hosts.insert(url_host.clone());
                        let entry = host_counter.entry(url_host.clone()).or_insert((0, 0, 0));
                        entry.0 += 1;
                        entry.1 += recv_bytes;
                        entry.2 += sent_bytes;
                    }

                    // URL 통계
                    if !url_host.is_empty() {
                        let url = if !url_path.is_empty() {
                            format!("{}://{}{}", 
                                record.url_protocol.as_ref().unwrap_or(&"http".to_string()),
                                url_host,
                                url_path
                            )
                        } else {
                            format!("{}://{}", 
                                record.url_protocol.as_ref().unwrap_or(&"http".to_string()),
                                url_host
                            )
                        };
                        *url_counter.entry(url).or_insert(0) += 1;
                    }

                    // 전체 통계
                    total_recv_bytes += recv_bytes;
                    total_sent_bytes += sent_bytes;

                    // 차단 카운트
                    if !block_id.is_empty() || action_names.contains("block") || action_names.contains("Block") {
                        blocked_count += 1;
                    }
                }
                Err(_) => {
                    unparsed_records += 1;
                }
            }
        }

        // TOP N 정렬
        let mut top_clients: Vec<TopClient> = client_counter
            .into_iter()
            .map(|(ip, (count, recv, sent))| TopClient {
                client_ip: ip,
                request_count: count,
                recv_bytes: recv,
                sent_bytes: sent,
            })
            .collect();
        top_clients.sort_by(|a, b| b.request_count.cmp(&a.request_count));
        top_clients.truncate(self.top_n);

        let mut top_hosts: Vec<TopHost> = host_counter
            .into_iter()
            .map(|(host, (count, recv, sent))| TopHost {
                host,
                request_count: count,
                recv_bytes: recv,
                sent_bytes: sent,
            })
            .collect();
        top_hosts.sort_by(|a, b| b.request_count.cmp(&a.request_count));
        top_hosts.truncate(self.top_n);

        let mut top_urls: Vec<TopUrl> = url_counter
            .into_iter()
            .map(|(url, count)| TopUrl {
                url,
                request_count: count,
            })
            .collect();
        top_urls.sort_by(|a, b| b.request_count.cmp(&a.request_count));
        top_urls.truncate(self.top_n);

        TopNAnalysis {
            top_clients,
            top_hosts,
            top_urls,
            total_records: lines.len(),
            parsed_records,
            unparsed_records,
            total_recv_bytes,
            total_sent_bytes,
            blocked_count,
            unique_clients: unique_clients.len(),
            unique_hosts: unique_hosts.len(),
        }
    }
}
