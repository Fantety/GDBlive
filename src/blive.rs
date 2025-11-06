use godot::prelude::*;
use godot::classes::{Node, INode};
use md5::{Md5, Digest};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

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
    
    base: Base<Node>
}

#[godot_api]
impl INode for Blive{
    fn init(base: Base<Node>) -> Self{
        godot_print!("Blive: Initializing");
        
        Self { 
            code: GString::from(""),
            app_id: GString::from(""),
            access_key_id: GString::from(""),
            access_key_secret: GString::from(""),
            api_base_url: GString::from("https://live-open.biliapi.com"),
            base
        }
    }
    
    fn ready(&mut self) {
        godot_print!("Blive: Ready");
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

    /// 项目心跳 - 调用 /v2/app/heartbeat 接口（同步）
    /// 推荐每20秒调用一次
    #[func]
    fn heartbeat(&mut self, game_id: GString) {
        godot_print!("heartbeat 函数被调用");
        
        let body_json = format!(
            r#"{{"game_id":"{}"}}"#,
            game_id.to_string()
        );

        let headers = self.generate_headers_map(&body_json);
        let url = format!("{}/v2/app/heartbeat", self.api_base_url.to_string());

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
            &StringName::from("heartbeat_completed"),
            &[GString::from(&response_json).to_variant()]
        );
    }

    /// 项目批量心跳 - 调用 /v2/app/batchHeartbeat 接口（同步）
    /// game_ids 数量要小于200条
    #[func]
    fn batch_heartbeat(&mut self, game_ids: Array<GString>) {
        godot_print!("batch_heartbeat 函数被调用，数量: {}", game_ids.len());
        
        // 构建 game_ids 数组的 JSON
        let mut game_ids_vec = Vec::new();
        for i in 0..game_ids.len() {
            let game_id = game_ids.at(i);
            game_ids_vec.push(format!(r#""{}""#, game_id.to_string()));
        }
        let game_ids_json = format!("[{}]", game_ids_vec.join(","));
        
        let body_json = format!(
            r#"{{"game_ids":{}}}"#,
            game_ids_json
        );

        let headers = self.generate_headers_map(&body_json);
        let url = format!("{}/v2/app/batchHeartbeat", self.api_base_url.to_string());

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
            &StringName::from("batch_heartbeat_completed"),
            &[GString::from(&response_json).to_variant()]
        );
    }
}
