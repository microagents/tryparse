use serde::Deserialize;
use tryparse::parse;

#[derive(Deserialize, Debug)]
struct User {
    name: String,
    age: u32,
}

#[test]
#[cfg(feature = "yaml")]
fn test_yaml_debug() {
    let response = "name: Alice\nage: 30";
    println!("Testing with input: {}", response);

    let result: Result<User, _> = parse(response);
    match &result {
        Ok(user) => {
            println!("SUCCESS: {:?}", user);
            assert_eq!(user.name, "Alice");
            assert_eq!(user.age, 30);
        }
        Err(e) => {
            println!("FAILED: {:?}", e);
            panic!("YAML parsing should work!");
        }
    }
}
