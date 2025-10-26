//! Test what candidates the parser produces for raw primitive inputs

use tryparse::parser::FlexibleParser;

#[test]
fn debug_raw_number_with_commas() {
    let parser = FlexibleParser::new();
    let input = "12,111";

    let candidates = parser.parse(input);
    println!("Input: '{}'", input);
    println!("Candidates: {:#?}", candidates);

    assert!(candidates.is_ok(), "Parser should produce candidates");
    let values = candidates.unwrap();
    println!("Number of candidates: {}", values.len());
    for (i, val) in values.iter().enumerate() {
        println!("Candidate {}: {:?}", i, val.value);
    }
}

#[test]
fn debug_raw_boolean() {
    let parser = FlexibleParser::new();
    let input = "true";

    let candidates = parser.parse(input);
    println!("Input: '{}'", input);
    println!("Candidates: {:#?}", candidates);
}

#[test]
fn debug_boolean_in_text() {
    let parser = FlexibleParser::new();
    let input = "The answer is true";

    let candidates = parser.parse(input);
    println!("Input: '{}'", input);
    println!("Candidates: {:#?}", candidates);
}

#[test]
fn debug_triple_quoted() {
    let parser = FlexibleParser::new();
    let input = r#"
{
  "headings": [
    {
      "heading": "Urban Oasis",
      "python_function_code": """def is_urban_oasis(property):
       return 'Large Green Area' in property['amenities'] or 'Garden' in property['amenities']""",
      "description": "Properties that offer a serene living experience."
    }
  ]
}
"#;

    let candidates = parser.parse(input);
    println!("Candidates: {:#?}", candidates);
}
