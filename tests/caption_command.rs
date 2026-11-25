#[tokio::test]
async fn caption_registered_with_required_text_and_attachment() {
    let mut commands = Vec::new();
    bibliotecaria::commands::register_all(&mut commands).await;
    let c = commands
        .iter()
        .map(|c| serde_json::to_value(c).unwrap())
        .find(|c| c["name"] == "caption")
        .expect("caption registered");
    assert_eq!(c["options"][0]["name"], "texto");
    assert_eq!(c["options"][0]["required"], true);
    assert_eq!(c["options"][1]["name"], "arquivo");
    assert_eq!(c["options"][1]["type"], 11);
}
