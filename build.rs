fn main() {
    println!("cargo:rustc-link-arg=/ENTRY:main");
    println!("cargo:rustc-link-arg=/ALIGN:8");
    println!("cargo:rustc-link-arg=/NODEFAULTLIB");
    println!("cargo:rustc-link-lib=ucrt");
    println!("cargo:rustc-link-lib=vcruntime");
}