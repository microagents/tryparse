use serde::Deserialize;
use tryparse::parse;

#[derive(Deserialize, Debug, PartialEq)]
struct User {
    name: String,
    age: u32,
}

#[test]
fn test_architectural_flaw_is_now_fixed() {
    // This WAS the KILLER scenario that demonstrated the architectural flaw:
    // - JSON is buried in prose (needs heuristic extraction)
    // - AND has syntax errors (needs fixing)
    //
    // OLD architecture COULDN'T handle this because:
    // 1. HeuristicStrategy extracted but didn't fix
    // 2. JsonFixerStrategy fixed but didn't extract
    // 3. RESULT: Both strategies failed
    //
    // NEW architecture (multi-stage) CAN handle this:
    // 1. Extract candidates (heuristic finds {name: 'Alice', age: 30})
    // 2. Try parsing each candidate
    // 3. If parsing fails, apply fixes
    // 4. SUCCESS!

    let response = r#"
    Sure! Here's the user data: {name: 'Alice', age: 30}
    Hope that helps!
    "#;

    let result: Result<User, _> = parse(response);

    match &result {
        Ok(user) => {
            println!("SUCCESS: {:?}", user);
            assert_eq!(user.name, "Alice");
            assert_eq!(user.age, 30);
        }
        Err(e) => {
            println!("FAILED: {:?}", e);
            panic!("Architectural flaw still exists!");
        }
    }

    // This should NOW work with the multi-stage architecture
    assert!(
        result.is_ok(),
        "Multi-stage architecture should handle extraction + fixing"
    );
}
