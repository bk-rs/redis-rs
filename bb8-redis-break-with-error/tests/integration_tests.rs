#![cfg(feature = "_integration_tests")]

#[path = "integration_tests"]
mod integration_tests {
    mod helpers;

    #[cfg(test)]
    mod extension_error_with_noauth;

    #[cfg(test)]
    mod extension_error_with_wpassword;
}
