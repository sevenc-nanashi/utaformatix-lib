use boa_engine::JsArgs;
use boa_engine::{Context, JsResult, JsValue};
use std::future::Future;

pub fn sleep(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> impl Future<Output = JsResult<JsValue>> {
    let delay = args.get_or_undefined(0).to_u32(context).unwrap();
    async move {
        tokio::time::sleep(std::time::Duration::from_millis(u64::from(delay))).await;
        Ok(JsValue::undefined())
    }
}
