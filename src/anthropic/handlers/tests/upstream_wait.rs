// Copyright (c) 2026 Harllan He. Licensed under MIT.
// 等待上游响应头阶段的 SSE 保活测试
#[cfg(test)]
mod tests {

    use super::super::super::bridge::body_dummy_bytes;
    use super::super::super::error::provider_error_sse_event;
    use super::super::super::stream::{create_awaiting_upstream_sse_stream, keepalive_comment_sse};

    use bytes::Bytes;
    use futures::{StreamExt, stream};
    use std::convert::Infallible;
    use std::time::Duration;
    use tokio::sync::oneshot;

    type UpstreamResult = anyhow::Result<(reqwest::Response, u64)>;

    const MESSAGE_START_FRAME: &str = "event: message_start\ndata: {}\n\n";
    const NEXT_FRAME_TIMEOUT: Duration = Duration::from_secs(1);

    fn dummy_upstream_response() -> reqwest::Response {
        reqwest::Response::from(http::Response::new(body_dummy_bytes()))
    }

    /// 上游调用由 oneshot 控制完成时机；into_sse_stream 产出单个 message_start 帧
    fn awaiting_stream(
        upstream_rx: oneshot::Receiver<UpstreamResult>,
    ) -> impl futures::Stream<Item = Result<Bytes, Infallible>> {
        let upstream_call = Box::pin(async move { upstream_rx.await.unwrap() });
        let into_sse_stream = |_response: reqwest::Response, _credential_id: u64| {
            stream::iter([Ok::<_, Infallible>(Bytes::from_static(
                MESSAGE_START_FRAME.as_bytes(),
            ))])
        };
        create_awaiting_upstream_sse_stream(
            upstream_call,
            into_sse_stream,
            "claude-sonnet-4".to_string(),
            100,
        )
    }

    async fn next_frame(
        sse: &mut (impl futures::Stream<Item = Result<Bytes, Infallible>> + Unpin),
    ) -> Option<String> {
        let item = tokio::time::timeout(NEXT_FRAME_TIMEOUT, sse.next())
            .await
            .expect("帧未在期限内到达");
        item.map(|frame| String::from_utf8(frame.unwrap().to_vec()).unwrap())
    }

    #[tokio::test]
    async fn test_awaiting_upstream_emits_keepalive_before_upstream_headers() {
        // 上游未就绪时首帧必须立即是注释保活：反代据此认为源站已开始响应
        let (_upstream_tx, upstream_rx) = oneshot::channel::<UpstreamResult>();
        let mut sse = Box::pin(awaiting_stream(upstream_rx));

        let first = next_frame(&mut sse).await.unwrap();
        assert_eq!(first.as_bytes(), keepalive_comment_sse().as_ref());
        assert!(first.starts_with(':'), "保活必须是 SSE 注释帧，客户端忽略");
    }

    #[tokio::test]
    async fn test_awaiting_upstream_switches_to_event_stream_on_success() {
        let (upstream_tx, upstream_rx) = oneshot::channel::<UpstreamResult>();
        let mut sse = Box::pin(awaiting_stream(upstream_rx));
        assert_eq!(
            next_frame(&mut sse).await.unwrap().as_bytes(),
            keepalive_comment_sse().as_ref()
        );

        upstream_tx
            .send(Ok((dummy_upstream_response(), 1)))
            .unwrap();

        assert_eq!(next_frame(&mut sse).await.unwrap(), MESSAGE_START_FRAME);
        assert_eq!(next_frame(&mut sse).await, None, "事件流耗尽后 SSE 应结束");
    }

    #[tokio::test]
    async fn test_awaiting_upstream_failure_becomes_single_error_event() {
        // 响应头已是 200：上游失败只能以流内 error 事件告知，且保留 429 的可重试类型
        let (upstream_tx, upstream_rx) = oneshot::channel::<UpstreamResult>();
        let mut sse = Box::pin(awaiting_stream(upstream_rx));
        next_frame(&mut sse).await.unwrap();

        upstream_tx
            .send(Err(anyhow::anyhow!("上游限流：429 Too Many Requests")))
            .unwrap();

        let frame = next_frame(&mut sse).await.unwrap();
        assert!(frame.starts_with("event: error\n"), "{frame}");
        let data: serde_json::Value =
            serde_json::from_str(frame.trim_end().split_once("data: ").unwrap().1).unwrap();
        assert_eq!(data["error"]["type"], "rate_limit_error");
        assert_eq!(next_frame(&mut sse).await, None);
    }

    #[test]
    fn test_provider_error_sse_event_matches_http_classification() {
        let cases = [
            ("[QUOTA_EXHAUSTED_ALL]", "quota_exceeded_error"),
            ("CONTENT_LENGTH_EXCEEDS_THRESHOLD", "invalid_request_error"),
            ("Input is too long", "invalid_request_error"),
            ("连接超时", "api_error"),
        ];
        for (upstream_error, expected_type) in cases {
            let event =
                provider_error_sse_event(&anyhow::anyhow!(upstream_error), "claude-sonnet-4", 100);
            assert_eq!(event.event, "error");
            assert_eq!(
                event.data["error"]["type"], expected_type,
                "{upstream_error}"
            );
        }
    }
}
