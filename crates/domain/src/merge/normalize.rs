//! 文本归一化：全角转半角、ASCII 小写、去标点空白、24 对繁简映射。

const TRAD_TO_SIMP: [(&str, &str); 24] = [
    ("著", "着"),
    ("祗", "只"),
    ("週", "周"),
    ("線", "线"),
    ("點", "点"),
    ("認", "认"),
    ("討", "讨"),
    ("論", "论"),
    ("會", "会"),
    ("議", "议"),
    ("時", "时"),
    ("間", "间"),
    ("開", "开"),
    ("關", "关"),
    ("實", "实"),
    ("現", "现"),
    ("監", "监"),
    ("測", "测"),
    ("準", "准"),
    ("備", "备"),
    ("確", "确"),
    ("發", "发"),
    ("計", "计"),
    ("風", "风"),
];

fn halfwidth(character: char) -> char {
    let code = character as u32;
    if (0xFF01..=0xFF5E).contains(&code) {
        char::from_u32(code - 0xFEE0).unwrap_or(character)
    } else if code == 0x3000 {
        ' '
    } else {
        character
    }
}

fn simplified(character: char) -> char {
    for (traditional, simplified) in TRAD_TO_SIMP {
        if traditional.starts_with(character) {
            return simplified.chars().next().unwrap_or(character);
        }
    }
    character
}

/// 纯函数：`&str -> String`，不 panic、不做 I/O。
pub fn normalize_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for raw in input.chars() {
        let folded = simplified(halfwidth(raw));
        if folded.is_alphanumeric() {
            out.push(folded.to_ascii_lowercase());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fullwidth_and_punct_and_case_collapse() {
        assert_eq!(normalize_text(" ＡＢＣ，１２３。 "), "abc123");
        assert_eq!(normalize_text("Hello, World!"), "helloworld");
    }

    #[test]
    fn traditional_maps_to_simplified() {
        assert_eq!(normalize_text("確認"), "确认");
        assert_eq!(normalize_text("下週"), "下周");
    }

    #[test]
    fn normalization_makes_variants_equal() {
        let plain = normalize_text("确认下周灰度上线。");
        let no_punct = normalize_text("确认下周灰度上线");
        let traditional = normalize_text("確認下週灰度上線");
        assert_eq!(plain, no_punct);
        assert_eq!(plain, traditional);
        assert_eq!(normalize_text("，。！？"), "");
    }
}
