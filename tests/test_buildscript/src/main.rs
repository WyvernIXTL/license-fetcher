use license_fetcher::read_package_list_from_out_dir;

fn main() {
    println!("Integration Test");

    let package_list = read_package_list_from_out_dir!().unwrap();

    assert_eq!(package_list[0].name, "test_buildscript");
    assert_eq!(
        package_list[0].license_text.clone().unwrap().trim(),
        "THIS IS NOT A LICENSE"
    );

    // println!("{}", package_list);

    println!("All OK!");
}
