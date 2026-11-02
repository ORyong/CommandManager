use crate::{command_search::CommandQuery, model::Command};

pub fn filter(commands: &[Command], query: &str, tags: &[String], any: bool) -> Vec<Command> {
    if query.len() > 2048 {
        return vec![];
    }
    let mut matcher = CommandQuery::new(query);
    let mut results = Vec::new();
    for (index, command) in commands.iter().enumerate() {
        let has = |tag: &String| {
            command
                .tags
                .iter()
                .any(|t| t.to_lowercase() == tag.to_lowercase())
        };
        if !tags.is_empty()
            && !(if any {
                tags.iter().any(has)
            } else {
                tags.iter().all(has)
            })
        {
            continue;
        }
        let tag_text = command.tags.join(" ");
        let fields = [
            &*command.name,
            &*tag_text,
            &*command.notes,
            &*command.command,
        ];
        let rank = matcher
            .score_fields(&fields)
            .or_else(|| approximate(query, &fields));
        if let Some(rank) = rank {
            results.push((rank, index, command.clone()));
        }
    }
    if !query.trim().is_empty() {
        results.sort_by_key(|(rank, index, _)| (std::cmp::Reverse(*rank), *index));
    }
    results.into_iter().map(|(_, _, c)| c).collect()
}

fn approximate(query: &str, fields: &[&str]) -> Option<u32> {
    let text = fields.join(" ").to_lowercase();
    let words: Vec<_> = text
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|s| !s.is_empty())
        .collect();
    for part in query.to_lowercase().split_whitespace() {
        if CommandQuery::new(part).score_fields(fields).is_some() {
            continue;
        }
        if text.contains(part) {
            continue;
        }
        let size = part.chars().count();
        if size < 3 || size > 40 {
            return None;
        }
        let limit = if size >= 6 { 2 } else { 1 };
        if !words
            .iter()
            .any(|word| distance(part, word, limit) <= limit)
        {
            return None;
        }
    }
    Some(1)
}

fn distance(a: &str, b: &str, limit: usize) -> usize {
    let a: Vec<_> = a.chars().collect();
    let b: Vec<_> = b.chars().collect();
    if a.len().abs_diff(b.len()) > limit {
        return limit + 1;
    }
    let mut row: Vec<_> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut next = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            next[j + 1] = (next[j] + 1)
                .min(row[j + 1] + 1)
                .min(row[j] + usize::from(ca != cb));
        }
        if *next.iter().min().unwrap() > limit {
            return limit + 1;
        }
        row = next;
    }
    row[b.len()]
}
