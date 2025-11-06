extends Node2D

# B站直播开放平台 GDBLive 插件示例

@onready var blive = $Blive

# 当前场次ID
var current_game_id: String = ""

func _ready():
	# 连接信号
	blive.start_completed.connect(_on_start_completed)
	blive.end_completed.connect(_on_end_completed)
	
	print("=== GDBLive 示例启动 ===")
	print("按 [Space] 开启项目")
	print("按 [E] 关闭项目")
	print("========================")

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
				# 显示 WebSocket 连接信息
				if data.has("websocket_info"):
					var ws_info = data.websocket_info
					if ws_info.has("auth_body"):
						print("  WebSocket 认证体: ", ws_info.auth_body.substr(0, 50), "...")
					if ws_info.has("wss_link"):
						print("  WebSocket 链接数量: ", ws_info.wss_link.size())
						if ws_info.wss_link.size() > 0:
							print("  主链接: ", ws_info.wss_link[0])
				
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
