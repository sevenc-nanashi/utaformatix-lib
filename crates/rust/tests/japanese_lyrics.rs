use tracing_test::traced_test;
use utaformatix_rs::ParseOptions;

#[rstest::fixture]
fn utaformatix() -> utaformatix_rs::base::UtaFormatix {
    utaformatix_rs::base::UtaFormatix::new()
}

#[rstest::rstest]
#[tokio::test]
#[traced_test]
async fn analyze_japanese_lyrics_type(utaformatix: utaformatix_rs::base::UtaFormatix) {
    let data = include_bytes!("../utaformatix-ts/testAssets/tsukuyomi_vcv.ust");
    let options = ParseOptions::default();
    let result = utaformatix.parse_ust(&[data], options).await;

    let parsed = result.expect("Failed to parse data");

    let result = utaformatix
        .analyze_japanese_lyrics_type(parsed)
        .await
        .expect("Failed to analyze Japanese lyrics type");

    assert_eq!(result, Some(utaformatix_rs::JapaneseLyricsType::KanaVcv));
}

#[rstest::rstest]
#[tokio::test]
#[traced_test]
async fn convert_japanese_lyrics(utaformatix: utaformatix_rs::base::UtaFormatix) {
    let data = include_bytes!("../utaformatix-ts/testAssets/tsukuyomi_vcv.ust");
    let options = ParseOptions::default();
    let result = utaformatix.parse_ust(&[data], options).await;

    let parsed = result.expect("Failed to parse data");

    let result = utaformatix
        .convert_japanese_lyrics(
            parsed,
            utaformatix_rs::JapaneseLyricsType::KanaVcv,
            utaformatix_rs::JapaneseLyricsType::KanaCv,
            Default::default(),
        )
        .await
        .expect("Failed to convert Japanese lyrics");

    insta::assert_debug_snapshot!(result);
}
