//! # sec
//!
//! 安全与加密有关操作
//!

use hex;
use sha2::{Digest, Sha256};

/// 注意：只能在新建对象的时候计算此 id，因为使用的字段未来可能可变
pub fn hash_object_id(university: &String, department: &String, supervisor: &String) -> String {
    let s = format!("{}{}{}", university, department, supervisor);
    hex::encode(&Sha256::digest(s.as_bytes())[..8])
}

/// 评价 id = sha256( object | 评价 | 日期 )[:16] 注意，这个也包含去重的性质
pub fn hash_comment_id(object_id: &String, comment: &String, date: &String) -> String {
    let s = format!("{}{}{}", object_id, comment, date);
    hex::encode(&Sha256::digest(s.as_bytes())[..8])
}

pub fn hash_author_sign(comment_id: &String, otp: &String) -> String {
    const SAFC_ASLT: &str = "SAFC_salt";
    let a = hex::encode(Sha256::digest(format!("{}{}", SAFC_ASLT, otp).as_bytes()));
    hex::encode(Sha256::digest(format!("{}{}", comment_id, a).as_bytes()))
}

#[test]
fn test_calc_object_id() {
    assert_eq!(
        "bf3d2da3a9bfc528".to_string(),
        hash_object_id(
            &"university".to_string(),
            &"department".to_string(),
            &"supervisor".to_string()
        )
    )
}

#[test]
fn test_hash_author_sign() {
    assert_eq!(
        "633d8c27f20896ab27a9c762d4e1e9da16b54edec78de13f3c950820aca70b7c".to_string(),
        hash_author_sign(&"cba0415143b305c0".to_string(), &"201809".to_string())
    )
}
