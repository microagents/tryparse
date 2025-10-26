use tryparse::parser::strategies::{JsonFixerStrategy, ParsingStrategy};

fn main() {
    let input = r#"{
  key: value with space,
  array: [1, 2, 3],
  object: {
    key: value
  }
}"#;

    println!("Input:\n{}\n", input);

    let fixer = JsonFixerStrategy::default();
    match fixer.parse(input) {
        Ok(candidates) => {
            println!("Found {} candidates", candidates.len());
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
