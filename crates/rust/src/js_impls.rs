use boa_engine::{
    js_string,
    object::builtins::{JsArray, JsTypedArray},
    Context, JsArgs, JsNativeError, JsResult, JsString, JsValue,
};
use std::future::Future;
use tracing::info;

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

pub fn encode(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let string = args
        .get_or_undefined(0)
        .as_string()
        .ok_or_else(|| JsNativeError::error().with_message("Invalid string"))?
        .to_std_string()
        .map_err(|_| JsNativeError::error().with_message("Invalid string"))?;
    let bytes = string.as_bytes();
    let array = JsArray::from_iter(
        bytes.iter().map(|byte| JsValue::new(byte.to_owned())),
        context,
    );

    Ok(JsValue::new(array))
}

pub fn decode(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let array = args.get_or_undefined(0).to_owned();
    let encoding = args
        .get_or_undefined(1)
        .as_string()
        .ok_or_else(|| {
            JsNativeError::error().with_message(format!("Invalid encoding: {:?}", args[1]))
        })?
        .to_std_string()
        .expect("Failed to get string");

    let uint8array = context
        .global_object()
        .get(js_string!("Uint8Array"), context)
        .expect("Failed to get Uint8Array");

    let array = uint8array
        .as_constructor()
        .expect("Failed to get constructor")
        .construct(&[array], None, context)?;

    let array = JsTypedArray::from_object(array.to_owned())
        .map_err(|_| JsNativeError::error().with_message("Invalid array"))?;

    let length = array.length(context).expect("Failed to get length");
    let mut data = Vec::with_capacity(length);
    for i in 0..length {
        let value = array.get(i, context).expect("Failed to get value");
        data.push(value.as_number().expect("Failed to get number") as u8);
    }

    info!("Decoding data with encoding: {}", encoding);
    let encoding =
        encoding_rs::Encoding::for_label(encoding.as_bytes()).expect("Failed to get encoding");

    let (decoded, _, _) = encoding.decode(&data);

    let result_string = JsValue::String(JsString::from(decoded.to_string()));

    Ok(result_string)
}
