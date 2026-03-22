use uuid::Uuid;

/// Generate a new unique panel/workspace ID.
#[allow(dead_code)]
pub fn new_id() -> Uuid {
    Uuid::new_v4()
}
