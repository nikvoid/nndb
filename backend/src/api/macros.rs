
/// Log error and return 500 status to client
#[macro_export]
macro_rules! log_n_bail {
    ($lit:literal $(, $($tt:tt)* )?) => {{
        tracing::error!($($($tt)*,)? $lit);
        return Err(actix_web::error::ErrorInternalServerError($lit));
    }};
}
/// Log info and return 200 status to client
#[macro_export]
macro_rules! log_n_ok {
    ($lit:literal $(, $($tt:tt)* )?) => {{
        tracing::info!($($($tt)*,)? $lit);
        return Ok($lit);
    }};
}
