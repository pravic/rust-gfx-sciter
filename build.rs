fn main() {
	let path = "d:/usr/lib/gui/htmlayout/sciter-sdk/lib";
	println!("cargo:rustc-link-search=native={}", path);
}
