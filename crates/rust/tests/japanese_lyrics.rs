use tracing_test::traced_test;
use utaformatix::ParseOptions;

#[rstest::fixture]
fn utaformatix_instance() -> utaformatix::base::UtaFormatix {
    utaformatix::base::UtaFormatix::new()
}

#[rstest::rstest]
#[tokio::test]
#[traced_test]
async fn analyze_japanese_lyrics_type(utaformatix_instance: utaformatix::base::UtaFormatix) {
    let data = include_bytes!("../utaformatix-ts/testAssets/tsukuyomi_vcv.ust");
    let options = ParseOptions::default();
    let result = utaformatix_instance.parse_ust(&[data], options).await;

    let parsed = result.expect("Failed to parse data");

    let result = utaformatix_instance
        .analyze_japanese_lyrics_type(parsed)
        .await
        .expect("Failed to analyze Japanese lyrics type");

    assert_eq!(result, Some(utaformatix::JapaneseLyricsType::KanaVcv));
}

#[rstest::rstest]
#[tokio::test]
#[traced_test]
async fn convert_japanese_lyrics(utaformatix_instance: utaformatix::base::UtaFormatix) {
    let data = include_bytes!("../utaformatix-ts/testAssets/tsukuyomi_vcv.ust");
    let options = ParseOptions::default();
    let result = utaformatix_instance.parse_ust(&[data], options).await;

    let parsed = result.expect("Failed to parse data");

    let result = utaformatix_instance
        .convert_japanese_lyrics(
            parsed,
            utaformatix::JapaneseLyricsType::KanaVcv,
            utaformatix::JapaneseLyricsType::KanaCv,
            Default::default(),
        )
        .await
        .expect("Failed to convert Japanese lyrics");

    insta::assert_debug_snapshot!(result);
}
