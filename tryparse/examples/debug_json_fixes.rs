use tryparse::parser::FlexibleParser;

fn main() {
    // Test 1: Unquoted keys
    let input1 = r#"
```json
{
  key: "value",
  array: [1, 2, 3],
  object: {
    key: "value"
  }
}
```
"#;
    println!("=== Test 1: Unquoted Keys ===");
    println!("Input: {}", input1);

    let parser = FlexibleParser::new();
    match parser.parse(input1) {
        Ok(candidates) => {
            println!("Parser found {} candidates", candidates.len());
            for (i, cand) in candidates.iter().enumerate() {
                println!("Candidate {}: source={:?}", i, cand.source);
                println!(
                    "  Value: {}",
                    serde_json::to_string_pretty(&cand.value).unwrap()
                );
            }
        }
        Err(e) => println!("Error: {:?}", e),
    }

    // Test 2: Unquoted values with spaces
    let input2 = r#"{
  key: value with space,
  array: [1, 2, 3],
  object: {
    key: value
  }
}"#;
    println!("\n=== Test 2: Unquoted Values with Spaces ===");
    println!("Input: {}", input2);

    match parser.parse(input2) {
        Ok(candidates) => {
            println!("Parser found {} candidates", candidates.len());
            for (i, cand) in candidates.iter().enumerate() {
                println!("Candidate {}: source={:?}", i, cand.source);
                println!(
                    "  Value: {}",
                    serde_json::to_string_pretty(&cand.value).unwrap()
                );
            }
        }
        Err(e) => println!("Error: {:?}", e),
    }

    // Test 3: Triple-quoted strings
    let input3 = r#"{
  "heading": "Urban Oasis",
  "python_function_code": """def is_urban_oasis(property):
   return 'Large Green Area' in property['amenities']"""
}"#;
    println!("\n=== Test 3: Triple-Quoted Strings ===");
    println!("Input: {}", input3);

    match parser.parse(input3) {
        Ok(candidates) => {
            println!("Parser found {} candidates", candidates.len());
            for (i, cand) in candidates.iter().enumerate() {
                println!("Candidate {}: source={:?}", i, cand.source);
                println!(
                    "  Value: {}",
                    serde_json::to_string_pretty(&cand.value).unwrap()
                );
            }
        }
        Err(e) => println!("Error: {:?}", e),
    }
}
