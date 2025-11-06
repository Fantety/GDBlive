use godot::prelude::*;
use godot::classes::{Node, INode};
use md5::{Md5, Digest};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::{Arc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures::{StreamExt, SinkExt};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;
use flate2::read::ZlibDecoder;
use std::io::Read;
use tokio::sync::mpsc;
use std::time::Duration;

use crate::runtime::RuntimeManager;

// WebSocket 操作码常量
const OP_HEARTBEAT: u32 = 2;
const OP_HEARTBEAT_REPLY: u32 = 3;
const OP_SEND_SMS_REPLY: u32 = 5;
const OP_AUTH: u32 = 7;
const OP_AUTH_REPLY: u32 = 8;

// 协议版本
const VERSION_NORMAL: u16 = 0;
const VERSION_ZLIB: u16 = 2;

// Header 长度固定为 16 字节
const HEADER_LENGTH: u16 = 16;

#[derive(GodotClass)]
#[class(base=Node)]
struct Blive {
    #[export]
    code: GString,
    #[export]
    app_id: GString,
    #[export]
    access_key_id: GString,
    #[export]
    access_key_secret: GString,
    #[export]
    api_base_url: GString,
    
    runtime: Option<Arc<RuntimeManager>>,
    ws_running: Arc<Mutex<bool>>,
    ws_message_rx: Option<Arc<Mutex<mpsc::UnboundedReceiver<WsMessage>>>>,
    
    // API 心跳相关
    heartbeat_running: Arc<Mutex<bool>>,
    current_game_id: Arc<Mutex<Option<String>>>,
    
    // 批量心跳相关
    batch_heartbeat_running: Arc<Mutex<bool>>,
    current_game_ids: Arc<Mutex<Vec<String>>>,
    
    base: Base<Node>
}

// WebSocket 消息类型
enum WsMessage {
    Connected,
    Disconnected,
    Message { cmd: String, data: String },
    Error(String),
    Debug(String), // 调试日志
}

#[godot_api]
impl INode for Blive{
    fn init(base: Base<Node>) -> Self{
        godot_print!("Blive: Initializing");
        
        // 创建 Tokio runtime
        let runtime_manager = RuntimeManager::new();
        
        Self { 
            code: GString::from(""),
            app_id: GString::from(""),
            access_key_id: GString::from(""),
            access_key_secret: GString::from(""),
            api_base_url: GString::from("https://live-open.biliapi.com"),
            runtime: Some(Arc::new(runtime_manager)),
            ws_running: Arc::new(Mutex::new(false)),
            ws_message_rx: None,
            heartbeat_running: Arc::new(Mutex::new(false)),
            current_game_id: Arc::new(Mutex::new(None)),
            batch_heartbeat_running: Arc::new(Mutex::new(false)),
            current_game_ids: Arc::new(Mutex::new(Vec::new())),
            base
        }
    }
    
    fn ready(&mut self) {
        godot_print!("Blive: Ready");
        // 启用 process 以处理 WebSocket 消息
        self.base_mut().set_process(true);
    }
    
    fn process(&mut self, _delta: f64) {
        // 检查 WebSocket 消息
        let message_opt = if let Some(ref rx_arc) = self.ws_message_rx {
            if let Ok(mut rx) = rx_arc.lock() {
                rx.try_recv().ok()
            } else {
                None
            }
        } else {
            None
        };
        
        // 处理消息
        if let Some(message) = message_opt {
            match message {
                WsMessage::Connected => {
                    self.base_mut().emit_signal("ws_connected", &[]);
                },
                WsMessage::Disconnected => {
                    self.base_mut().emit_signal("ws_disconnected", &[]);
                    self.ws_message_rx = None;
                },
                WsMessage::Message { cmd, data } => {
                    self.base_mut().emit_signal(
                        "ws_message_received",
                        &[GString::from(&cmd).to_variant(), GString::from(&data).to_variant()]
                    );
                },
                WsMessage::Error(error) => {
                    self.base_mut().emit_signal(
                        "ws_error",
                        &[GString::from(&error).to_variant()]
                    );
                },
                WsMessage::Debug(debug) => {
                    self.base_mut().emit_signal(
                        "ws_debug",
                        &[GString::from(&debug).to_variant()]
                    );
                }
            }
        }
    }
}

#[godot_api]
impl Blive{
    /// 信号：start 请求完成
    #[signal]
    fn start_completed(response_json: GString);

    /// 信号：end 请求完成
    #[signal]
    fn end_completed(response_json: GString);
    
    /// 信号：heartbeat 请求完成
    #[signal]
    fn heartbeat_completed(response_json: GString);
    
    /// 信号：batch_heartbeat 请求完成
    #[signal]
    fn batch_heartbeat_completed(response_json: GString);
    
    /// 信号：WebSocket 连接成功
    #[signal]
    fn ws_connected();
    
    /// 信号：WebSocket 连接断开
    #[signal]
    fn ws_disconnected();
    
    /// 信号：WebSocket 收到消息
    #[signal]
    fn ws_message_received(cmd: GString, data_json: GString);
    
    /// 信号：WebSocket 发生错误
    #[signal]
    fn ws_error(error_msg: GString);
    
    /// 信号：WebSocket 调试日志
    #[signal]
    fn ws_debug(debug_msg: GString);
    
    /// 信号：心跳调试日志
    #[signal]
    fn heartbeat_debug(debug_msg: GString);
    
    /// 生成请求体的MD5编码
    fn generate_content_md5(body: &str) -> String {
        let mut hasher = Md5::new();
        hasher.update(body.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// 生成签名随机数
    fn generate_nonce() -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{}", timestamp)
    }

    /// 生成Unix时间戳（秒）
    fn generate_timestamp() -> String {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string()
    }

    /// 生成签名
    fn generate_signature(headers: &BTreeMap<String, String>, access_key_secret: &str) -> String {
        // 按字典排序拼接所有x-bili-前缀的header
        // 格式: key:value\n
        let mut sign_parts = Vec::new();
        for (key, value) in headers.iter() {
            if key.starts_with("x-bili-") {
                sign_parts.push(format!("{}:{}", key, value));
            }
        }
        
        // 用换行符连接，注意最后一行不加换行符
        let sign_string = sign_parts.join("\n");
        
        godot_print!("待签名字符串:\n{}", &sign_string);

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(access_key_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(sign_string.as_bytes());
        let result = mac.finalize();
        let code_bytes = result.into_bytes();
        
        let signature = hex::encode(code_bytes);
        godot_print!("生成的签名: {}", &signature);
        
        signature
    }

    /// 生成完整的请求头
    fn generate_headers_map(&self, body: &str) -> BTreeMap<String, String> {
        let access_key_id = self.access_key_id.to_string();
        let access_key_secret = self.access_key_secret.to_string();

        let content_md5 = Self::generate_content_md5(body);
        let timestamp = Self::generate_timestamp();
        let nonce = Self::generate_nonce();

        let mut bili_headers = BTreeMap::new();
        bili_headers.insert("x-bili-accesskeyid".to_string(), access_key_id.clone());
        bili_headers.insert("x-bili-content-md5".to_string(), content_md5.clone());
        bili_headers.insert("x-bili-signature-method".to_string(), "HMAC-SHA256".to_string());
        bili_headers.insert("x-bili-signature-nonce".to_string(), nonce.clone());
        bili_headers.insert("x-bili-signature-version".to_string(), "1.0".to_string());
        bili_headers.insert("x-bili-timestamp".to_string(), timestamp.clone());

        let signature = Self::generate_signature(&bili_headers, &access_key_secret);

        let mut headers = BTreeMap::new();
        headers.insert("Accept".to_string(), "application/json".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("x-bili-accesskeyid".to_string(), access_key_id);
        headers.insert("x-bili-content-md5".to_string(), content_md5);
        headers.insert("x-bili-signature-method".to_string(), "HMAC-SHA256".to_string());
        headers.insert("x-bili-signature-nonce".to_string(), nonce);
        headers.insert("x-bili-signature-version".to_string(), "1.0".to_string());
        headers.insert("x-bili-timestamp".to_string(), timestamp);
        headers.insert("Authorization".to_string(), signature);

        headers
    }

    /// 开启项目 - 调用 /v2/app/start 接口（同步）
    #[func]
    fn start(&mut self) {
        godot_print!("start 函数被调用");
        
        let body_json = format!(
            r#"{{"code":"{}","app_id":{}}}"#,
            self.code.to_string(),
            self.app_id.to_string()
        );

        let headers = self.generate_headers_map(&body_json);
        let url = format!("{}/v2/app/start", self.api_base_url.to_string());

        godot_print!("发送 HTTP 请求到: {}", &url);
        
        let client = reqwest::blocking::Client::new();
        let mut request_builder = client.post(&url).body(body_json);

        for (key, value) in headers.iter() {
            request_builder = request_builder.header(key, value);
        }

        let response_json = match request_builder.send() {
            Ok(resp) => {
                godot_print!("收到响应");
                match resp.text() {
                    Ok(text) => text,
                    Err(e) => format!(r#"{{"code":-1,"message":"响应读取失败: {}"}}"#, e),
                }
            },
            Err(e) => format!(r#"{{"code":-1,"message":"请求发送失败: {}"}}"#, e),
        };

        godot_print!("请求完成");
        
        self.base_mut().emit_signal(
            &StringName::from("start_completed"),
            &[GString::from(&response_json).to_variant()]
        );
    }

    /// 关闭项目 - 调用 /v2/app/end 接口（同步）
    #[func]
    fn end(&mut self, game_id: GString) {
        godot_print!("end 函数被调用");
        
        let body_json = format!(
            r#"{{"app_id":{},"game_id":"{}"}}"#,
            self.app_id.to_string(),
            game_id.to_string()
        );

        let headers = self.generate_headers_map(&body_json);
        let url = format!("{}/v2/app/end", self.api_base_url.to_string());

        let client = reqwest::blocking::Client::new();
        let mut request_builder = client.post(&url).body(body_json);

        for (key, value) in headers.iter() {
            request_builder = request_builder.header(key, value);
        }

        let response_json = match request_builder.send() {
            Ok(resp) => match resp.text() {
                Ok(text) => text,
                Err(e) => format!(r#"{{"code":-1,"message":"响应读取失败: {}"}}"#, e),
            },
            Err(e) => format!(r#"{{"code":-1,"message":"请求发送失败: {}"}}"#, e),
        };

        self.base_mut().emit_signal(
            &StringName::from("end_completed"),
            &[GString::from(&response_json).to_variant()]
        );
    }

    /// 启动项目心跳 - 自动每20秒调用 /v2/app/heartbeat 接口
    #[func]
    fn start_heartbeat(&mut self, game_id: GString) {
        godot_print!("start_heartbeat 函数被调用");
        
        // 检查是否已经在运行
        {
            let mut running = self.heartbeat_running.lock().unwrap();
            if *running {
                godot_print!("心跳已经在运行中");
                return;
            }
            *running = true;
        }
        
        // 保存 game_id
        {
            let mut gid = self.current_game_id.lock().unwrap();
            *gid = Some(game_id.to_string());
        }
        
        let heartbeat_running = Arc::clone(&self.heartbeat_running);
        let current_game_id = Arc::clone(&self.current_game_id);
        
        // 获取需要的配置
        let access_key_id = self.access_key_id.to_string();
        let access_key_secret = self.access_key_secret.to_string();
        let api_base_url = self.api_base_url.to_string();
        
        // 获取实例 ID 用于发送信号
        let instance_id = self.base().instance_id();
        
        godot_print!("启动心跳线程...");
        
        // 在独立线程中运行心跳
        std::thread::Builder::new()
            .name("heartbeat-thread".to_string())
            .spawn(move || {
                // 发送调试信息的辅助函数
                let send_debug = |msg: &str| {
                    if let Ok(mut node) = Gd::<Blive>::try_from_instance_id(instance_id) {
                        let method = StringName::from("emit_signal");
                        let signal_name = StringName::from("heartbeat_debug");
                        let args = vec![
                            signal_name.to_variant(),
                            GString::from(msg).to_variant()
                        ];
                        node.call_deferred(&method, &args);
                    }
                };
                
                send_debug("心跳线程已启动");
                
                while *heartbeat_running.lock().unwrap() {
                    // 获取当前 game_id
                    let gid = {
                        let gid_lock = current_game_id.lock().unwrap();
                        gid_lock.clone()
                    };
                    
                    if let Some(game_id) = gid {
                        send_debug(&format!("准备发送心跳: game_id={}", game_id));
                        
                        // 构建请求
                        let body_json = format!(r#"{{"game_id":"{}"}}"#, game_id);
                        
                        // 生成请求头
                        let headers = Self::generate_headers_for_heartbeat(
                            &body_json,
                            &access_key_id,
                            &access_key_secret
                        );
                        
                        let url = format!("{}/v2/app/heartbeat", api_base_url);
                        
                        // 发送请求
                        let client = reqwest::blocking::Client::new();
                        let mut request_builder = client.post(&url).body(body_json);
                        
                        for (key, value) in headers.iter() {
                            request_builder = request_builder.header(key, value);
                        }
                        
                        let response_json = match request_builder.send() {
                            Ok(resp) => {
                                send_debug("心跳请求已发送");
                                match resp.text() {
                                    Ok(text) => {
                                        send_debug("收到心跳响应");
                                        text
                                    },
                                    Err(e) => {
                                        let err_msg = format!("响应读取失败: {}", e);
                                        send_debug(&err_msg);
                                        format!(r#"{{"code":-1,"message":"{}"}}"#, err_msg)
                                    }
                                }
                            },
                            Err(e) => {
                                let err_msg = format!("请求发送失败: {}", e);
                                send_debug(&err_msg);
                                format!(r#"{{"code":-1,"message":"{}"}}"#, err_msg)
                            }
                        };
                        
                        // 发送信号
                        if let Ok(mut node) = Gd::<Blive>::try_from_instance_id(instance_id) {
                            let method = StringName::from("emit_signal");
                            let signal_name = StringName::from("heartbeat_completed");
                            let args = vec![
                                signal_name.to_variant(),
                                GString::from(&response_json).to_variant()
                            ];
                            node.call_deferred(&method, &args);
                        }
                    }
                    
                    // 等待 20 秒
                    send_debug("等待 20 秒后发送下一次心跳");
                    std::thread::sleep(std::time::Duration::from_secs(20));
                }
                
                send_debug("心跳线程结束");
            })
            .expect("Failed to spawn heartbeat thread");
        
        godot_print!("心跳线程已启动");
    }
    
    /// 停止项目心跳
    #[func]
    fn stop_heartbeat(&mut self) {
        godot_print!("stop_heartbeat 函数被调用");
        let mut running = self.heartbeat_running.lock().unwrap();
        *running = false;
    }
    
    /// 生成心跳请求的请求头（静态方法，用于线程中）
    fn generate_headers_for_heartbeat(
        body: &str,
        access_key_id: &str,
        access_key_secret: &str
    ) -> BTreeMap<String, String> {
        let content_md5 = Self::generate_content_md5(body);
        let timestamp = Self::generate_timestamp();
        let nonce = Self::generate_nonce();

        let mut bili_headers = BTreeMap::new();
        bili_headers.insert("x-bili-accesskeyid".to_string(), access_key_id.to_string());
        bili_headers.insert("x-bili-content-md5".to_string(), content_md5.clone());
        bili_headers.insert("x-bili-signature-method".to_string(), "HMAC-SHA256".to_string());
        bili_headers.insert("x-bili-signature-nonce".to_string(), nonce.clone());
        bili_headers.insert("x-bili-signature-version".to_string(), "1.0".to_string());
        bili_headers.insert("x-bili-timestamp".to_string(), timestamp.clone());

        let signature = Self::generate_signature(&bili_headers, access_key_secret);

        let mut headers = BTreeMap::new();
        headers.insert("Accept".to_string(), "application/json".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("x-bili-accesskeyid".to_string(), access_key_id.to_string());
        headers.insert("x-bili-content-md5".to_string(), content_md5);
        headers.insert("x-bili-signature-method".to_string(), "HMAC-SHA256".to_string());
        headers.insert("x-bili-signature-nonce".to_string(), nonce);
        headers.insert("x-bili-signature-version".to_string(), "1.0".to_string());
        headers.insert("x-bili-timestamp".to_string(), timestamp);
        headers.insert("Authorization".to_string(), signature);

        headers
    }

    /// 启动项目批量心跳 - 自动每20秒调用 /v2/app/batchHeartbeat 接口
    /// game_ids 数量要小于200条
    #[func]
    fn start_batch_heartbeat(&mut self, game_ids: Array<GString>) {
        godot_print!("start_batch_heartbeat 函数被调用，数量: {}", game_ids.len());
        
        if game_ids.len() == 0 {
            godot_print!("错误：game_ids 为空");
            return;
        }
        
        if game_ids.len() >= 200 {
            godot_print!("警告：game_ids 数量超过 200，可能会失败");
        }
        
        // 检查是否已经在运行
        {
            let mut running = self.batch_heartbeat_running.lock().unwrap();
            if *running {
                godot_print!("批量心跳已经在运行中");
                return;
            }
            *running = true;
        }
        
        // 保存 game_ids
        {
            let mut gids = self.current_game_ids.lock().unwrap();
            gids.clear();
            for i in 0..game_ids.len() {
                gids.push(game_ids.at(i).to_string());
            }
        }
        
        let batch_heartbeat_running = Arc::clone(&self.batch_heartbeat_running);
        let current_game_ids = Arc::clone(&self.current_game_ids);
        
        // 获取需要的配置
        let access_key_id = self.access_key_id.to_string();
        let access_key_secret = self.access_key_secret.to_string();
        let api_base_url = self.api_base_url.to_string();
        
        // 获取实例 ID 用于发送信号
        let instance_id = self.base().instance_id();
        
        godot_print!("启动批量心跳线程...");
        
        // 在独立线程中运行批量心跳
        std::thread::Builder::new()
            .name("batch-heartbeat-thread".to_string())
            .spawn(move || {
                // 发送调试信息的辅助函数
                let send_debug = |msg: &str| {
                    if let Ok(mut node) = Gd::<Blive>::try_from_instance_id(instance_id) {
                        let method = StringName::from("emit_signal");
                        let signal_name = StringName::from("heartbeat_debug");
                        let args = vec![
                            signal_name.to_variant(),
                            GString::from(msg).to_variant()
                        ];
                        node.call_deferred(&method, &args);
                    }
                };
                
                send_debug("批量心跳线程已启动");
                
                while *batch_heartbeat_running.lock().unwrap() {
                    // 获取当前 game_ids
                    let gids = {
                        let gids_lock = current_game_ids.lock().unwrap();
                        gids_lock.clone()
                    };
                    
                    if !gids.is_empty() {
                        send_debug(&format!("准备发送批量心跳: {} 个场次", gids.len()));
                        
                        // 构建 game_ids 数组的 JSON
                        let game_ids_json: Vec<String> = gids.iter()
                            .map(|id| format!(r#""{}""#, id))
                            .collect();
                        let game_ids_json_str = format!("[{}]", game_ids_json.join(","));
                        
                        let body_json = format!(r#"{{"game_ids":{}}}"#, game_ids_json_str);
                        
                        // 生成请求头
                        let headers = Self::generate_headers_for_heartbeat(
                            &body_json,
                            &access_key_id,
                            &access_key_secret
                        );
                        
                        let url = format!("{}/v2/app/batchHeartbeat", api_base_url);
                        
                        // 发送请求
                        let client = reqwest::blocking::Client::new();
                        let mut request_builder = client.post(&url).body(body_json);
                        
                        for (key, value) in headers.iter() {
                            request_builder = request_builder.header(key, value);
                        }
                        
                        let response_json = match request_builder.send() {
                            Ok(resp) => {
                                send_debug("批量心跳请求已发送");
                                match resp.text() {
                                    Ok(text) => {
                                        send_debug("收到批量心跳响应");
                                        text
                                    },
                                    Err(e) => {
                                        let err_msg = format!("响应读取失败: {}", e);
                                        send_debug(&err_msg);
                                        format!(r#"{{"code":-1,"message":"{}"}}"#, err_msg)
                                    }
                                }
                            },
                            Err(e) => {
                                let err_msg = format!("请求发送失败: {}", e);
                                send_debug(&err_msg);
                                format!(r#"{{"code":-1,"message":"{}"}}"#, err_msg)
                            }
                        };
                        
                        // 发送信号
                        if let Ok(mut node) = Gd::<Blive>::try_from_instance_id(instance_id) {
                            let method = StringName::from("emit_signal");
                            let signal_name = StringName::from("batch_heartbeat_completed");
                            let args = vec![
                                signal_name.to_variant(),
                                GString::from(&response_json).to_variant()
                            ];
                            node.call_deferred(&method, &args);
                        }
                    } else {
                        send_debug("game_ids 为空，跳过本次心跳");
                    }
                    
                    // 等待 20 秒
                    send_debug("等待 20 秒后发送下一次批量心跳");
                    std::thread::sleep(std::time::Duration::from_secs(20));
                }
                
                send_debug("批量心跳线程结束");
            })
            .expect("Failed to spawn batch heartbeat thread");
        
        godot_print!("批量心跳线程已启动");
    }
    
    /// 停止项目批量心跳
    #[func]
    fn stop_batch_heartbeat(&mut self) {
        godot_print!("stop_batch_heartbeat 函数被调用");
        let mut running = self.batch_heartbeat_running.lock().unwrap();
        *running = false;
    }
    
    /// 更新批量心跳的 game_ids
    #[func]
    fn update_batch_heartbeat_game_ids(&mut self, game_ids: Array<GString>) {
        godot_print!("update_batch_heartbeat_game_ids 函数被调用，数量: {}", game_ids.len());
        
        let mut gids = self.current_game_ids.lock().unwrap();
        gids.clear();
        for i in 0..game_ids.len() {
            gids.push(game_ids.at(i).to_string());
        }
        
        godot_print!("批量心跳 game_ids 已更新");
    }

    /// 编码协议包
    fn encode_packet(operation: u32, body: &[u8]) -> Vec<u8> {
        let packet_length = (HEADER_LENGTH as u32) + (body.len() as u32);
        let mut packet = Vec::new();
        
        packet.write_u32::<BigEndian>(packet_length).unwrap();
        packet.write_u16::<BigEndian>(HEADER_LENGTH).unwrap();
        packet.write_u16::<BigEndian>(VERSION_NORMAL).unwrap();
        packet.write_u32::<BigEndian>(operation).unwrap();
        packet.write_u32::<BigEndian>(1).unwrap(); // Sequence ID
        packet.extend_from_slice(body);
        
        packet
    }

    /// 解码协议包
    fn decode_packet(data: &[u8]) -> Result<Vec<(u32, Vec<u8>)>, String> {
        let mut cursor = Cursor::new(data);
        let mut packets = Vec::new();
        
        while cursor.position() < data.len() as u64 {
            let packet_length = cursor.read_u32::<BigEndian>()
                .map_err(|e| format!("读取包长度失败: {}", e))?;
            let header_length = cursor.read_u16::<BigEndian>()
                .map_err(|e| format!("读取头长度失败: {}", e))?;
            let version = cursor.read_u16::<BigEndian>()
                .map_err(|e| format!("读取版本失败: {}", e))?;
            let operation = cursor.read_u32::<BigEndian>()
                .map_err(|e| format!("读取操作码失败: {}", e))?;
            let _sequence = cursor.read_u32::<BigEndian>()
                .map_err(|e| format!("读取序列号失败: {}", e))?;
            
            let body_length = packet_length - (header_length as u32);
            let mut body = vec![0u8; body_length as usize];
            cursor.read_exact(&mut body)
                .map_err(|e| format!("读取包体失败: {}", e))?;
            
            // 如果是压缩包，解压后递归解析
            if version == VERSION_ZLIB {
                let mut decoder = ZlibDecoder::new(&body[..]);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)
                    .map_err(|e| format!("解压失败: {}", e))?;
                
                let inner_packets = Self::decode_packet(&decompressed)?;
                packets.extend(inner_packets);
            } else {
                packets.push((operation, body));
            }
        }
        
        Ok(packets)
    }

    /// 启动 WebSocket 连接
    /// ws_url: WebSocket 服务器地址（从 start 接口获取）
    /// auth_body: 鉴权字符串（从 start 接口获取）
    #[func]
    fn start_websocket(&mut self, ws_url: GString, auth_body: GString) {
        godot_print!("start_websocket 函数被调用");
        
        // 检查是否已经在运行
        {
            let mut running = self.ws_running.lock().unwrap();
            if *running {
                godot_print!("WebSocket 已经在运行中");
                return;
            }
            *running = true;
        }
        
        let ws_url_str = ws_url.to_string();
        let auth_body_str = auth_body.to_string();
        let ws_running = Arc::clone(&self.ws_running);
        
        // 创建消息通道
        let (tx, rx) = mpsc::unbounded_channel();
        self.ws_message_rx = Some(Arc::new(Mutex::new(rx)));
        
        godot_print!("准备启动 WebSocket 任务...");
        godot_print!("WS URL: {}", ws_url_str);
        godot_print!("Auth body 长度: {}", auth_body_str.len());
        
        // 使用 std::thread 启动一个新线程，在其中运行 Tokio runtime
        godot_print!("启动独立线程...");
        
        let spawn_result = std::thread::Builder::new()
            .name("websocket-thread".to_string())
            .spawn(move || {
            let _ = tx.send(WsMessage::Debug("线程已启动".to_string()));
            
            // 在这个线程中创建一个新的 Tokio runtime
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => {
                    let _ = tx.send(WsMessage::Debug("Tokio runtime 创建成功".to_string()));
                    rt
                },
                Err(e) => {
                    let _ = tx.send(WsMessage::Error(format!("创建 runtime 失败: {}", e)));
                    return;
                }
            };
            
            rt.block_on(async move {
            let _ = tx.send(WsMessage::Debug(format!("开始连接 WebSocket: {}", ws_url_str)));
            
            // 连接 WebSocket
            let ws_stream = match connect_async(&ws_url_str).await {
                Ok((stream, _)) => {
                    let _ = tx.send(WsMessage::Debug("WebSocket 连接成功".to_string()));
                    stream
                },
                Err(e) => {
                    let _ = tx.send(WsMessage::Error(format!("连接失败: {}", e)));
                    *ws_running.lock().unwrap() = false;
                    return;
                }
            };
            
            let _ = tx.send(WsMessage::Connected);
            
            let (mut write, mut read) = ws_stream.split();
            
            // 发送鉴权包
            let _ = tx.send(WsMessage::Debug("准备发送鉴权包".to_string()));
            let auth_packet = Self::encode_packet(OP_AUTH, auth_body_str.as_bytes());
            if let Err(e) = write.send(Message::Binary(auth_packet)).await {
                let _ = tx.send(WsMessage::Error(format!("鉴权失败: {}", e)));
                *ws_running.lock().unwrap() = false;
                return;
            }
            let _ = tx.send(WsMessage::Debug("鉴权包已发送".to_string()));
            
            // 启动心跳任务
            let _ = tx.send(WsMessage::Debug("启动心跳任务".to_string()));
            let ws_running_heartbeat = Arc::clone(&ws_running);
            let tx_heartbeat = tx.clone();
            tokio::spawn(async move {
                let _ = tx_heartbeat.send(WsMessage::Debug("心跳任务已启动".to_string()));
                let mut interval = tokio::time::interval(Duration::from_secs(20));
                
                loop {
                    interval.tick().await;
                    
                    if !*ws_running_heartbeat.lock().unwrap() {
                        break;
                    }
                    
                    let heartbeat_packet = Self::encode_packet(OP_HEARTBEAT, &[]);
                    if let Err(e) = write.send(Message::Binary(heartbeat_packet)).await {
                        let _ = tx_heartbeat.send(WsMessage::Error(format!("心跳失败: {}", e)));
                        break;
                    }
                    let _ = tx_heartbeat.send(WsMessage::Debug("心跳包已发送".to_string()));
                }
                let _ = tx_heartbeat.send(WsMessage::Debug("心跳任务结束".to_string()));
            });
            
            // 接收消息循环
            let _ = tx.send(WsMessage::Debug("开始接收消息循环".to_string()));
            while let Some(msg_result) = read.next().await {
                if !*ws_running.lock().unwrap() {
                    break;
                }
                
                match msg_result {
                    Ok(Message::Binary(data)) => {
                        match Self::decode_packet(&data) {
                            Ok(packets) => {
                                for (operation, body) in packets {
                                    match operation {
                                        OP_AUTH_REPLY => {
                                            let _ = tx.send(WsMessage::Debug("收到鉴权回复".to_string()));
                                        },
                                        OP_HEARTBEAT_REPLY => {
                                            let _ = tx.send(WsMessage::Debug("收到心跳回复".to_string()));
                                        },
                                        OP_SEND_SMS_REPLY => {
                                            if let Ok(text) = String::from_utf8(body) {
                                                // 解析 JSON 获取 cmd
                                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                                    let cmd = json.get("cmd")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("UNKNOWN")
                                                        .to_string();
                                                    
                                                    let _ = tx.send(WsMessage::Debug(format!("收到消息: {}", cmd)));
                                                    let _ = tx.send(WsMessage::Message {
                                                        cmd,
                                                        data: text,
                                                    });
                                                }
                                            }
                                        },
                                        _ => {
                                            let _ = tx.send(WsMessage::Debug(format!("收到未知操作码: {}", operation)));
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                let _ = tx.send(WsMessage::Debug(format!("解码包失败: {}", e)));
                            }
                        }
                    },
                    Ok(Message::Close(_)) => {
                        let _ = tx.send(WsMessage::Debug("收到关闭消息".to_string()));
                        break;
                    },
                    Err(e) => {
                        let _ = tx.send(WsMessage::Error(format!("接收失败: {}", e)));
                        break;
                    },
                    _ => {}
                }
            }
            
            let _ = tx.send(WsMessage::Debug("消息循环结束".to_string()));
            let _ = tx.send(WsMessage::Disconnected);
            *ws_running.lock().unwrap() = false;
            });
        });
        
        match spawn_result {
            Ok(_) => godot_print!("线程创建成功"),
            Err(e) => godot_print!("线程创建失败: {}", e),
        }
    }

    /// 停止 WebSocket 连接
    #[func]
    fn stop_websocket(&mut self) {
        godot_print!("stop_websocket 函数被调用");
        let mut running = self.ws_running.lock().unwrap();
        *running = false;
    }

}
