pub fn is_on_apify() -> bool {
    match std::env::var("APIFY_IS_AT_HOME") {
        Ok(ref x) if x == "1"  => true,
        _ => false
    }
}