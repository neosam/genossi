use inventurly_rest_types::PersonTO;
use serde_json;
use uuid::Uuid;

fn main() {
    // Test 1: Deserializing JSON without datetime fields (create scenario)
    let json_without_dates = r#"{"name": "Test Person", "age": 30}"#;
    let person_result = serde_json::from_str::<PersonTO>(json_without_dates);
    match person_result {
        Ok(person) => {
            println!("✅ Deserialization without dates successful:");
            println!("   Name: {}, Age: {}", person.name, person.age);
            println!("   Created: {:?}, Deleted: {:?}", person.created, person.deleted);
        }
        Err(e) => {
            println!("❌ Deserialization failed: {}", e);
            return;
        }
    }

    // Test 2: Serializing person with dates (response scenario)
    let now = time::OffsetDateTime::now_utc();
    let datetime = time::PrimitiveDateTime::new(now.date(), now.time());
    
    let person_with_dates = PersonTO {
        id: Some(Uuid::new_v4()),
        name: "Test Person".to_string(),
        age: 30,
        created: Some(datetime),
        deleted: None,
        version: Some(Uuid::new_v4()),
    };

    match serde_json::to_string_pretty(&person_with_dates) {
        Ok(json) => {
            println!("\n✅ Serialization with dates successful:");
            println!("{}", json);
        }
        Err(e) => {
            println!("❌ Serialization failed: {}", e);
        }
    }
}