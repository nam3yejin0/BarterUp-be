pub mod auth_dtos;
pub mod personal_dtos;
pub mod profile_picture_dtos;
pub mod post_dtos;
// alias supaya dapat dipanggil sebagai `crate::dtos::auth` dan `crate::dtos::personal`
pub use auth_dtos as auth;
pub use personal_dtos as personal;