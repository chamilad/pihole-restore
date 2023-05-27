use std::process::{Command, Output};

pub fn execute(arguments: Vec<&str>) -> Result<Output, std::io::Error> {
    Command::new("pihole").args(arguments).output()
}

pub fn restart_dns() -> Result<Output, std::io::Error> {
    let restart_cmd: Vec<&str> = vec!["restartdns"];
    execute(restart_cmd)
}
