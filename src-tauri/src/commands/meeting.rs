// Commands are defined in lib.rs for proper Tauri macro expansion
// This module is kept for organization but not used

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_meeting() {
        let meeting = create_meeting("Test".to_string()).expect("Failed to create");
        assert_eq!(meeting.title, "Test");
        assert!(!meeting.id.is_empty());
    }
}
