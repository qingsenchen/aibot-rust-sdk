use aibot_rust_sdk::{
  generate_req_id,
  DefaultLogger,
  WSClient,
  WSClientOptions,
  WsFrame,
  WsFrameHeaders,
  WelcomeReplyBody,
  WelcomeTextReplyBody,
  WelcomeTextContent,
  TemplateCard,
  TemplateCardSource,
  TemplateCardMainTitle,
  TemplateCardSelectionItem,
  TemplateCardSelectionOption,
  TemplateCardSubmitButton,
};
use std::sync::Arc;
use url::Url;

#[tokio::main]
async fn main() {
  // 创建 WSClient 实例
  let ws_client = WSClient::new(WSClientOptions {
    bot_id: "your_bot_id".to_string(),
    secret: "your_bot_secret".to_string(),
    reconnect_interval: None,
    max_reconnect_attempts: None,
    heartbeat_interval: None,
    request_timeout: None,
    ws_url: None,
    logger: Some(Arc::new(DefaultLogger::default())),
  });

  // 模板卡片示例（multiple_interaction）
  let template_card = TemplateCard {
    card_type: "multiple_interaction".to_string(),
    source: Some(TemplateCardSource {
      icon_url: Some("https://wework.qpic.cn/wwpic/252813_jOfDHtcISzuodLa_1629280209/0".to_string()),
      desc: Some("企业微信".to_string()),
      desc_color: None,
    }),
    main_title: Some(TemplateCardMainTitle {
      title: Some("欢迎使用企业微信".to_string()),
      desc: Some("您的好友正在邀请您加入企业微信".to_string()),
    }),
    select_list: Some(vec![
      TemplateCardSelectionItem {
        question_key: "question_key_one".to_string(),
        title: Some("选择标签1".to_string()),
        disable: Some(false),
        selected_id: Some("id_one".to_string()),
        option_list: vec![
          TemplateCardSelectionOption { id: "id_one".to_string(), text: "选择器选项1".to_string() },
          TemplateCardSelectionOption { id: "id_two".to_string(), text: "选择器选项2".to_string() },
        ],
      },
      TemplateCardSelectionItem {
        question_key: "question_key_two".to_string(),
        title: Some("选择标签2".to_string()),
        disable: None,
        selected_id: Some("id_three".to_string()),
        option_list: vec![
          TemplateCardSelectionOption { id: "id_three".to_string(), text: "选择器选项3".to_string() },
          TemplateCardSelectionOption { id: "id_four".to_string(), text: "选择器选项4".to_string() },
        ],
      },
    ]),
    submit_button: Some(TemplateCardSubmitButton {
      text: "提交".to_string(),
      key: "submit_key".to_string(),
    }),
    task_id: Some(format!("task_id_{}", chrono::Utc::now().timestamp_millis())),
    action_menu: None,
    emphasis_content: None,
    quote_area: None,
    sub_title_text: None,
    horizontal_content_list: None,
    jump_list: None,
    card_action: None,
    card_image: None,
    image_text_area: None,
    vertical_content_list: None,
    button_selection: None,
    button_list: None,
    checkbox: None,
    feedback: None,
  };

  // 建立连接
  ws_client.connect();

  // 监听连接事件
  ws_client.on_connected(|| {
    println!("✅ WebSocket 已连接");
  });

  // 监听认证成功事件
  ws_client.on_authenticated(|| {
    println!("🔐 认证成功");
  });

  // 监听断开事件
  ws_client.on_disconnected(|reason| {
    println!("❌ 连接已断开: {}", reason);
  });

  // 监听重连事件
  ws_client.on_reconnecting(|attempt| {
    println!("🔄 正在进行第 {} 次重连...", attempt);
  });

  // 监听错误事件
  ws_client.on_error(|error| {
    eprintln!("⚠️ 发生错误: {}", error);
  });

  // 监听所有消息
  // ws_client.on_message(|frame: WsFrame<aibot_rust_sdk::BaseMessage>| {
  //   if let Some(body) = frame.body {
  //     let text = serde_json::to_string(&body).unwrap_or_default();
  //     println!("📨 收到消息: {}", text.chars().take(200).collect::<String>());
  //   }
  // });

  // 监听文本消息，使用流式回复
  {
    let client = ws_client.clone();
    let template_card = template_card.clone();
    ws_client.on_message_text(move |frame: WsFrame<aibot_rust_sdk::TextMessage>| {
      if let Some(body) = frame.body.clone() {
        println!("📝 收到文本消息: {}", body.text.content);

        let stream_id = generate_req_id("stream");

        // 测试主动发送消息（将 CHATID 替换为实际会话 ID）
        let client_clone = client.clone();
        tokio::spawn(async move {
          let _ = client_clone.send_message(&body.from.userid, aibot_rust_sdk::SendMsgBody::Markdown(
            aibot_rust_sdk::SendMarkdownMsgBody {
              msgtype: "markdown".to_string(),
              markdown: aibot_rust_sdk::SendMarkdownContent { content: "这是一条**主动推送**的消息".to_string() },
            }
          )).await;
        });

        // 发送流式中间内容
        let client_clone = client.clone();
        let headers: WsFrameHeaders = (&frame).into();
        let stream_id_clone = stream_id.clone();
        tokio::spawn(async move {
          let _ = client_clone.reply_stream(&headers, &stream_id_clone, "<think></think>", false, None, None).await;
        });

        // 模拟异步处理后发送最终结果
        let client_clone = client.clone();
        let headers: WsFrameHeaders = (&frame).into();
        let content = body.text.content.clone();
        let stream_id_clone = stream_id.clone();
        tokio::spawn(async move {
          tokio::time::sleep(std::time::Duration::from_secs(2)).await;
          let _ = client_clone.reply_stream(&headers, &stream_id_clone, "你好！你说的是", false, None, None).await;
          tokio::time::sleep(std::time::Duration::from_secs(1)).await;
          let _ = client_clone.reply_stream(
            &headers,
            &stream_id_clone,
            &format!("你好！你说的是: \"{}\"", content),
            true,
            None,
            None,
          )
          .await;
        });

        // 卡片
        let client_clone = client.clone();
        let headers: WsFrameHeaders = (&frame).into();
        let card = template_card.clone();
        tokio::spawn(async move {
          let _ = client_clone.reply_template_card(&headers, card, None).await;
        });
      }
    });
  }

  // 监听进入会话事件（发送欢迎语）
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

  // 监听图片消息，下载并解密
  {
    let client = ws_client.clone();
    ws_client.on_message_image(move |frame: WsFrame<aibot_rust_sdk::ImageMessage>| {
      if let Some(body) = frame.body {
        let image_url = body.image.url.clone();
        println!("🖼️ 收到图片消息: {}", image_url);

        let aeskey = body.image.aeskey.clone();
        let client_clone = client.clone();
        tokio::spawn(async move {
          match client_clone.download_file(&image_url, aeskey.as_deref()).await {
            Ok((buffer, filename)) => {
              println!("✅ 图片下载成功，大小: {} bytes", buffer.len());

              let name = filename
                .or_else(|| url_filename(&image_url))
                .unwrap_or_else(|| format!("image_{}", chrono::Utc::now().timestamp_millis()));

              let save_path = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join(&name);

              if let Err(err) = std::fs::write(&save_path, buffer) {
                eprintln!("❌ 图片保存失败: {}", err);
              } else {
                println!("💾 图片已保存到: {}", save_path.display());
              }
            }
            Err(err) => {
              eprintln!("❌ 图片下载失败: {}", err);
            }
          }
        });
      }
    });
  }

  // 监听图文混排消息
  ws_client.on_message_mixed(|frame: WsFrame<aibot_rust_sdk::MixedMessage>| {
    if let Some(body) = frame.body {
      let items = body.mixed.msg_item;
      println!("🖼️ 收到图文混排消息，包含 {} 个子项", items.len());
      for (index, item) in items.iter().enumerate() {
        match item.msgtype.as_str() {
          "text" => println!("  [{}] 文本: {}", index, item.text.as_ref().map(|t| t.content.clone()).unwrap_or_default()),
          "image" => println!("  [{}] 图片: {}", index, item.image.as_ref().map(|i| i.url.clone()).unwrap_or_default()),
          _ => {}
        }
      }
    }
  });

  // 监听语音消息
  ws_client.on_message_voice(|frame: WsFrame<aibot_rust_sdk::VoiceMessage>| {
    if let Some(body) = frame.body {
      println!("🎙️ 收到语音消息（转文本）: {}", body.voice.content);
    }
  });

  // 监听文件消息
  {
    let client = ws_client.clone();
    ws_client.on_message_file(move |frame: WsFrame<aibot_rust_sdk::FileMessage>| {
      if let Some(body) = frame.body {
        let file_url = body.file.url.clone();
        println!("📁 收到文件消息: {}", file_url);

        let aeskey = body.file.aeskey.clone();
        let client_clone = client.clone();
        tokio::spawn(async move {
          match client_clone.download_file(&file_url, aeskey.as_deref()).await {
            Ok((buffer, filename)) => {
              println!("✅ 文件下载成功，大小: {} bytes", buffer.len());

              let name = filename
                .or_else(|| url_filename(&file_url))
                .unwrap_or_else(|| format!("file_{}", chrono::Utc::now().timestamp_millis()));

              let save_path = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join(&name);

              if let Err(err) = std::fs::write(&save_path, buffer) {
                eprintln!("❌ 文件保存失败: {}", err);
              } else {
                println!("💾 文件已保存到: {}", save_path.display());
              }
            }
            Err(err) => {
              eprintln!("❌ 文件下载失败: {}", err);
            }
          }
        });
      }
    });
  }

  // 优雅退出
  tokio::signal::ctrl_c().await.ok();
  ws_client.disconnect();
}

fn url_filename(url: &str) -> Option<String> {
  Url::parse(url)
    .ok()
    .and_then(|u| u.path_segments().and_then(|s| s.last().map(|v| v.to_string())))
    .filter(|s| !s.is_empty())
}
