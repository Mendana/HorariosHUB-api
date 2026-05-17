// src/jwt.rs
use crate::errors::AppError;
use crate::modules::auth::models::UserRole;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // user_id
    pub email: String,
    pub role: UserRole,
    pub exp: usize,
    pub iat: usize,
}

pub fn generate_token(
    user_id: &str,
    email: &str,
    role: &UserRole,
    secret: &str,
    ttl_seconds: u64,
) -> Result<String, AppError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        role: role.clone(),
        exp: now + ttl_seconds as usize,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(e.into()))
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::Unauthorized,
        _ => AppError::Unauthorized,
    })
}

#[cfg(test)]
mod jwt_unit_tests {
    use crate::{jwt, modules::auth::models::UserRole};
    use uuid::Uuid;

    #[test]
    fn test_generate_token_creates_valid_jwt() {
        let user_id = Uuid::new_v4().to_string();
        let email = "test@uniovi.es";
        let role = UserRole::Student;
        let secret = "test_secret_key";
        let ttl = 3600u64;

        let token = jwt::generate_token(&user_id, email, &role, secret, ttl)
            .expect("No se pudo generar el token");

        // El token no debe estar vacío
        assert!(!token.is_empty());
        // El token debe tener el formato JWT (3 partes separadas por puntos)
        assert_eq!(token.split('.').count(), 3);
    }

    #[test]
    fn test_verify_token_extracts_claims() {
        let user_id = Uuid::new_v4().to_string();
        let email = "test@uniovi.es";
        let role = UserRole::Student;
        let secret = "test_secret_key";
        let ttl = 3600u64;

        let token = jwt::generate_token(&user_id, email, &role, secret, ttl)
            .expect("No se pudo generar el token");

        let claims = jwt::verify_token(&token, secret).expect("No se pudo verificar el token");

        // Verificar que los claims se extrajeron correctamente
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, email);
        assert_eq!(claims.role, role);
    }

    #[test]
    fn test_verify_token_fails_with_wrong_secret() {
        let user_id = Uuid::new_v4().to_string();
        let email = "test@uniovi.es";
        let role = UserRole::Student;
        let secret = "test_secret_key";
        let ttl = 3600u64;

        let token = jwt::generate_token(&user_id, email, &role, secret, ttl)
            .expect("No se pudo generar el token");

        // Intentar verificar con un secret diferente
        let result = jwt::verify_token(&token, "different_secret");

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_fails_with_invalid_token() {
        let secret = "test_secret_key";
        let invalid_token = "not.a.valid.jwt.token";

        let result = jwt::verify_token(invalid_token, secret);

        assert!(result.is_err());
    }

    #[test]
    fn test_claims_include_all_required_fields() {
        let user_id = Uuid::new_v4().to_string();
        let email = "test@uniovi.es";
        let role = UserRole::Student;
        let secret = "test_secret_key";
        let ttl = 3600u64;

        let token = jwt::generate_token(&user_id, email, &role, secret, ttl)
            .expect("No se pudo generar el token");

        let claims = jwt::verify_token(&token, secret).expect("No se pudo verificar el token");

        // Verificar que todos los campos requeridos estén presentes
        assert!(!claims.sub.is_empty());
        assert!(!claims.email.is_empty());
        assert!(claims.exp > 0);
        assert!(claims.iat > 0);
        assert!(claims.exp > claims.iat); // exp debe ser mayor que iat
    }

    #[test]
    fn test_token_expiration_fields_are_valid() {
        let user_id = Uuid::new_v4().to_string();
        let email = "test@uniovi.es";
        let role = UserRole::Student;
        let secret = "test_secret_key";
        let ttl = 3600u64;

        let token = jwt::generate_token(&user_id, email, &role, secret, ttl)
            .expect("No se pudo generar el token");

        let claims = jwt::verify_token(&token, secret).expect("No se pudo verificar el token");

        // El token debe expirar en aproximadamente ttl segundos
        let expiration_diff = (claims.exp - claims.iat) as u64;
        assert!(expiration_diff >= ttl - 10 && expiration_diff <= ttl + 10);
    }
}
