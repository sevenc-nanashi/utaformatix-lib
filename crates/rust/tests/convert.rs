extern crate utaformatix_rs;

use duplicate::duplicate_item;
use tracing_test::traced_test;
use utaformatix_rs::{GenerateOptions, ParseOptions};

#[rstest::fixture]
fn utaformatix() -> utaformatix_rs::base::UtaFormatix {
    utaformatix_rs::base::UtaFormatix::new()
}

#[duplicate_item(
    test_name                 function             path;
    [test_parse_standard_mid] [parse_standard_mid] ["generated/standard.mid"];
    [parse_music_xml]         [parse_music_xml]    ["generated/musicXml.musicxml"];
    [parse_ccs]               [parse_ccs]          ["generated/cevio.ccs"];
    [parse_dv]                [parse_dv]           ["generated/dv.dv"];
    [parse_ustx]              [parse_ustx]         ["generated/openutau.ustx"];
    [parse_svp]               [parse_svp]          ["generated/synthV.svp"];
    [parse_tssln]             [parse_tssln]        ["voisona.tssln"];
    [parse_uf_data]           [parse_uf_data]      ["generated/ufdata.ufdata"];
    [parse_vocaloid_mid]      [parse_vocaloid_mid] ["generated/vocaloid.mid"];
    [parse_vsq]               [parse_vsq]          ["generated/vsq.vsq"];
    [parse_vsqx]              [parse_vsqx]         ["generated/vsqx.vsqx"];
    [parse_vpr]               [parse_vpr]          ["generated/vpr.vpr"];
)]
#[rstest::rstest]
#[tokio::test]
#[traced_test]
async fn test_name(utaformatix: utaformatix_rs::base::UtaFormatix) {
    let data = include_bytes!(concat!("../utaformatix-ts/testAssets/", path));
    let options = ParseOptions::default();
    let result = utaformatix.function(data, options).await;

    let parsed = result.expect("Failed to parse data");

    insta::assert_debug_snapshot!(parsed);
}

#[duplicate_item(
    test_name                 function             path;
    [test_parse_ust]          [parse_ust]          ["generated/utau.ust"];
)]
#[rstest::rstest]
#[tokio::test]
#[traced_test]
async fn test_name(utaformatix: utaformatix_rs::base::UtaFormatix) {
    let data = include_bytes!(concat!("../utaformatix-ts/testAssets/", path));
    let options = ParseOptions::default();
    let result = utaformatix.function(&[data], options).await;

    let parsed = result.expect("Failed to parse data");

    insta::assert_debug_snapshot!(parsed);
}

#[duplicate_item(
    test_name                    function;
    [test_generate_standard_mid] [generate_standard_mid];
)]
#[rstest::rstest]
#[tokio::test]
#[traced_test]
async fn test_name(utaformatix: utaformatix_rs::base::UtaFormatix) {
    let data = include_bytes!("../utaformatix-ts/testAssets/generated/standard.mid");
    let ufdata = utaformatix
        .parse_standard_mid(data, ParseOptions::default())
        .await
        .expect("Failed to parse data");
    let options = GenerateOptions::default();
    let result = utaformatix.function(ufdata, options).await;

    result.expect("Failed to generate data");
}
