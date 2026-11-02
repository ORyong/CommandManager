//! 命令列表的检索：大小写不敏感，按词切分，任何顺序都要同时命中。
//!
//! 不引第三方匹配库：命中强度只有五个档位，同一档里再按匹配的紧凑程度微调，
//! 足够了，也省掉一层格式化成本。

/// 字段越靠前权重越高（名称 > 标签 > 备注 > 正文）。
const FIELD_WEIGHT: u32 = 5_000;
/// 档位之间的落差，必须大于同档微调的上限。
const TIER_WEIGHT: u32 = 1_000;
/// 同档微调的最大值，不与下一档重叠。
const BONUS_CAP: u32 = 999;
/// 超长查询直接判为无结果，避免病态输入拖着匹配跑。
const MAX_QUERY_BYTES: usize = 2048;

/// 一次查询一个实例，可反复对多条记录打分。
pub struct CommandQuery {
    words: Vec<String>,
    usable: bool,
}

impl CommandQuery {
    pub fn new(query: &str) -> Self {
        let usable = query.len() <= MAX_QUERY_BYTES;
        let words = if usable {
            query.split_whitespace().map(str::to_lowercase).collect()
        } else {
            Vec::new()
        };
        Self { words, usable }
    }

    /// 单字段打分，等价于只有一个字段的 `score_fields`。
    #[cfg(test)]
    pub fn score(&mut self, text: &str) -> Option<u32> {
        self.score_fields(&[text])
    }

    /// 逐个字段打分取最高档；字段顺序即优先级。
    pub fn score_fields(&mut self, fields: &[&str]) -> Option<u32> {
        if !self.usable {
            return None;
        }
        if self.words.is_empty() {
            return Some(0);
        }
        let folded: Vec<String> = fields.iter().map(|field| field.to_lowercase()).collect();

        let mut total = 0;
        for word in &self.words {
            let mut best = None;
            for (index, text) in folded.iter().enumerate() {
                let Some(found) = locate(text, word) else {
                    continue;
                };
                let position = (fields.len() - index - 1) as u32 * FIELD_WEIGHT;
                let value = position + found.tier as u32 * TIER_WEIGHT + found.bonus;
                best = Some(best.map_or(value, |previous: u32| previous.max(value)));
            }
            total += best?;
        }
        Some(total)
    }
}

#[derive(Clone, Copy)]
struct Found {
    tier: Tier,
    bonus: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Tier {
    /// 散落在字段里的字，依次出现即算命中。
    Scattered = 0,
    /// 字段里出现过，但不在词的边界上。
    Inside = 1,
    /// 命中了某个词的开头。
    WordStart = 2,
    /// 字段以查询词开头。
    Prefix = 3,
    /// 整字段就是查询词。
    Exact = 4,
}

/// 在已转小写的字段里定位一个查询词。
fn locate(field: &str, word: &str) -> Option<Found> {
    let tier = if field == word {
        Tier::Exact
    } else if field.starts_with(word) {
        Tier::Prefix
    } else if field
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '-')
        .any(|part| part.starts_with(word))
    {
        Tier::WordStart
    } else if field.contains(word) {
        Tier::Inside
    } else if is_subsequence(field, word) {
        Tier::Scattered
    } else {
        return None;
    };

    Some(Found {
        tier,
        bonus: tightness(field, word, tier),
    })
}

/// 同档内的微调：越早出现、越不分散，分越高。
fn tightness(field: &str, word: &str, tier: Tier) -> u32 {
    match tier {
        Tier::Scattered => {
            let chars: Vec<char> = field.chars().collect();
            let mut cursor = 0;
            let mut gaps = 0usize;
            for target in word.chars() {
                while cursor < chars.len() && chars[cursor] != target {
                    cursor += 1;
                    gaps += 1;
                }
                cursor += 1;
            }
            BONUS_CAP.saturating_sub((gaps as u32).saturating_mul(40).min(BONUS_CAP))
        }
        _ => {
            // 命中位置靠前、字段本身短，都算更贴近。
            let lead = field.find(word).unwrap_or(0).min(BONUS_CAP as usize) as u32;
            let length = field.chars().count().min(BONUS_CAP as usize) as u32;
            BONUS_CAP
                .saturating_sub(lead.saturating_mul(4))
                .saturating_sub(length.saturating_mul(2))
        }
    }
}

fn is_subsequence(field: &str, word: &str) -> bool {
    let mut rest = field.chars();
    word.chars().all(|target| rest.any(|ch| ch == target))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_spaces_and_abbreviations_are_consistent() {
        for query in ["DOCKER COMPOSE", "compose docker", "dck cmp"] {
            assert!(
                CommandQuery::new(query)
                    .score("Docker Compose up")
                    .is_some()
            );
        }
        assert!(
            CommandQuery::new("环境 CREATE")
                .score_fields(&["创建环境", "conda create"])
                .is_some()
        );
        assert!(
            CommandQuery::new("docker missing")
                .score("docker compose")
                .is_none()
        );
        assert!(CommandQuery::new("-xyz").score("docker compose").is_none());
        assert!(
            CommandQuery::new(&"a".repeat(2049))
                .score("anything")
                .is_none()
        );
        assert_eq!(CommandQuery::new("   ").score("anything"), Some(0));
    }

    #[test]
    fn exact_prefix_word_substring_and_fuzzy_have_stable_priority() {
        let mut query = CommandQuery::new("git");
        let scores =
            ["git", "gitk", "run git", "legit", "g-in-t"].map(|text| query.score(text).unwrap());
        assert!(scores.windows(2).all(|pair| pair[0] > pair[1]));
        assert!(
            query.score_fields(&["Git status", "echo hello"]).unwrap()
                > query
                    .score_fields(&["Inspect repository", "git status"])
                    .unwrap()
        );
    }

    #[test]
    fn every_word_must_land_somewhere() {
        let mut query = CommandQuery::new("docker up");
        assert!(query.score("docker compose up -d").is_some());
        assert!(query.score_fields(&["docker", "up -d"]).is_some());
        assert!(query.score("docker compose down").is_none());
    }
}
