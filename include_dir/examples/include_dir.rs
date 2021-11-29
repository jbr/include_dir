fn main() {
    let examples = trillium_include_dir::include_dir!("examples");
    let nonexistent = trillium_include_dir::try_include_dir!("nope");
    let src = trillium_include_dir::try_include_dir!("src");

    dbg!(&examples, nonexistent.unwrap_err(), src.unwrap());
}
