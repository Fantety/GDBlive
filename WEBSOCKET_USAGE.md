# B站直播开放平台 WebSocket 使用说明

## 功能概述

已实现 B站直播开放平台的 WebSocket 长连接功能，用于实时接收弹幕和其他互动消息。

## 协议说明

### 数据包格式（大端对齐）

- **Packet Length** (4 bytes): 整个包的长度，包含 Header
- **Header Length** (2 bytes): Header 长度，固定为 16
- **Version** (2 bytes): 
  - 0: Body 是实际数据
  - 2: Body 是 zlib 压缩数据
- **Operation** (4 bytes): 消息类型
  - 2: OP_HEARTBEAT - 客户端心跳包
  - 3: OP_HEARTBEAT_REPLY - 服务器心跳回复
  - 5: OP_SEND_SMS_REPLY - 服务器推送的消息
  - 7: OP_AUTH - 客户端鉴权包
  - 8: OP_AUTH_REPLY - 服务器鉴权回复
- **Sequence ID** (4 bytes): 保留字段
- **Body**: 消息体（JSON 格式）

## GDScript 使用示例

```gdscript
extends Node

var blive: Blive

func _ready():
    # 创建 Blive 节点
    blive = Blive.new()
    add_child(blive)
    
    # 设置配置
    blive.code = "your_code"
    blive.app_id = "your_app_id"
    blive.access_key_id = "your_access_key_id"
    blive.access_key_secret = "your_access_key_secret"
    
    # 连接信号
    blive.start_completed.connect(_on_start_completed)
    blive.ws_connected.connect(_on_ws_connected)
    blive.ws_disconnected.connect(_on_ws_disconnected)
    blive.ws_message_received.connect(_on_ws_message_received)
    blive.ws_error.connect(_on_ws_error)
    
    # 启动项目
    blive.start()

func _on_start_completed(response_json: String):
    print("Start 响应: ", response_json)
    
    # 解析响应获取 WebSocket 信息
    var json = JSON.new()
    var error = json.parse(response_json)
    if error == OK:
        var data = json.data
        if data.has("data") and data["data"].has("websocket_info"):
            var ws_info = data["data"]["websocket_info"]
            var ws_url = ws_info["wss_link"][0]  # 获取第一个 WebSocket 地址
            var auth_body = ws_info["auth_body"]
            
            # 启动 WebSocket 连接
            blive.start_websocket(ws_url, auth_body)
            
            # 启动 API 心跳（每 20 秒）
            var game_id = data["data"]["game_info"]["game_id"]
            start_api_heartbeat(game_id)

func _on_ws_connected():
    print("WebSocket 已连接")

func _on_ws_disconnected():
    print("WebSocket 已断开")

func _on_ws_message_received(cmd: String, data_json: String):
    print("收到消息 CMD: ", cmd)
    print("消息内容: ", data_json)
    
    # 解析消息
    var json = JSON.new()
    var error = json.parse(data_json)
    if error == OK:
        var data = json.data
        match cmd:
            "LIVE_OPEN_PLATFORM_DM":
                # 弹幕消息
                var dm_data = data["data"]
                print("用户: ", dm_data["uname"])
                print("弹幕: ", dm_data["msg"])
            # 可以添加更多 CMD 类型的处理

func _on_ws_error(error_msg: String):
    print("WebSocket 错误: ", error_msg)

# API 心跳定时器
var heartbeat_timer: Timer

func start_api_heartbeat(game_id: String):
    heartbeat_timer = Timer.new()
    add_child(heartbeat_timer)
    heartbeat_timer.wait_time = 20.0
    heartbeat_timer.timeout.connect(func(): blive.heartbeat(game_id))
    heartbeat_timer.start()

func _exit_tree():
    # 停止 WebSocket
    if blive:
        blive.stop_websocket()
```

## 主要功能

### 1. 启动 WebSocket 连接

```gdscript
blive.start_websocket(ws_url, auth_body)
```

- `ws_url`: WebSocket 服务器地址（从 `/v2/app/start` 接口获取）
- `auth_body`: 鉴权字符串（从 `/v2/app/start` 接口获取）

### 2. 停止 WebSocket 连接

```gdscript
blive.stop_websocket()
```

### 3. 信号

- `ws_connected()`: WebSocket 连接成功
- `ws_disconnected()`: WebSocket 连接断开
- `ws_message_received(cmd: String, data_json: String)`: 收到消息
- `ws_error(error_msg: String)`: 发生错误

## 心跳维护

需要维持两个心跳：

1. **WebSocket 心跳**：自动维护，每 20 秒发送一次，用于保持 WebSocket 连接
2. **API 心跳**：需要手动调用 `blive.heartbeat(game_id)`，每 20 秒一次，用于维持鉴权和互动能力

## 常见消息类型 (CMD)

- `LIVE_OPEN_PLATFORM_DM`: 弹幕消息
- `LIVE_OPEN_PLATFORM_SEND_GIFT`: 礼物消息
- `LIVE_OPEN_PLATFORM_SUPER_CHAT`: SC 消息
- `LIVE_OPEN_PLATFORM_GUARD`: 上舰消息
- `LIVE_OPEN_PLATFORM_LIKE`: 点赞消息

更多消息类型请参考 B站直播开放平台文档。

## 注意事项

1. WebSocket 连接在独立线程中运行，不会阻塞主线程
2. 所有信号都通过 `call_deferred` 发送，确保线程安全
3. 记得在节点销毁时调用 `stop_websocket()` 停止连接
4. 压缩数据（Version=2）会自动解压处理
