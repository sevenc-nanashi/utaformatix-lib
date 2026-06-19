use rquickjs::{Ctx, Result, TypedArray};
use tracing::info;

pub async fn sleep(delay: u32) -> Result<()> {
    tokio::time::sleep(std::time::Duration::from_millis(u64::from(delay))).await;
    Ok(())
}

pub fn encode<'js>(ctx: Ctx<'js>, string: String) -> Result<TypedArray<'js, u8>> {
    TypedArray::new(ctx, string.into_bytes())
}

pub fn decode(array: TypedArray<'_, u8>, encoding: String) -> Result<String> {
    let data = array
        .as_bytes()
        .expect("Failed to get typed array bytes")
        .to_vec();

    info!("Decoding data with encoding: {}", encoding);
    let encoding =
        encoding_rs::Encoding::for_label(encoding.as_bytes()).expect("Failed to get encoding");

    let (decoded, _, _) = encoding.decode(&data);
    Ok(decoded.to_string())
}

pub fn log(level: u8, message: String) {
    match level {
        0 => info!("[JS] {}", message),
        1 => tracing::warn!("[JS] {}", message),
        2 => tracing::error!("[JS] {}", message),
        _ => tracing::warn!("[JS] {}", message),
    }
}
