pub fn validate_multiple_of_30(duration: i32) -> Result<(), validator::ValidationError> {
    if duration % 30 != 0 {
        return Err(validator::ValidationError::new(
            "duration_minutes must be a multiple of 30",
        ));
    }
    Ok(())
}
