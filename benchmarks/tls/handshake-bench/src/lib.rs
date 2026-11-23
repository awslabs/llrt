pub fn provider_name() -> &'static str {
    #[cfg(feature = "tls-rust")]
    {
        "tls-rust"
    }
    #[cfg(feature = "tls-ring")]
    {
        "tls-ring"
    }
    #[cfg(feature = "tls-aws-lc")]
    {
        "tls-aws-lc"
    }
    #[cfg(feature = "tls-graviola")]
    {
        "tls-graviola"
    }
}
