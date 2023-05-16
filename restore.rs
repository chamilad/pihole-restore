use std::env;
// use std::process;

fn main() {
    // 1. read archive file
    // for arg in std::env::args() {
    // println!("'{}'", arg);
    // }

    // let args: Vec<String> = std::env::args().skip(1).collect();
    // if args.len() == 0 {
    //     println!("err: backup archive file should be specified");
    //     process::exit(1);
    // }

    let _archive_file = env::args()
        .nth(1)
        .expect("err: backup archive file should be specified");

    // 2. for each file
    // 2.1 check type and insert to table
}
