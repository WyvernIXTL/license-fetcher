use license_fetcher::prelude::*;

fn main() {
    println!("Integration Test");

    let package_list = read_package_list_from_out_dir!().unwrap();

    assert_eq!(package_list[0].name, "test_buildscript");
    assert_eq!(
        package_list[0].license_texts.clone()[0].1.trim(),
        "THIS IS NOT A LICENSE"
    );

    // println!("{}", package_list);

    println!("All OK!");
}
