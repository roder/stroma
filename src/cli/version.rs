/// Display version information
pub fn execute() {
    println!("stroma {}", env!("CARGO_PKG_VERSION"));
    println!("Operator CLI for Stroma trust network bot");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_execute() {
        // Version command should not panic
        execute();
    }
}
