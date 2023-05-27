use std::process::{Command, Output};

pub fn execute(arguments: Vec<&str>) -> Result<Output, std::io::Error> {
    Command::new("pihole").args(arguments).output()
}
