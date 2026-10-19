fn main() -> flowlink_engine::Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"displays":flowlink_engine::platform::displays("local")?,"addresses":flowlink_engine::discovery::addresses(),"permissions":flowlink_engine::platform::permissions()})
        )?
    );
    Ok(())
}
