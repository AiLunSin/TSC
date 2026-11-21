//! 哈希内核：FNV-1a 64、SimHash 64 与汉明距离。全部确定性、无随机源。

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a 64 位。空输入返回 offset basis。
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn is_cjk(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF
    )
}

fn features(text: &str) -> Vec<u64> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if is_cjk(chars[index]) {
            if index + 1 < chars.len() && is_cjk(chars[index + 1]) {
                let mut gram = String::new();
                gram.push(chars[index]);
                gram.push(chars[index + 1]);
                out.push(fnv1a64(gram.as_bytes()));
            }
            index += 1;
        } else if chars[index].is_ascii_alphanumeric() {
            let start = index;
            while index < chars.len() && chars[index].is_ascii_alphanumeric() {
                index += 1;
            }
            let word: String = chars[start..index].iter().collect();
            out.push(fnv1a64(word.as_bytes()));
        } else {
            index += 1;
        }
    }
    out
}

/// CJK 取相邻字符 2-gram、ASCII 取连续字母数字词 unigram；空输入返回 0。
pub fn simhash64(text: &str) -> u64 {
    let hashes = features(text);
    if hashes.is_empty() {
        return 0;
    }
    let mut votes = [0i64; 64];
    for hash in &hashes {
        for (bit, slot) in votes.iter_mut().enumerate() {
            *slot += if (hash >> bit) & 1 == 1 { 1 } else { -1 };
        }
    }
    let mut out = 0u64;
    for (bit, vote) in votes.iter().enumerate() {
        if *vote > 0 {
            out |= 1u64 << bit;
        }
    }
    out
}

/// 汉明距离，值域 `0..=64`。
pub fn hamming_distance(left: u64, right: u64) -> u8 {
    u8::try_from((left ^ right).count_ones()).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a64_matches_reference_vectors() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn simhash_is_stable_and_handles_empty() {
        assert_eq!(simhash64(""), 0);
        let first = simhash64("确认下周灰度上线");
        assert_eq!(first, simhash64("确认下周灰度上线"));
        assert_ne!(first, 0);
    }

    #[test]
    fn hamming_distance_bounds() {
        assert_eq!(hamming_distance(0, 0), 0);
        assert_eq!(hamming_distance(u64::MAX, u64::MAX), 0);
        assert_eq!(hamming_distance(0, 1), 1);
        assert_eq!(hamming_distance(0, u64::MAX), 64);
    }
}
