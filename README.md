# aibot-rust-sdk

企业微信智能机器人 Rust SDK —— 基于 WebSocket 长连接通道，提供消息收发、流式回复、模板卡片、事件回调、文件下载解密、媒体素材上传等核心能力。

## 特性

- WebSocket 长连接，内置默认地址 `wss://openws.work.weixin.qq.com`
- 连接后自动认证（bot_id + secret）
- 心跳保活与断线重连（指数退避，默认上限 30s）
- 消息分发：text / image / mixed / voice / file
- 流式回复与模板卡片回复
- 主动推送（Markdown / 模板卡片 / 媒体）
- 事件回调：enter_chat / template_card_event / feedback_event
- 同一 req_id 串行回复队列，自动等待回执
- 内置 AES-256-CBC 文件解密与素材分片上传
- 可插拔日志，内置 `DefaultLogger`

## 安装

本仓库暂未发布到 crates.io。使用本地或 Git 依赖：

```toml
[dependencies]
aibot-rust-sdk = { path = "." }
```

或（如你已发布到 Git 仓库）：

```toml
[dependencies]
aibot-rust-sdk = { git = "YOUR_GIT_REPO_URL" }
```

## 快速开始

示例见 `examples/quick_start.rs`：

```rust
use aibot_rust_sdk::{
  generate_req_id,
  DefaultLogger,
  WSClient,
  WSClientOptions,
  WsFrame,
  WsFrameHeaders,
  WelcomeReplyBody,
  WelcomeTextContent,
  WelcomeTextReplyBody,
};
use std::sync::Arc;

#[tokio::main]
async fn main() {
  // 1. 创建客户端实例
  let ws_client = WSClient::new(WSClientOptions {
    bot_id: "your-bot-id".to_string(),
    secret: "your-bot-secret".to_string(),
    reconnect_interval: None,
    max_reconnect_attempts: None,
    heartbeat_interval: None,
    request_timeout: None,
    ws_url: None,
    logger: Some(Arc::new(DefaultLogger::default())),
  });

  // 2. 建立连接
  ws_client.connect();

  // 3. 监听认证成功
  ws_client.on_authenticated(|| {
    println!("🔐 认证成功");
  });

  // 4. 监听文本消息并进行流式回复
  {
    let client = ws_client.clone();
    ws_client.on_message_text(move |frame: WsFrame<aibot_rust_sdk::TextMessage>| {
      if let Some(body) = frame.body.clone() {
        let content = body.text.content;
        println!("收到文本: {}", content);

        let stream_id = generate_req_id("stream");
        let headers: WsFrameHeaders = (&frame).into();

        let client_clone = client.clone();
        tokio::spawn(async move {
          let _ = client_clone
            .reply_stream(&headers, &stream_id, "正在思考中...", false, None, None)
            .await;

          tokio::time::sleep(std::time::Duration::from_secs(1)).await;
          let _ = client_clone
            .reply_stream(
              &headers,
              &stream_id,
              &format!("你好！你说的是: \"{}\"", content),
              true,
              None,
              None,
            )
            .await;
        });
      }
    });
  }

  // 5. 监听进入会话事件（发送欢迎语）
  {
    let client = ws_client.clone();
    ws_client.on_event_enter_chat(move |frame: WsFrame<aibot_rust_sdk::EventMessage>| {
      let headers: WsFrameHeaders = (&frame).into();
      let client_clone = client.clone();
      tokio::spawn(async move {
        let body = WelcomeReplyBody::Text(WelcomeTextReplyBody {
          msgtype: "text".to_string(),
          text: WelcomeTextContent { content: "您好！我是智能助手，有什么可以帮您的吗？".to_string() },
        });
        let _ = client_clone.reply_welcome(&headers, body).await;
      });
    });
  }

  // 6. 优雅退出
  tokio::signal::ctrl_c().await.ok();
  ws_client.disconnect();
}
```

## API 文档

### WSClient

核心客户端类，负责连接管理与消息收发。

#### 方法一览

| 方法 | 说明 | 返回值 |
| --- | --- | --- |
| `connect()` | 建立 WebSocket 连接，连接后自动认证 | `()` |
| `disconnect()` | 主动断开连接 | `()` |
| `is_connected()` | 当前连接状态 | `bool` |
| `api()` | 获取内部 API 客户端（高级用途） | `WeComApiClient` |
| `reply(frame, body, cmd)` | 通用回复（低阶 API） | `Result<WsFrame<Value>, AiBotError>` |
| `reply_stream(frame, stream_id, content, finish, msg_item, feedback)` | 流式回复（支持 Markdown） | `Result<WsFrame<Value>, AiBotError>` |
| `reply_welcome(frame, body)` | 欢迎语回复，需 5 秒内调用 | `Result<WsFrame<Value>, AiBotError>` |
| `reply_template_card(frame, template_card, feedback)` | 回复模板卡片 | `Result<WsFrame<Value>, AiBotError>` |
| `reply_stream_with_card(frame, stream_id, content, finish, options)` | 流式回复 + 模板卡片组合 | `Result<WsFrame<Value>, AiBotError>` |
| `update_template_card(frame, template_card, userids)` | 更新模板卡片，需 5 秒内调用 | `Result<WsFrame<Value>, AiBotError>` |
| `send_message(chatid, body)` | 主动发送消息（Markdown / 模板卡片 / 媒体） | `Result<WsFrame<Value>, AiBotError>` |
| `upload_media(file_buffer, options)` | 上传临时素材（三步分片上传） | `Result<UploadMediaFinishResult, AiBotError>` |
| `reply_media(frame, media_type, media_id, video_options)` | 被动回复媒体消息 | `Result<WsFrame<Value>, AiBotError>` |
| `send_media_message(chatid, media_type, media_id, video_options)` | 主动发送媒体消息 | `Result<WsFrame<Value>, AiBotError>` |
| `download_file(url, aes_key)` | 下载文件并解密 | `Result<(Vec<u8>, Option<String>), AiBotError>` |

#### 事件回调

| 方法 | 回调参数 | 说明 |
| --- | --- | --- |
| `on_connected` | `Fn()` | WebSocket 连接建立 |
| `on_authenticated` | `Fn()` | 认证成功 |
| `on_disconnected` | `Fn(String)` | 连接断开 |
| `on_reconnecting` | `Fn(u32)` | 正在重连（第 N 次） |
| `on_error` | `Fn(String)` | 发生错误 |
| `on_message` | `Fn(WsFrame<BaseMessage>)` | 收到消息（所有类型） |
| `on_message_text` | `Fn(WsFrame<TextMessage>)` | 收到文本消息 |
| `on_message_image` | `Fn(WsFrame<ImageMessage>)` | 收到图片消息 |
| `on_message_mixed` | `Fn(WsFrame<MixedMessage>)` | 收到图文混排消息 |
| `on_message_voice` | `Fn(WsFrame<VoiceMessage>)` | 收到语音消息 |
| `on_message_file` | `Fn(WsFrame<FileMessage>)` | 收到文件消息 |
| `on_event` | `Fn(WsFrame<EventMessage>)` | 收到事件回调（所有事件类型） |
| `on_event_enter_chat` | `Fn(WsFrame<EventMessage>)` | 进入会话事件 |
| `on_event_template_card_event` | `Fn(WsFrame<EventMessage>)` | 模板卡片点击事件 |
| `on_event_feedback_event` | `Fn(WsFrame<EventMessage>)` | 用户反馈事件 |

### reply_stream 说明

```rust
reply_stream(
  frame: &WsFrameHeaders,
  stream_id: &str,
  content: &str,
  finish: bool,
  msg_item: Option<Vec<ReplyMsgItem>>,
  feedback: Option<ReplyFeedback>,
)
```

### reply_welcome 说明

欢迎语需在收到 `event.enter_chat` 后 5 秒内调用。

### update_template_card 说明

更新模板卡片需在收到 `event.template_card_event` 后 5 秒内调用，且 `task_id` 必须一致。

### send_message 说明

`chatid` 单聊填用户 `userid`，群聊填 `chatid`。

### upload_media 说明

三步分片上传：`init → chunk × N → finish`。

## 配置选项

`WSClientOptions`：

| 参数 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `bot_id` | `String` | 是 | — | 机器人 ID |
| `secret` | `String` | 是 | — | 机器人 Secret |
| `reconnect_interval` | `Option<u64>` | 否 | `1000` | 重连基础延迟（ms），指数退避 |
| `max_reconnect_attempts` | `Option<i64>` | 否 | `10` | 最大重连次数（`-1` 为无限重连） |
| `heartbeat_interval` | `Option<u64>` | 否 | `30000` | 心跳间隔（ms） |
| `request_timeout` | `Option<u64>` | 否 | `10000` | HTTP 请求超时（ms） |
| `ws_url` | `Option<String>` | 否 | `wss://openws.work.weixin.qq.com` | 自定义 WS 地址 |
| `logger` | `Option<Arc<dyn Logger>>` | 否 | `DefaultLogger` | 自定义日志 |

## 消息与事件类型

### MessageType

| 类型 | 值 | 说明 |
| --- | --- | --- |
| `Text` | `text` | 文本消息 |
| `Image` | `image` | 图片消息 |
| `Mixed` | `mixed` | 图文混排消息 |
| `Voice` | `voice` | 语音消息（已转文本） |
| `File` | `file` | 文件消息 |

### EventType

| 类型 | 值 | 说明 |
| --- | --- | --- |
| `EnterChat` | `enter_chat` | 进入会话事件 |
| `TemplateCardEvent` | `template_card_event` | 模板卡片事件 |
| `FeedbackEvent` | `feedback_event` | 用户反馈事件 |

### WeComMediaType

`"file" | "image" | "voice" | "video"`

### TemplateCardType

| 类型 | 值 | 说明 |
| --- | --- | --- |
| `TextNotice` | `text_notice` | 文本通知 |
| `NewsNotice` | `news_notice` | 图文展示 |
| `ButtonInteraction` | `button_interaction` | 按钮交互 |
| `VoteInteraction` | `vote_interaction` | 投票选择 |
| `MultipleInteraction` | `multiple_interaction` | 多项选择 |

## WebSocket 命令协议

| 方向 | 常量 | 值 | 说明 |
| --- | --- | --- | --- |
| 开发者 → 企微 | `SUBSCRIBE` | `aibot_subscribe` | 认证订阅 |
| 开发者 → 企微 | `HEARTBEAT` | `ping` | 心跳 |
| 开发者 → 企微 | `RESPONSE` | `aibot_respond_msg` | 回复消息 |
| 开发者 → 企微 | `RESPONSE_WELCOME` | `aibot_respond_welcome_msg` | 回复欢迎语 |
| 开发者 → 企微 | `RESPONSE_UPDATE` | `aibot_respond_update_msg` | 更新模板卡片 |
| 开发者 → 企微 | `SEND_MSG` | `aibot_send_msg` | 主动发送消息 |
| 开发者 → 企微 | `UPLOAD_MEDIA_INIT` | `aibot_upload_media_init` | 上传素材 - 初始化 |
| 开发者 → 企微 | `UPLOAD_MEDIA_CHUNK` | `aibot_upload_media_chunk` | 上传素材 - 分片 |
| 开发者 → 企微 | `UPLOAD_MEDIA_FINISH` | `aibot_upload_media_finish` | 上传素材 - 完成 |
| 企微 → 开发者 | `CALLBACK` | `aibot_msg_callback` | 消息推送回调 |
| 企微 → 开发者 | `EVENT_CALLBACK` | `aibot_event_callback` | 事件推送回调 |

## 项目结构

```
aibot-rust-sdk/
├── src/
│   ├── lib.rs               # 统一导出
│   ├── client.rs            # WSClient 核心客户端
│   ├── ws.rs                # WebSocket 长连接管理器
│   ├── message_handler.rs   # 消息解析与事件分发
│   ├── api.rs               # HTTP API 客户端（文件下载）
│   ├── crypto.rs            # AES-256-CBC 文件解密
│   ├── logger.rs            # 默认日志实现
│   ├── utils.rs             # 工具方法（generate_req_id 等）
│   └── types/
│       ├── config.rs         # 配置选项类型
│       ├── event.rs          # 事件类型
│       ├── message.rs        # 消息类型
│       ├── api.rs            # API/WebSocket 帧/模板卡片类型
│       └── common.rs         # 通用类型（Logger）
├── examples/
│   ├── basic.rs             # 完整示例
│   └── quick_start.rs       # 快速开始示例
├── Cargo.toml
└── Cargo.lock
```

## 导出说明

本 SDK 采用具名导出：

```rust
use aibot_rust_sdk::{WSClient, WSClientOptions, generate_req_id};
```

完整导出列表参见 `src/lib.rs`。

## License

MIT
