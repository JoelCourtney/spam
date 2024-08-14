use static_files::resource_dir;

fn main() -> std::io::Result<()> {
    println!("cargo::rerun-if-changed=./web/build");
    resource_dir("./web/build").build()
}
