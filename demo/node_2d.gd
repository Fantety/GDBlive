extends Node2D

# B站直播开放平台 GDBLive 插件示例（含 WebSocket 测试）

@onready var blive = $Blive

# 当前场次ID
var current_game_id: String = ""
# WebSocket 连接状态
var ws_connected: bool = false

func _ready():
	# 连接 HTTP API 信号
	blive.start_completed.connect(_on_start_completed)
	blive.end_completed.connect(_on_end_completed)
	blive.heartbeat_completed.connect(_on_heartbeat_completed)
	
	# 连接 WebSocket 信号
	blive.ws_connected.connect(_on_ws_connected)
	blive.ws_disconnected.connect(_on_ws_disconnected)
	blive.ws_message_received.connect(_on_ws_message_received)
	blive.ws_error.connect(_on_ws_error)
	blive.ws_debug.connect(_on_ws_debug)
	blive.heartbeat_debug.connect(_on_heartbeat_debug)
	
	print("=== GDBLive WebSocket 测试 ===")
	print("按 [Space] 开启项目并连接 WebSocket")
	print("按 [E] 关闭项目并断开 WebSocket")
	print("================================")

func _input(event):
	# 按空格键开启项目
	if event is InputEventKey and event.pressed and event.keycode == KEY_SPACE:
		start_project()
	
	# 按 E 键关闭项目
	if event is InputEventKey and event.pressed and event.keycode == KEY_E:
		end_project()

# 开启项目
func start_project():
	print("\n>>> 正在开启项目...")
	blive.start()

# 关闭项目
func end_project():
	if current_game_id.is_empty():
		print("错误：没有活动的场次，无法关闭")
		return
	
	print("\n>>> 正在关闭项目...")
	
	# 停止 WebSocket
	if ws_connected:
		print(">>> 正在断开 WebSocket...")
		blive.stop_websocket()
	
	# 停止心跳
	print(">>> 停止 API 心跳...")
	blive.stop_heartbeat()
	
	# 关闭项目
	blive.end(current_game_id)

# 开启项目完成回调
func _on_start_completed(response_json: String):
	print("\n<<< 开启项目响应:")
	print(response_json)
	# 解析 JSON 响应
	var json = JSON.new()
	var error = json.parse(response_json)
	if error == OK:
		var response = json.data
		# 检查响应码
		if response.has("code") and response.code == 0:
			print("✓ 项目开启成功！")
			# 获取数据部分
			if response.has("data"):
				var data = response.data
				# 保存 game_id
				if data.has("game_info") and data.game_info.has("game_id"):
					current_game_id = data.game_info.game_id
					print("  场次ID: ", current_game_id)
				# 启动 WebSocket 连接
				if data.has("websocket_info"):
					var ws_info = data.websocket_info
					if ws_info.has("auth_body"):
						print("  WebSocket 认证体: ", ws_info.auth_body.substr(0, 50), "...")
					if ws_info.has("wss_link"):
						print("  WebSocket 链接数量: ", ws_info.wss_link.size())
						if ws_info.wss_link.size() > 0:
							var ws_url = ws_info.wss_link[0]
							var auth_body = ws_info.auth_body
							print("  主链接: ", ws_url)
							print("\n>>> 正在连接 WebSocket...")
							blive.start_websocket(ws_url, auth_body)
							
							# 启动 API 心跳
							print(">>> 启动 API 心跳...")
							blive.start_heartbeat(current_game_id)
				
				# 显示主播信息
				if data.has("anchor_info"):
					var anchor = data.anchor_info
					print("  === 主播信息 ===")
					if anchor.has("uname"):
						print("  昵称: ", anchor.uname)
					if anchor.has("uid"):
						print("  UID: ", anchor.uid)
					if anchor.has("room_id"):
						print("  房间ID: ", anchor.room_id)
					if anchor.has("uface"):
						print("  头像: ", anchor.uface)
					if anchor.has("open_id"):
						print("  Open ID: ", anchor.open_id)
					if anchor.has("union_id"):
						print("  Union ID: ", anchor.union_id)
		else:
			print("✗ 项目开启失败")
			if response.has("message"):
				print("  错误信息: ", response.message)
			if response.has("code"):
				print("  错误码: ", response.code)
	else:
		print("✗ JSON 解析失败: ", error)

# 关闭项目完成回调
func _on_end_completed(response_json: String):
	print("\n<<< 关闭项目响应:")
	print(response_json)
	
	# 解析 JSON 响应
	var json = JSON.new()
	var error = json.parse(response_json)
	
	if error == OK:
		var response = json.data
		
		# 检查响应码
		if response.has("code") and response.code == 0:
			print("✓ 项目关闭成功！")
			current_game_id = ""
		else:
			print("✗ 项目关闭失败")
			if response.has("message"):
				print("  错误信息: ", response.message)
			if response.has("code"):
				print("  错误码: ", response.code)
	else:
		print("✗ JSON 解析失败: ", error)

# API 心跳完成回调
func _on_heartbeat_completed(response_json: String):
	var json = JSON.new()
	var error = json.parse(response_json)
	
	if error == OK:
		var response = json.data
		if response.has("code") and response.code == 0:
			print("♥ API 心跳成功")
		else:
			print("✗ API 心跳失败: ", response.get("message", "未知错误"))
	else:
		print("✗ API 心跳 JSON 解析失败")

# WebSocket 连接成功
func _on_ws_connected():
	ws_connected = true
	print("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	print("✓ WebSocket 连接成功！")
	print("等待接收消息...")
	print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

# WebSocket 断开连接
func _on_ws_disconnected():
	ws_connected = false
	print("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	print("✗ WebSocket 连接已断开")
	print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

# WebSocket 收到消息
func _on_ws_message_received(cmd: String, data_json: String):
	print("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	print("📨 收到消息 CMD: ", cmd)
	
	var json = JSON.new()
	var error = json.parse(data_json)
	
	if error != OK:
		print("✗ 消息 JSON 解析失败")
		print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
		return
	
	var msg_data = json.data
	
	# 根据不同的 CMD 类型处理消息
	match cmd:
		"LIVE_OPEN_PLATFORM_DM":
			# 弹幕消息
			if msg_data.has("data"):
				var dm = msg_data.data
				print("💬 【弹幕】")
				print("   用户: ", dm.get("uname", "未知"))
				print("   内容: ", dm.get("msg", ""))
				if dm.get("fans_medal_level", 0) > 0:
					print("   粉丝牌: Lv.", dm.get("fans_medal_level", 0))
				if dm.get("guard_level", 0) > 0:
					print("   舰长等级: ", dm.get("guard_level", 0))
		
		"LIVE_OPEN_PLATFORM_SEND_GIFT":
			# 礼物消息
			if msg_data.has("data"):
				var gift = msg_data.data
				print("🎁 【礼物】")
				print("   用户: ", gift.get("uname", "未知"))
				print("   礼物: ", gift.get("gift_name", ""), " x", gift.get("gift_num", 0))
				print("   价值: ", gift.get("price", 0), " 金瓜子")
		
		"LIVE_OPEN_PLATFORM_SUPER_CHAT":
			# SC 消息
			if msg_data.has("data"):
				var sc = msg_data.data
				print("💰 【醒目留言】")
				print("   用户: ", sc.get("uname", "未知"))
				print("   内容: ", sc.get("message", ""))
				print("   金额: ¥", sc.get("rmb", 0))
		
		"LIVE_OPEN_PLATFORM_GUARD":
			# 上舰消息
			if msg_data.has("data"):
				var guard = msg_data.data
				var user_info = guard.get("user_info", {})
				print("⚓ 【上舰】")
				print("   用户: ", user_info.get("uname", "未知"))
				print("   舰长等级: ", guard.get("guard_level", 0))
				print("   数量: ", guard.get("guard_num", 0))
		
		"LIVE_OPEN_PLATFORM_LIKE":
			# 点赞消息
			if msg_data.has("data"):
				var like = msg_data.data
				print("👍 【点赞】")
				print("   用户: ", like.get("uname", "未知"))
				print("   点赞数: ", like.get("like_count", 0))
		
		_:
			# 其他消息类型
			print("📋 【其他消息】")
			print(data_json)
	
	print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

# WebSocket 错误
func _on_ws_error(error_msg: String):
	print("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	print("❌ WebSocket 错误: ", error_msg)
	print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

# WebSocket 调试日志
func _on_ws_debug(debug_msg: String):
	print("🔍 [WS Debug] ", debug_msg)

# 心跳调试日志
func _on_heartbeat_debug(debug_msg: String):
	print("💓 [Heartbeat Debug] ", debug_msg)



# 清理资源
func _exit_tree():
	print("\n>>> 正在清理资源...")
	
	# 停止 WebSocket
	if ws_connected:
		blive.stop_websocket()
	
	# 停止心跳
	blive.stop_heartbeat()
	
	# 结束项目
	if not current_game_id.is_empty():
		blive.end(current_game_id)
