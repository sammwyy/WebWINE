use std::fs;
use std::io::Write;

fn main() {
    let mut file = fs::File::create("hello.txt").expect("create hello.txt");
    file.write_all(b"world").expect("write hello.txt");
    println!("created hello.txt");

    fs::create_dir_all("foo").expect("create foo/");
    println!("created foo/");

    fs::File::create("foo/bar.txt").expect("create foo/bar.txt");
    println!("created foo/bar.txt");
}
