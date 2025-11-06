extends Node

# B站直播开放平台 WebSocket 示例

var blive: Blive
var game_id: String = ""
var heartbeat_timer: Timer

func _ready():
	# 创建 Blive 节点
	blive = Blive.new()
	add_child(blive)
	
	# 设置配置（请替换为你的实际配置）
	blive.code = "your_code_here"
	blive.app_id = "your_app_id_here"
	blive.access_key_id = "your_access_key_id_here"
	blive.access_key_secret = "your_access_key_secret_here"
	
	# 连接所有信号
	blive.start_completed.connect(_on_start_completed)
	blive.end_completed.connect(_on_end_completed)
	blive.heartbeat_completed.connect(_on_heartbeat_completed)
	blive.ws_connected.connect(_on_ws_connected)
	blive.ws_disconnected.connect(_on_ws_disconnected)
	blive.ws_message_received.connect(_on_ws_message_received)
	blive.ws_error.connect(_on_ws_error)
	
	print("=== B站直播开放平台 WebSocket 示例 ===")
	print("正在启动项目...")
	
	# 启动项目
	blive.start()

# 处理 start 接口响应
func _on_start_completed(response_json: String):
	print("\n收到 start 响应:")
	print(response_json)
	
	var json = JSON.new()
	var error = json.parse(response_json)
	
	if error != OK:
		print("JSON 解析失败: ", error)
		return
	
	var data = json.data
	
	if not data.has("code") or data["code"] != 0:
		print("启动失败: ", data.get("message", "未知错误"))
		return
	
	# 获取 game_id
	if data.has("data") and data["data"].has("game_info"):
		game_id = data["data"]["game_info"]["game_id"]
		print("\n项目启动成功!")
		print("Game ID: ", game_id)
	
	# 获取 WebSocket 信息并连接
	if data.has("data") and data["data"].has("websocket_info"):
		var ws_info = data["data"]["websocket_info"]
		var wss_links = ws_info["wss_link"]
		var auth_body = ws_info["auth_body"]
		
		if wss_links.size() > 0:
			var ws_url = wss_links[0]
			print("\nWebSocket 地址: ", ws_url)
			print("正在连接 WebSocket...")
			
			# 启动 WebSocket 连接
			blive.start_websocket(ws_url, auth_body)
			
			# 启动 API 心跳定时器
			_start_api_heartbeat()

# WebSocket 连接成功
func _on_ws_connected():
	print("\n✓ WebSocket 连接成功!")
	print("等待接收消息...")

# WebSocket 断开连接
func _on_ws_disconnected():
	print("\n✗ WebSocket 连接已断开")
	_stop_api_heartbeat()

# 收到 WebSocket 消息
func _on_ws_message_received(cmd: String, data_json: String):
	print("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	print("收到消息 CMD: ", cmd)
	
	var json = JSON.new()
	var error = json.parse(data_json)
	
	if error != OK:
		print("消息 JSON 解析失败")
		return
	
	var msg_data = json.data
	
	# 根据不同的 CMD 类型处理消息
	match cmd:
		"LIVE_OPEN_PLATFORM_DM":
			# 弹幕消息
			if msg_data.has("data"):
				var dm = msg_data["data"]
				print("【弹幕】")
				print("  用户: ", dm.get("uname", "未知"))
				print("  UID: ", dm.get("uid", 0))
				print("  Open ID: ", dm.get("open_id", ""))
				print("  内容: ", dm.get("msg", ""))
				print("  粉丝牌等级: ", dm.get("fans_medal_level", 0))
				print("  舰长等级: ", dm.get("guard_level", 0))
		
		"LIVE_OPEN_PLATFORM_SEND_GIFT":
			# 礼物消息
			if msg_data.has("data"):
				var gift = msg_data["data"]
				print("【礼物】")
				print("  用户: ", gift.get("uname", "未知"))
				print("  礼物名称: ", gift.get("gift_name", ""))
				print("  数量: ", gift.get("gift_num", 0))
				print("  价值: ", gift.get("price", 0), " 金瓜子")
		
		"LIVE_OPEN_PLATFORM_SUPER_CHAT":
			# SC 消息
			if msg_data.has("data"):
				var sc = msg_data["data"]
				print("【醒目留言】")
				print("  用户: ", sc.get("uname", "未知"))
				print("  内容: ", sc.get("message", ""))
				print("  金额: ", sc.get("rmb", 0), " 元")
		
		"LIVE_OPEN_PLATFORM_GUARD":
			# 上舰消息
			if msg_data.has("data"):
				var guard = msg_data["data"]
				print("【上舰】")
				print("  用户: ", guard.get("user_info", {}).get("uname", "未知"))
				print("  舰长等级: ", guard.get("guard_level", 0))
				print("  数量: ", guard.get("guard_num", 0))
		
		"LIVE_OPEN_PLATFORM_LIKE":
			# 点赞消息
			if msg_data.has("data"):
				var like = msg_data["data"]
				print("【点赞】")
				print("  用户: ", like.get("uname", "未知"))
				print("  点赞数: ", like.get("like_count", 0))
		
		_:
			# 其他消息类型
			print("【其他消息】")
			print(data_json)
	
	print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

# WebSocket 错误
func _on_ws_error(error_msg: String):
	print("\n✗ WebSocket 错误: ", error_msg)

# API 心跳响应
func _on_heartbeat_completed(response_json: String):
	var json = JSON.new()
	var error = json.parse(response_json)
	
	if error == OK:
		var data = json.data
		if data.has("code") and data["code"] == 0:
			print("♥ API 心跳成功")
		else:
			print("✗ API 心跳失败: ", data.get("message", "未知错误"))

# 项目结束响应
func _on_end_completed(response_json: String):
	print("\n项目已结束")
	print(response_json)

# 启动 API 心跳定时器
func _start_api_heartbeat():
	if heartbeat_timer:
		return
	
	heartbeat_timer = Timer.new()
	add_child(heartbeat_timer)
	heartbeat_timer.wait_time = 20.0
	heartbeat_timer.timeout.connect(_send_api_heartbeat)
	heartbeat_timer.start()
	print("\n✓ API 心跳定时器已启动 (每 20 秒)")

# 停止 API 心跳定时器
func _stop_api_heartbeat():
	if heartbeat_timer:
		heartbeat_timer.stop()
		heartbeat_timer.queue_free()
		heartbeat_timer = null
		print("✗ API 心跳定时器已停止")

# 发送 API 心跳
func _send_api_heartbeat():
	if game_id != "":
		blive.heartbeat(game_id)

# 清理资源
func _exit_tree():
	print("\n正在清理资源...")
	
	# 停止 WebSocket
	if blive:
		blive.stop_websocket()
	
	# 停止心跳定时器
	_stop_api_heartbeat()
	
	# 结束项目
	if game_id != "":
		print("正在结束项目...")
		blive.end(game_id)
