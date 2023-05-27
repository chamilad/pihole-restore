use std::process::{Command, Output};

/// Execute a pihole CLI command
pub fn execute(arguments: Vec<&str>) -> Result<Output, std::io::Error> {
    Command::new("pihole").args(arguments).output()
}

/// restart DNS service
pub fn restart_dns() -> Result<Output, std::io::Error> {
    let restart_cmd: Vec<&str> = vec!["restartdns"];
    execute(restart_cmd)
}
