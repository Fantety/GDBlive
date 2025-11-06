# GDBLive - B站直播开放平台 Godot 扩展

这是一个用于 Godot 4 的 GDExtension，实现了 B站直播开放平台的 API 和 WebSocket 长连接功能。

## 功能特性

### HTTP API 接口

- ✅ `/v2/app/start` - 开启项目
- ✅ `/v2/app/end` - 关闭项目
- ✅ `/v2/app/heartbeat` - 项目心跳
- ✅ `/v2/app/batchHeartbeat` - 项目批量心跳

### WebSocket 长连接

- ✅ 自动鉴权（OP_AUTH）
- ✅ 自动心跳维持（每 20 秒）
- ✅ 协议包编码/解码
- ✅ Zlib 压缩数据自动解压
- ✅ 实时消息推送
- ✅ 线程安全的信号发送

### 支持的消息类型

- 弹幕消息 (`LIVE_OPEN_PLATFORM_DM`)
- 礼物消息 (`LIVE_OPEN_PLATFORM_SEND_GIFT`)
- 醒目留言 (`LIVE_OPEN_PLATFORM_SUPER_CHAT`)
- 上舰消息 (`LIVE_OPEN_PLATFORM_GUARD`)
- 点赞消息 (`LIVE_OPEN_PLATFORM_LIKE`)

## 编译

### Windows

```powershell
# 开发版本
.\build-dev.ps1

# 或使用 bat 文件
.\build-dev.bat

# 发布版本
.\build.ps1
# 或
.\build.bat
```

### 手动编译

```bash
cargo build --release
```

编译后的动态库会输出到 `demo/addons/gdblive/bin/` 目录。

## 使用方法

### 1. 基本设置

```gdscript
extends Node

var blive: Blive

func _ready():
    blive = Blive.new()
    add_child(blive)
    
    # 配置
    blive.code = "your_code"
    blive.app_id = "your_app_id"
    blive.access_key_id = "your_access_key_id"
    blive.access_key_secret = "your_access_key_secret"
```

### 2. HTTP API 调用

```gdscript
# 连接信号
blive.start_completed.connect(_on_start_completed)

# 启动项目
blive.start()

func _on_start_completed(response_json: String):
    var json = JSON.new()
    json.parse(response_json)
    var data = json.data
    print(data)
```

### 3. WebSocket 连接

```gdscript
# 连接 WebSocket 信号
blive.ws_connected.connect(_on_ws_connected)
blive.ws_message_received.connect(_on_ws_message_received)

# 从 start 接口获取 WebSocket 信息后启动
func _on_start_completed(response_json: String):
    var json = JSON.new()
    json.parse(response_json)
    var data = json.data
    
    if data["code"] == 0:
        var ws_info = data["data"]["websocket_info"]
        var ws_url = ws_info["wss_link"][0]
        var auth_body = ws_info["auth_body"]
        
        # 启动 WebSocket
        blive.start_websocket(ws_url, auth_body)

func _on_ws_message_received(cmd: String, data_json: String):
    print("收到消息: ", cmd)
    print("内容: ", data_json)
```

### 4. 完整示例

查看 `demo/websocket_example.gd` 获取完整的使用示例。

## API 文档

### 方法

#### `start()`
开启项目，调用 `/v2/app/start` 接口。

#### `end(game_id: String)`
关闭项目，调用 `/v2/app/end` 接口。

#### `heartbeat(game_id: String)`
发送项目心跳，调用 `/v2/app/heartbeat` 接口。推荐每 20 秒调用一次。

#### `batch_heartbeat(game_ids: Array[String])`
批量发送项目心跳，调用 `/v2/app/batchHeartbeat` 接口。game_ids 数量要小于 200 条。

#### `start_websocket(ws_url: String, auth_body: String)`
启动 WebSocket 连接。
- `ws_url`: WebSocket 服务器地址
- `auth_body`: 鉴权字符串

#### `stop_websocket()`
停止 WebSocket 连接。

### 信号

#### HTTP API 信号

- `start_completed(response_json: String)` - start 请求完成
- `end_completed(response_json: String)` - end 请求完成
- `heartbeat_completed(response_json: String)` - heartbeat 请求完成
- `batch_heartbeat_completed(response_json: String)` - batch_heartbeat 请求完成

#### WebSocket 信号

- `ws_connected()` - WebSocket 连接成功
- `ws_disconnected()` - WebSocket 连接断开
- `ws_message_received(cmd: String, data_json: String)` - 收到消息
- `ws_error(error_msg: String)` - 发生错误

## 技术细节

### 协议实现

WebSocket 协议基于 B站直播开放平台的应用层协议：

- 大端对齐的二进制协议
- 支持 Zlib 压缩（Version=2）
- 自动心跳维持（20 秒间隔）
- 线程安全的消息处理

### 线程模型

WebSocket 连接在独立的系统线程中运行，使用 Tokio runtime 处理异步操作：

- 使用 `std::thread::spawn` 创建独立线程
- 在线程中创建 Tokio runtime 并使用 `block_on` 运行异步任务
- 通过 `mpsc` 通道与主线程通信
- 注意：子线程中的 `godot_print!` 不会输出到 Godot 控制台

### 依赖

- `godot` - Godot 4 GDExtension 绑定
- `reqwest` - HTTP 客户端
- `tungstenite` - WebSocket 客户端
- `flate2` - Zlib 解压
- `byteorder` - 字节序处理
- `md5`, `hmac`, `sha2` - 签名生成

## 许可证

MIT License

## 相关链接

- [B站直播开放平台文档](https://open-live.bilibili.com/document/)
- [Godot Engine](https://godotengine.org/)
- [godot-rust](https://godot-rust.github.io/)
