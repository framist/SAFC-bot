//! # sec
//!
//! 安全与加密有关操作。
//!
//! 本模块集中维护与哈希、签名相关的工具函数，以保证“对象”“评论”
//! 等核心实体在数据库中的标识具备稳定性与去重特性。若未来需要调整
//! 哈希算法或截取策略，应在此模块内统一修改。

use hex;
use sha2::{Digest, Sha256};

/// 根据学校、二级单位与导师名称生成对象标识。
///
/// 取输入文本拼接后计算 SHA-256，并截取前 8 字节（16 个十六进制字符）
/// 作为对象 id。仅在创建新对象时调用；后续如字段发生改名，应采用迁移
/// 逻辑而不是重新计算，以避免破坏引用关系。
///
/// ```
/// # use safc::sec::hash_object_id;
/// let object_id = hash_object_id(
///     &"Some University".to_string(),
///     &"Department".to_string(),
///     &"Prof. Zhang".to_string()
/// );
/// assert_eq!(object_id.len(), 16);
/// ```
pub fn hash_object_id(university: &String, department: &String, supervisor: &String) -> String {
    let s = format!("{}{}{}", university, department, supervisor);
    hex::encode(&Sha256::digest(s.as_bytes())[..8])
}

/// 生成评论唯一 id，用于评论去重与追踪。
///
/// 评论 id 由对象 id、评论正文与日期组成的字符串取 SHA-256，再截取
/// 前 8 字节（16 个字符）。采用日期参与哈希可区别相同文本在不同日期
/// 的多次提交。
pub fn hash_comment_id(object_id: &String, comment: &String, date: &String) -> String {
    let s = format!("{}{}{}", object_id, comment, date);
    hex::encode(&Sha256::digest(s.as_bytes())[..8])
}

/// 根据评论 id 与一次性口令生成作者签名。
///
/// 内部流程：`sign = sha256( comment_id | sha256("SAFC_salt" + otp) )`。签名
/// 可用于后续证明“这是我提交的评论”，同时不会泄漏真实的 OTP。
/// TODO: 当 `otp` 为空时应视使用随机数生成 otp。
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
    // 这个函数可以临时作为验证 otp 使用
    assert_eq!(
        "633d8c27f20896ab27a9c762d4e1e9da16b54edec78de13f3c950820aca70b7c".to_string(),
        hash_author_sign(&"cba0415143b305c0".to_string(), &"201809".to_string())
    )
}
